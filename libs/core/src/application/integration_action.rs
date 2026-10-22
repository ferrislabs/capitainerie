use std::time::Instant;

use capitainerie_common::{CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
        card::ports::CardRepository,
        integration::{
            InstanceConnection, IntegrationInstance,
            action::{Access, ActionCall},
            journal::{ActionContext, ActionOutcome, IntegrationAction},
            perform::{Entry, ensure_allowed, fill_from_before, publication_guard, unrecorded},
            ports::{
                IntegrationActionRepository, IntegrationConnector, IntegrationInstanceRepository,
            },
            redact::restore_masked,
            stamp,
        },
        run::ports::RunRepository,
    },
    infrastructure::integration::{
        actions::PgIntegrationActionRepository, postgres::PgIntegrationInstanceRepository,
    },
};

struct Ready {
    instance: IntegrationInstance,
    connection: InstanceConnection,
}

enum Prepared {
    Refused(ActionOutcome),
    Ready(Box<Ready>),
}

fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

impl CapitainerieUseCase {
    pub async fn perform_integration_action<C: IntegrationConnector>(
        &self,
        context: &ActionContext,
        call: ActionCall,
        connector: &C,
    ) -> Result<ActionOutcome, CoreError> {
        let ready = match self.prepare_integration_action(context, &call).await? {
            Prepared::Refused(outcome) => return Ok(outcome),
            Prepared::Ready(ready) => ready,
        };
        let Ready {
            instance,
            connection,
        } = *ready;
        let entry = Entry {
            context,
            instance: &instance,
            request: &call.request,
            secret: connection.secret.expose(),
        };
        match call.request.access() {
            Access::Read => {
                self.perform_integration_read(&entry, &connection, connector)
                    .await
            }
            Access::Write => {
                self.perform_integration_write(&entry, &connection, connector)
                    .await
            }
        }
    }

    #[transactional(run, authz)]
    pub async fn integration_actions_of_run(
        &self,
        subject: Subject,
        run_id: RunId,
    ) -> Result<Vec<IntegrationAction>, CoreError> {
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(subject, Action::RunRead, ResourceKind::Run, Some(run_id.0))
                .in_run(Some(run_id)),
        )
        .await?;
        let mut run_repository = run_repository;
        run_repository
            .find(organization_id, run_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        PgIntegrationActionRepository::new(&tx, self.secrets.as_deref())
            .list_for_run(organization_id, run_id)
            .await
    }

    #[transactional(card, authz)]
    pub async fn integration_actions_of_card(
        &self,
        subject: Subject,
        card_id: CardId,
    ) -> Result<Vec<IntegrationAction>, CoreError> {
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(
                subject,
                Action::CardRead,
                ResourceKind::Card,
                Some(card_id.0),
            ),
        )
        .await?;
        let mut card_repository = card_repository;
        card_repository
            .get(organization_id, card_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        PgIntegrationActionRepository::new(&tx, self.secrets.as_deref())
            .list_for_card(organization_id, card_id)
            .await
    }

    #[transactional()]
    pub async fn count_integration_writes(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<i64, CoreError> {
        PgIntegrationActionRepository::new(&tx, self.secrets.as_deref())
            .count_writes_not_denied(organization_id, run_id)
            .await
    }

    #[transactional(authz)]
    async fn prepare_integration_action(
        &self,
        context: &ActionContext,
        call: &ActionCall,
    ) -> Result<Prepared, CoreError> {
        let organization_id = context.subject.organization_id;
        let decision = authz
            .evaluate(
                &AccessRequest::new(
                    context.subject.clone(),
                    Action::IntegrationCall {
                        access: call.request.access(),
                    },
                    ResourceKind::IntegrationInstance,
                    Some(call.instance.0),
                )
                .in_run(context.run_id),
            )
            .await?;
        let mut instances = PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        let instance = instances.find(organization_id, call.instance).await?;
        if let Err(denial) = ensure_allowed(&decision, context, instance.as_ref(), call) {
            let Some(instance) = instance else {
                return Ok(Prepared::Refused(unrecorded(&denial, &call.request)));
            };
            let entry = Entry {
                context,
                instance: &instance,
                request: &call.request,
                secret: "",
            };
            let row = entry.denied(&denial, stamp());
            PgIntegrationActionRepository::new(&tx, self.secrets.as_deref())
                .insert(&row, None)
                .await?;
            return Ok(Prepared::Refused(entry.conclude(&row, None, false)));
        }
        let instance = instance.ok_or(CoreError::NotFound)?;
        let connection = instances
            .connection(organization_id, call.instance)
            .await?
            .ok_or(CoreError::NotFound)?;
        Ok(Prepared::Ready(Box::new(Ready {
            instance,
            connection,
        })))
    }

    #[transactional()]
    async fn record_integration_action(
        &self,
        action: &IntegrationAction,
        snapshot: Option<&serde_json::Value>,
    ) -> Result<(), CoreError> {
        PgIntegrationActionRepository::new(&tx, self.secrets.as_deref())
            .insert(action, snapshot)
            .await
    }

    #[transactional()]
    async fn complete_integration_action(
        &self,
        action: &IntegrationAction,
    ) -> Result<(), CoreError> {
        PgIntegrationActionRepository::new(&tx, self.secrets.as_deref())
            .complete(action)
            .await
    }

    async fn perform_integration_read<C: IntegrationConnector>(
        &self,
        entry: &Entry<'_>,
        connection: &InstanceConnection,
        connector: &C,
    ) -> Result<ActionOutcome, CoreError> {
        let started = Instant::now();
        let result = connector.perform(connection, entry.request).await;
        let row = entry.read(&result, elapsed_ms(started), stamp());
        self.record_integration_action(&row, None).await?;
        Ok(entry.conclude(&row, result.as_ref().ok(), false))
    }

    async fn perform_integration_write<C: IntegrationConnector>(
        &self,
        entry: &Entry<'_>,
        connection: &InstanceConnection,
        connector: &C,
    ) -> Result<ActionOutcome, CoreError> {
        let before = match entry.request.before_read() {
            None => None,
            Some(read) => {
                let started = Instant::now();
                match connector.perform(connection, &read).await {
                    Ok(state) => Some(state),
                    Err(failure) => {
                        let row = entry.unreadable(&failure, elapsed_ms(started), stamp());
                        self.record_integration_action(&row, None).await?;
                        return Ok(entry.conclude(&row, None, false));
                    }
                }
            }
        };
        let mut request = entry.request.clone();
        let checked = publication_guard(entry.instance, &request, before.as_ref())
            .and_then(|()| restore_masked(&mut request, before.as_ref()));
        if let Err(denial) = checked {
            let row = entry.denied(&denial, stamp());
            self.record_integration_action(&row, None).await?;
            return Ok(entry.conclude(&row, None, false));
        }
        fill_from_before(&mut request, before.as_ref());
        let entry = Entry {
            request: &request,
            ..*entry
        };
        let pending = entry.pending(before.as_ref(), stamp());
        self.record_integration_action(&pending, before.as_ref())
            .await?;
        let started = Instant::now();
        let result = connector.perform(connection, &request).await;
        let latency_ms = elapsed_ms(started);
        let after = match result.as_ref() {
            Ok(value) => match request.after_read(value) {
                Some(read) => connector.perform(connection, &read).await.ok(),
                None => Some(value.clone()),
            },
            Err(_) => None,
        };
        let completed = entry.complete(pending, &result, after.as_ref(), latency_ms, stamp());
        self.complete_integration_action(&completed).await?;
        Ok(entry.conclude(&completed, result.as_ref().ok(), true))
    }
}
