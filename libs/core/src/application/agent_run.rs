use capitainerie_common::{CardCommentId, CardId, CoreError, OrganizationId, RunId};
use capitainerie_macros::transactional;
use chrono::{SubsecRound, Utc};

use super::{CapitainerieUseCase, execution::COMMENT_ID_FIELD};
use crate::{
    domain::{
        agent_task::{
            AgentRunError, AgentRunRefusal, AgentTask, StartAgentRun, actor_of,
            ports::AgentTaskRepository,
        },
        authorization::{AccessRequest, Action, Decision, ResourceKind, Subject, require},
        card::{
            Card, CardComment,
            ports::{CardCommentRepository, CardRepository},
            service::CardService,
        },
        crew::{CrewAgent, ports::CrewRepository},
        event::ports::EventEmitter,
        mcp::{
            McpConnector,
            ports::{McpConnectionRepository, McpConnectorRepository},
        },
        provider::{
            ModelRef,
            ports::{ModelAssignmentRepository, ProviderRepository},
        },
        run::{
            Run, RunSubject, StepKind,
            ports::{RunRepository, StepRepository},
        },
        user_provider::ports::UserProviderRepository,
    },
    infrastructure::{
        agent_task::postgres::PgAgentTaskRepository, mcp::postgres::PgMcpConnectionRepository,
        provider::postgres::PgProviderRepository,
        user_provider::postgres::PgUserProviderRepository,
    },
};

#[derive(Debug, Clone)]
pub struct StartedAgentRun {
    pub run_id: RunId,
}

#[derive(Debug, Clone)]
pub struct AgentTaskSource {
    pub task: AgentTask,
    pub card: Card,
    pub comments: Vec<CardComment>,
    pub agent: CrewAgent,
    pub connectors: Vec<McpConnector>,
}

type Stored<T> = Result<Result<T, AgentRunRefusal>, CoreError>;

