use capitainerie_common::{CoreError, IntegrationActionId, OrganizationId};
use capitainerie_macros::transactional;
use serde_json::Value;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::{AccessRequest, Action, ResourceKind, Subject, require},
        integration::{
            InstanceConnection, IntegrationInstance,
            action::{Access, ActionCall},
            journal::{ActionContext, Grant, IntegrationAction, Outcome},
            ports::{
                IntegrationActionRepository, IntegrationConnector, IntegrationInstanceRepository,
            },
            revert::{
                RevertError, RevertPlan, RevertRefusal, ensure_revertible, plan, plan_against,
            },
        },
        run::{RunStatus, ports::RunRepository},
    },
    infrastructure::integration::{
        actions::PgIntegrationActionRepository, postgres::PgIntegrationInstanceRepository,
    },
};

struct Prepared {
    action: IntegrationAction,
    instance: IntegrationInstance,
    connection: InstanceConnection,
    snapshot: Option<Value>,
    plan: RevertPlan,
}

type Stored<T> = Result<Result<T, RevertRefusal>, CoreError>;

impl CapitainerieUseCase {
    pub async fn revert_integration_action<C: IntegrationConnector>(
        &self,
        subject: Subject,
        action_id: IntegrationActionId,
        confirm_production: bool,
        connector: &C,
    ) -> Result<Vec<IntegrationAction>, RevertError> {
        let organization_id = subject.organization_id;
        let prepared = self
            .prepare_integration_revert(subject.clone(), action_id, confirm_production)
            .await??;
        let restored = self
            .carry_out_integration_revert(subject, prepared, connector)
            .await;
        if restored.is_err()
            && let Err(error) = self
                .release_integration_revert(organization_id, action_id)
                .await
        {
            tracing::warn!(%action_id, %error, "a restoration reservation could not be released, it expires by itself");
        }
        restored
    }

    async fn carry_out_integration_revert<C: IntegrationConnector>(
        &self,
        subject: Subject,
        prepared: Prepared,
        connector: &C,
    ) -> Result<Vec<IntegrationAction>, RevertError> {
        let organization_id = subject.organization_id;
        let Prepared {
            action,
            instance,
            connection,
            snapshot,
            plan,
        } = prepared;
        let current = match &plan.check {
            Some(check) => connector.perform(&connection, check).await?,
            None => Value::Null,
        };
        let plan = plan_against(
            &action,
            snapshot.as_ref(),
            instance.server_version.as_deref(),
            &current,
        )?;
        let context = ActionContext {
            subject,
            run_id: action.run_id,
            card_id: action.card_id,
            client_id: instance.client_id,
            grants: vec![Grant {
                instance_id: instance.id,
                access: Access::Write,
            }],
            reverts_action_id: Some(action.id),
        };
        let mut written: Vec<IntegrationActionId> = Vec::with_capacity(plan.writes.len());
        let mut restored = true;
        for request in plan.writes {
            let call = ActionCall {
                instance: instance.id,
                request,
            };
            let outcome = self
                .perform_integration_action(&context, call, connector)
                .await?;
            written.extend(outcome.action_id);
            if !outcome.succeeded {
                restored = false;
                break;
            }
        }
        Ok(self
            .record_integration_revert(organization_id, &action, &written, restored)
            .await??)
    }

    #[transactional(run, authz)]
    async fn prepare_integration_revert(
        &self,
        subject: Subject,
        action_id: IntegrationActionId,
        confirm_production: bool,
    ) -> Stored<Prepared> {
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(
                subject,
                Action::IntegrationRevert,
                ResourceKind::IntegrationInstance,
                None,
            ),
        )
        .await?;
        let mut actions = PgIntegrationActionRepository::new(&tx, self.secrets.as_deref());
        let action = actions
            .find(organization_id, action_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if action.reverted_by_action_id.is_some() {
            return Ok(Err(RevertRefusal::AlreadyReverted));
        }
        if let Err(refusal) = ensure_revertible(&action) {
            return Ok(Err(refusal));
        }
        if let Some(run_id) = action.run_id {
            let mut run_repository = run_repository;
            let running = run_repository
                .find(organization_id, run_id)
                .await?
                .is_some_and(|overview| {
                    matches!(overview.run.status, RunStatus::Queued | RunStatus::Running)
                });
            if running {
                return Ok(Err(RevertRefusal::InterventionRunning));
            }
        }
        let mut instances = PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        let instance = instances
            .find(organization_id, action.instance_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if instance.is_archived() {
            return Ok(Err(RevertRefusal::InstanceArchived));
        }
        if instance.environment.is_production() && !confirm_production {
            return Ok(Err(RevertRefusal::ProductionWriteNotConfirmed));
        }
        let snapshot = actions.snapshot(organization_id, action.id).await?;
        let plan = match plan(
            &action,
            snapshot.as_ref(),
            instance.server_version.as_deref(),
        ) {
            Ok(plan) => plan,
            Err(refusal) => return Ok(Err(refusal)),
        };
        let connection = instances
            .connection(organization_id, instance.id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if !actions.claim_revert(organization_id, action.id).await? {
            return Ok(Err(RevertRefusal::AlreadyReverted));
        }
        Ok(Ok(Prepared {
            action,
            instance,
            connection,
            snapshot,
            plan,
        }))
    }

    #[transactional()]
    async fn release_integration_revert(
        &self,
        organization_id: OrganizationId,
        action_id: IntegrationActionId,
    ) -> Result<(), CoreError> {
        PgIntegrationActionRepository::new(&tx, self.secrets.as_deref())
            .release_revert(organization_id, action_id)
            .await
    }

    #[transactional()]
    async fn record_integration_revert(
        &self,
        organization_id: OrganizationId,
        original: &IntegrationAction,
        written: &[IntegrationActionId],
        restored: bool,
    ) -> Stored<Vec<IntegrationAction>> {
        let mut actions = PgIntegrationActionRepository::new(&tx, self.secrets.as_deref());
        if restored {
            let by = match written.first() {
                Some(first) => Some(*first),
                None => earlier_restoration(&mut actions, organization_id, original).await?,
            };
            let Some(by) = by else {
                actions.release_revert(organization_id, original.id).await?;
                return Ok(Err(RevertRefusal::ChangedSince));
            };
            actions
                .mark_reverted(organization_id, original.id, by)
                .await?;
        } else {
            actions.release_revert(organization_id, original.id).await?;
        }
        let mut rows = Vec::with_capacity(written.len());
        for id in written {
            rows.push(
                actions
                    .find(organization_id, *id)
                    .await?
                    .ok_or(CoreError::NotFound)?,
            );
        }
        Ok(Ok(rows))
    }
}

async fn earlier_restoration(
    actions: &mut PgIntegrationActionRepository<'_, '_>,
    organization_id: OrganizationId,
    original: &IntegrationAction,
) -> Result<Option<IntegrationActionId>, CoreError> {
    let rows = match (original.run_id, original.card_id) {
        (Some(run_id), _) => actions.list_for_run(organization_id, run_id).await?,
        (None, Some(card_id)) => actions.list_for_card(organization_id, card_id).await?,
        (None, None) => Vec::new(),
    };
    Ok(rows
        .iter()
        .filter(|row| row.reverts_action_id == Some(original.id))
        .min_by_key(|row| (row.outcome != Outcome::Succeeded, row.created_at))
        .map(|row| row.id))
}