impl CapitainerieUseCase {
    pub async fn start_agent_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
    ) -> Result<StartedAgentRun, AgentRunError> {
        Ok(self.store_agent_run(subject, card_id, command).await??)
    }

    #[transactional(card, run, model_assignment, emitter, authz, crew_agent, mcp_connector)]
    async fn store_agent_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
    ) -> Stored<StartedAgentRun> {
        let mut card_repository = card_repository;
        let mut run_repository = run_repository;
        let mut model_assignment_repository = model_assignment_repository;
        let mut crew_agent_repository = crew_agent_repository;
        let mut mcp_connector_repository = mcp_connector_repository;
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(
                subject.clone(),
                Action::AgentRun,
                ResourceKind::Card,
                Some(card_id.0),
            ),
        )
        .await?;
        let card = card_repository
            .find(organization_id, card_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        card.ensure_version(command.expected_version)?;
        let Some(agent_id) = card.attributes.assignee_agent_id else {
            return Ok(Err(AgentRunRefusal::NoAssignee));
        };
        let Some(agent) = crew_agent_repository
            .find(organization_id, agent_id)
            .await?
        else {
            return Ok(Err(AgentRunRefusal::NoAssignee));
        };
        if agent.archived {
            return Ok(Err(AgentRunRefusal::AgentArchived));
        }
        let own = PgUserProviderRepository::new(&tx, self.secrets.as_deref())
            .list(organization_id, &subject.id)
            .await?;
        if own.is_empty()
            && PgProviderRepository::new(&tx, self.secrets.as_deref())
                .list(organization_id)
                .await?
                .is_empty()
        {
            return Ok(Err(AgentRunRefusal::NoProvider));
        }
        let usable = mcp_connector_repository
            .list(organization_id, Some(false))
            .await?;
        let granted: Vec<&McpConnector> = agent
            .connector_grants
            .as_slice()
            .iter()
            .filter_map(|slug| {
                usable
                    .iter()
                    .find(|connector| connector.slug.as_str() == slug)
            })
            .collect();
        if granted.is_empty() {
            return Ok(Err(AgentRunRefusal::NoConnector));
        }
        let connected = PgMcpConnectionRepository::new(&tx, self.secrets.as_deref())
            .expiries(organization_id, &subject.id)
            .await?;
        if let Some(missing) = granted
            .iter()
            .find(|connector| !connected.iter().any(|(id, _)| *id == connector.id))
        {
            return Ok(Err(AgentRunRefusal::NotConnected(
                missing.slug.as_str().to_owned(),
            )));
        }
        if run_repository.has_active_execution(card_id).await? {
            return Err(CoreError::Conflict("execution_already_active".into()));
        }
        let now = Utc::now().trunc_subsecs(6);
        let (mut run, queued) =
            Run::queue_agent_task(organization_id, card_id, subject.id.clone(), now);
        run.models = model_assignment_repository.get(organization_id).await?;
        run_repository.insert(&run).await?;
        emitter.emit(&queued)?;
        PgAgentTaskRepository::new(&tx)
            .insert(&AgentTask {
                run_id: run.id,
                organization_id,
                card_id,
                agent_id: agent.id,
                allow_writes: command.allow_writes,
                model: None,
                created_by: subject.id,
                created_at: now,
            })
            .await?;
        Ok(Ok(StartedAgentRun { run_id: run.id }))
    }

    #[transactional()]
    pub(super) async fn freeze_agent_task_model(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
        model: &ModelRef,
    ) -> Result<ModelRef, CoreError> {
        PgAgentTaskRepository::new(&tx)
            .freeze_model(organization_id, run_id, model)
            .await
    }

    pub async fn is_agent_task_run(&self, id: RunId) -> Result<bool, CoreError> {
        sqlx::query_scalar::<_, bool>("SELECT 'agent_task' = ANY(pipeline) FROM runs WHERE id = $1")
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .map(|found| found.unwrap_or(false))
            .map_err(|error| CoreError::Database(error.to_string()))
    }

    #[transactional(card, card_comment, crew_agent, mcp_connector)]
    pub async fn agent_task_source(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Option<AgentTaskSource>, CoreError> {
        let mut card_repository = card_repository;
        let mut card_comment_repository = card_comment_repository;
        let mut crew_agent_repository = crew_agent_repository;
        let mut mcp_connector_repository = mcp_connector_repository;
        let Some(task) = PgAgentTaskRepository::new(&tx)
            .find(organization_id, run_id)
            .await?
        else {
            return Ok(None);
        };
        let Some(card) = card_repository.find(organization_id, task.card_id).await? else {
            return Ok(None);
        };
        let Some(agent) = crew_agent_repository
            .find(organization_id, task.agent_id)
            .await?
        else {
            return Ok(None);
        };
        let comments = card_comment_repository
            .list(organization_id, task.card_id)
            .await?;
        let usable = mcp_connector_repository
            .list(organization_id, Some(false))
            .await?;
        let connectors = agent
            .connector_grants
            .as_slice()
            .iter()
            .filter_map(|slug| {
                usable
                    .iter()
                    .find(|connector| connector.slug.as_str() == slug)
                    .cloned()
            })
            .collect();
        Ok(Some(AgentTaskSource {
            task,
            card,
            comments,
            agent,
            connectors,
        }))
    }

    pub async fn authorize_mcp_call(
        &self,
        subject: Subject,
        run_id: RunId,
        connector: &McpConnector,
        initiated: bool,
        granted: bool,
    ) -> Result<Decision, CoreError> {
        use crate::domain::authorization::ports::Authorizer;
        self.authz
            .evaluate(
                &AccessRequest::new(
                    subject,
                    Action::McpCall { initiated, granted },
                    ResourceKind::McpConnector,
                    Some(connector.id.0),
                )
                .in_run(Some(run_id)),
            )
            .await
    }

    #[transactional(card, card_comment, transcript, run, step, emitter, authz)]
    pub async fn comment_card_as_agent(
        &self,
        run_id: RunId,
        card_id: CardId,
        agent_slug: &str,
        body: String,
    ) -> Result<CardCommentId, CoreError> {
        let mut run_repository = run_repository;
        let mut step_repository = step_repository;
        let run = run_repository
            .lock(run_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if !matches!(run.subject, RunSubject::AgentTask { card_id: owner } if owner == card_id) {
            return Err(CoreError::Invalid(
                "the run does not belong to this card".into(),
            ));
        }
        let mut task_step = step_repository
            .list(run_id)
            .await?
            .into_iter()
            .find(|step| step.kind == StepKind::AgentTask);
        if let Some(posted) = task_step
            .as_ref()
            .and_then(|step| step.output.as_ref())
            .and_then(|output| output[COMMENT_ID_FIELD].as_str())
            .and_then(|id| id.parse::<uuid::Uuid>().ok())
        {
            return Ok(CardCommentId(posted));
        }
        let comment = CardService::new(
            card_repository,
            card_comment_repository,
            transcript_repository,
            emitter,
            authz,
        )
        .comment(
            Subject {
                id: run.initiated_by.clone(),
                organization_id: run.organization_id,
                actor: Some(actor_of(agent_slug)),
                is_admin: false,
            },
            card_id,
            body,
            Some(run_id),
        )
        .await?;
        if let Some(step) = task_step.as_mut() {
            let mut output = step.output.take().unwrap_or_else(|| serde_json::json!({}));
            output[COMMENT_ID_FIELD] = serde_json::json!(comment.id.0.to_string());
            step.output = Some(output);
            step_repository.save(step).await?;
        }
        Ok(comment.id)
    }
}
