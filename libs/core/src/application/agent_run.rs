use capitainerie_common::{CardCommentId, CardId, CoreError, OrganizationId, RunId};
use capitainerie_events::card::{CardAuthor, CardMoved};
use capitainerie_macros::transactional;
use chrono::{SubsecRound, Utc};
use serde_json::Value;

use super::{CapitainerieUseCase, execution::COMMENT_ID_FIELD};
use crate::{
    domain::{
        agent::SecretToken,
        agent_identity::{AgentClientId, IdentityStatus, ports::AgentIdentityRepository},
        agent_task::{
            AgentRunError, AgentRunRefusal, AgentTask, RepositoryTarget, StartAgentRun, actor_of,
            ports::AgentTaskRepository,
        },
        approval::{
            Approval, ApprovalStatus, ApprovedCall, ToolGrant, ports::ApprovalRepository,
            service::ApprovalService,
        },
        authorization::{AccessRequest, Action, Decision, ResourceKind, Subject, require},
        card::{
            Author, Card, CardComment, Column,
            ports::{CardCommentRepository, CardRepository},
            service::CardService,
        },
        crew::{CrewAgent, ports::CrewRepository},
        delegation::{
            DelegatedToken,
            ports::{DelegationVault, TokenExchanger},
        },
        event::ports::EventEmitter,
        execution::{RepositoryRef, ports::GithubInstallationRepository},
        mcp::{
            McpConnector,
            ports::{McpConnectionRepository, McpConnectorRepository},
        },
        member::ports::MemberRepository,
        provider::{
            ModelRef, ProviderKind,
            ports::{ModelAssignmentRepository, ProviderRepository},
        },
        run::{
            Run, RunSubject, StepKind,
            ports::{RunRepository, StepRepository},
        },
        skill::{Skill, ports::SkillRepository},
        user_provider::ports::UserProviderRepository,
    },
    infrastructure::{
        agent_identity::postgres::PgAgentIdentityRepository,
        agent_task::postgres::PgAgentTaskRepository,
        card::postgres::PgCardRepository,
        crew::postgres::PgCrewRepository,
        delegation::PgDelegationVault,
        execution::postgres::{PgGithubInstallationRepository, executor_status_in},
        mcp::postgres::PgMcpConnectionRepository,
        postgres::SharedTx,
        provider::postgres::PgProviderRepository,
        run::postgres::PgRunRepository,
        user_provider::postgres::PgUserProviderRepository,
    },
};

#[derive(Debug, Clone)]
pub struct StartedAgentRun {
    pub run_id: RunId,
}

/// The agent an execution runs as, when a person started it from an agent.
#[derive(Debug, Clone)]
pub struct ExecutionAgentSource {
    pub task: AgentTask,
    pub agent: CrewAgent,
    /// The agent's active skills, in the order it lists them.
    pub skills: Vec<Skill>,
}

fn in_agent_order(mut found: Vec<Skill>, agent: &CrewAgent) -> Vec<Skill> {
    let mut skills = Vec::with_capacity(found.len());
    for slug in agent.skills.as_slice() {
        if let Some(index) = found.iter().position(|skill| skill.slug.as_str() == slug) {
            skills.push(found.swap_remove(index));
        }
    }
    skills
}

#[derive(Debug, Clone)]
pub struct AgentTaskSource {
    pub task: AgentTask,
    pub card: Card,
    pub comments: Vec<CardComment>,
    pub agent: CrewAgent,
    pub connectors: Vec<McpConnector>,
    /// The agent's active skills, in the order it lists them.
    pub skills: Vec<Skill>,
    /// The approved call this run was started to carry out, if any.
    pub approved_call: Option<ApprovedCall>,
}

/// What lets the agent of a run act for the person starting it: the
/// person's token, exchanged for one addressed to the agent's own client.
pub struct RunDelegation<'a, X> {
    pub exchanger: &'a X,
    pub issuer: &'a str,
    pub user_token: &'a SecretToken,
}

/// What a member is called in text the system writes.
pub(super) const UNNAMED_MEMBER: &str = "un membre";

type Stored<T> = Result<Result<T, AgentRunRefusal>, CoreError>;

impl CapitainerieUseCase {
    pub async fn start_agent_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
    ) -> Result<StartedAgentRun, AgentRunError> {
        Ok(self
            .store_agent_run(subject, card_id, command, None, None)
            .await??)
    }

    /// Starts the run like `start_agent_run`, and stores the person's token
    /// exchanged for the agent's client, so the run can act for them. The
    /// exchange comes first: an identity provider outage refuses the start
    /// instead of queueing a run without its delegation.
    pub async fn start_agent_run_delegated<X: TokenExchanger>(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
        delegation: RunDelegation<'_, X>,
    ) -> Result<StartedAgentRun, AgentRunError> {
        let delegated = match self
            .assigned_agent_client(subject.organization_id, card_id)
            .await?
        {
            Some(client) => {
                let token = delegation
                    .exchanger
                    .for_agent(delegation.issuer, delegation.user_token, client.as_str())
                    .await?;
                Some((client, token))
            }
            None => None,
        };
        Ok(self
            .store_agent_run(subject, card_id, command, None, delegated)
            .await??)
    }

    /// The client of the card's agent when its identity is active. Anything
    /// else is refused by `store_agent_run`, which checks it again.
    #[transactional(card)]
    async fn assigned_agent_client(
        &self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> Result<Option<AgentClientId>, CoreError> {
        let mut card_repository = card_repository;
        let Some(agent_id) = card_repository
            .find(organization_id, card_id)
            .await?
            .and_then(|card| card.attributes.assignee_agent_id)
        else {
            return Ok(None);
        };
        let active = PgAgentIdentityRepository::new(&tx, None)
            .summary(organization_id, agent_id)
            .await?
            .is_some_and(|identity| identity.status == IdentityStatus::Active);
        Ok(active.then(|| AgentClientId::of(agent_id)))
    }

    /// Starts a run of the card's agent for `subject`, the approved call of
    /// `resume` granted once; the card version is not checked.
    pub(super) async fn start_resumed_agent_run(
        &self,
        subject: Subject,
        card_id: CardId,
        resume: Approval,
    ) -> Result<StartedAgentRun, AgentRunError> {
        let command = StartAgentRun {
            expected_version: 0,
            allow_writes: false,
        };
        Ok(self
            .store_agent_run(subject, card_id, command, Some(resume), None)
            .await??)
    }

    #[transactional(
        card,
        run,
        model_assignment,
        emitter,
        authz,
        crew_agent,
        mcp_connector,
        approval
    )]
    async fn store_agent_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
        resume: Option<Approval>,
        delegation: Option<(AgentClientId, DelegatedToken)>,
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
        if resume.is_none() {
            card.ensure_version(command.expected_version)?;
        }
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
        if agent.paused {
            return Ok(Err(AgentRunRefusal::AgentPaused));
        }
        let active = PgAgentIdentityRepository::new(&tx, None)
            .summary(organization_id, agent.id)
            .await?
            .is_some_and(|identity| identity.status == IdentityStatus::Active);
        if !active {
            return Ok(Err(AgentRunRefusal::IdentityNotActive));
        }
        if delegation
            .as_ref()
            .is_some_and(|(client, _)| *client != AgentClientId::of(agent.id))
        {
            return Err(CoreError::Conflict("the card's agent changed".into()));
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
                created_by: subject.id.clone(),
                created_at: now,
            })
            .await?;
        if let Some((client, token)) = &delegation {
            PgDelegationVault::new(&tx, self.cipher()?)
                .store(run.id, client.as_str(), token)
                .await?;
        }
        if let Some(approval) = resume {
            ApprovalService::new(approval_repository, emitter, authz)
                .link_resumed(&approval, run.id, subject.id, now)
                .await?;
        }
        Ok(Ok(StartedAgentRun { run_id: run.id }))
    }

    /// The read-only half of `start_agent_repository_run`, so a handler can
    /// refuse before it calls GitHub; the run checks everything again.
    pub async fn check_agent_repository_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
        repository: RepositoryRef,
    ) -> Result<(), AgentRunError> {
        self.vetted_repository_run(subject, card_id, command, repository)
            .await??;
        Ok(())
    }

    #[transactional()]
    async fn vetted_repository_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
        repository: RepositoryRef,
    ) -> Stored<()> {
        Ok(self
            .vet_repository_run(&tx, &subject, card_id, command, &repository)
            .await?
            .map(|_| ()))
    }

    /// Starts an execution of the card's agent on `target`: read-only audit
    /// unless writes are allowed, then the usual branch and draft pull request.
    pub async fn start_agent_repository_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
        target: RepositoryTarget,
    ) -> Result<StartedAgentRun, AgentRunError> {
        Ok(self
            .store_agent_repository_run(subject, card_id, command, target)
            .await??)
    }

    #[transactional(run, model_assignment, emitter)]
    async fn store_agent_repository_run(
        &self,
        subject: Subject,
        card_id: CardId,
        command: StartAgentRun,
        target: RepositoryTarget,
    ) -> Stored<StartedAgentRun> {
        let mut run_repository = run_repository;
        let mut model_assignment_repository = model_assignment_repository;
        let base_branch = target
            .base_branch
            .clone()
            .ok_or_else(|| CoreError::Invalid("the base branch is unknown".into()))?;
        let agent = match self
            .vet_repository_run(&tx, &subject, card_id, command, &target.repository)
            .await?
        {
            Ok(agent) => agent,
            Err(refusal) => return Ok(Err(refusal)),
        };
        let organization_id = subject.organization_id;
        let now = Utc::now().trunc_subsecs(6);
        let (mut run, queued) = Run::queue_execution(
            organization_id,
            card_id,
            target.repository,
            base_branch,
            subject.id.clone(),
            now,
        );
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

    /// Everything a repository run of the card's agent needs, in the order a
    /// person can fix it.
    async fn vet_repository_run(
        &self,
        tx: &SharedTx<'_>,
        subject: &Subject,
        card_id: CardId,
        command: StartAgentRun,
        repository: &RepositoryRef,
    ) -> Stored<CrewAgent> {
        let organization_id = subject.organization_id;
        require(
            &self.authz,
            AccessRequest::new(
                subject.clone(),
                Action::AgentRun,
                ResourceKind::Card,
                Some(card_id.0),
            ),
        )
        .await?;
        if command.allow_writes {
            require(
                &self.authz,
                AccessRequest::new(
                    subject.clone(),
                    Action::CardExecute,
                    ResourceKind::Card,
                    Some(card_id.0),
                ),
            )
            .await?;
        }
        let card = PgCardRepository::new(tx)
            .find(organization_id, card_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        card.ensure_version(command.expected_version)?;
        let Some(agent_id) = card.attributes.assignee_agent_id else {
            return Ok(Err(AgentRunRefusal::NoAssignee));
        };
        let Some(agent) = PgCrewRepository::new(tx)
            .find(organization_id, agent_id)
            .await?
        else {
            return Ok(Err(AgentRunRefusal::NoAssignee));
        };
        if agent.archived {
            return Ok(Err(AgentRunRefusal::AgentArchived));
        }
        if agent.paused {
            return Ok(Err(AgentRunRefusal::AgentPaused));
        }
        if !agent.repository_grants.allows(repository) {
            return Ok(Err(AgentRunRefusal::RepositoryNotGranted(
                repository.to_string(),
            )));
        }
        if PgGithubInstallationRepository::new(tx)
            .get(organization_id)
            .await?
            .is_none()
        {
            return Ok(Err(AgentRunRefusal::GithubNotConnected));
        }
        if let Some(status) = executor_status_in(tx).await?
            && !status.enabled
        {
            return Ok(Err(AgentRunRefusal::ExecutorDisabled(
                status
                    .reason
                    .unwrap_or_else(|| "aucun exécutant disponible".into()),
            )));
        }
        let own = PgUserProviderRepository::new(tx, self.secrets.as_deref())
            .list(organization_id, &subject.id)
            .await?;
        let anthropic = own
            .iter()
            .any(|summary| summary.kind == ProviderKind::Anthropic)
            || PgProviderRepository::new(tx, self.secrets.as_deref())
                .list(organization_id)
                .await?
                .iter()
                .any(|summary| summary.kind == ProviderKind::Anthropic);
        if !anthropic {
            return Ok(Err(AgentRunRefusal::NoProvider));
        }
        if PgRunRepository::new(tx)
            .has_active_execution(card_id)
            .await?
        {
            return Err(CoreError::Conflict("execution_already_active".into()));
        }
        Ok(Ok(agent))
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

    /// `None` for an execution no agent was asked to run (a worksite). An
    /// execution whose agent vanished is refused rather than run without it.
    #[transactional(crew_agent, skill)]
    pub async fn execution_agent_source(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Option<ExecutionAgentSource>, CoreError> {
        let mut crew_agent_repository = crew_agent_repository;
        let mut skill_repository = skill_repository;
        let Some(task) = PgAgentTaskRepository::new(&tx)
            .find(organization_id, run_id)
            .await?
        else {
            return Ok(None);
        };
        let agent = crew_agent_repository
            .find(organization_id, task.agent_id)
            .await?
            .ok_or_else(|| CoreError::Invalid("l'agent de l'exécution est introuvable".into()))?;
        let skills = in_agent_order(
            skill_repository
                .active_by_slugs(organization_id, agent.skills.as_slice())
                .await?,
            &agent,
        );
        Ok(Some(ExecutionAgentSource {
            task,
            agent,
            skills,
        }))
    }

    #[transactional(card, card_comment, crew_agent, mcp_connector, skill, approval, member)]
    pub async fn agent_task_source(
        &self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> Result<Option<AgentTaskSource>, CoreError> {
        let mut card_repository = card_repository;
        let mut card_comment_repository = card_comment_repository;
        let mut crew_agent_repository = crew_agent_repository;
        let mut mcp_connector_repository = mcp_connector_repository;
        let mut skill_repository = skill_repository;
        let mut approval_repository = approval_repository;
        let mut member_repository = member_repository;
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
        let skills = in_agent_order(
            skill_repository
                .active_by_slugs(organization_id, agent.skills.as_slice())
                .await?,
            &agent,
        );
        let mut approved_call = None;
        if let Some(grant) = approval_repository
            .grant_for_run(organization_id, run_id)
            .await?
            && let Some(approval) = approval_repository
                .find(organization_id, grant.approval_id)
                .await?
        {
            let decided_by = match approval.decided_by.as_deref() {
                Some(subject) => member_repository
                    .find(organization_id, subject)
                    .await?
                    .map(|member| member.name)
                    .filter(|name| !name.trim().is_empty()),
                None => None,
            };
            approved_call = Some(ApprovedCall {
                arguments: approval.shown_arguments(),
                decided_by: decided_by.unwrap_or_else(|| UNNAMED_MEMBER.to_owned()),
                note: approval.decision_note,
                grant,
            });
        }
        Ok(Some(AgentTaskSource {
            task,
            card,
            comments,
            agent,
            connectors,
            skills,
            approved_call,
        }))
    }

    /// Spends the run's grant when it covers this call.
    #[transactional(approval, emitter, authz)]
    pub(super) async fn use_tool_grant(
        &self,
        organization_id: OrganizationId,
        grant: &ToolGrant,
        tool: &str,
        arguments: &Value,
    ) -> Result<bool, CoreError> {
        ApprovalService::new(approval_repository, emitter, authz)
            .use_grant(organization_id, grant, tool, arguments, Utc::now())
            .await
    }

    /// The display name of a member, if known.
    #[transactional(member)]
    pub(super) async fn member_name(
        &self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> Result<Option<String>, CoreError> {
        let mut member_repository = member_repository;
        Ok(member_repository
            .find(organization_id, subject)
            .await?
            .map(|member| member.name)
            .filter(|name| !name.trim().is_empty()))
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

/// Where an agent task run stands, as the board shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStage {
    Started,
    Succeeded,
    Failed,
}

impl CapitainerieUseCase {
    /// Moves the card of an agent task along with its run: in progress when
    /// the run starts (a done card reopens: someone replied), done when it
    /// succeeds, back to todo when it fails. A run
    /// that stops on a pending approval leaves the card in progress. The run lifecycle drives the move, not an actor's
    /// choice, so the PDP is not asked; a card a human has settled, proposed
    /// or moved elsewhere is left where it is.
    #[transactional(card, approval, emitter)]
    pub async fn track_task_card(
        &self,
        run: &Run,
        card_id: CardId,
        stage: TaskStage,
    ) -> Result<(), CoreError> {
        let (organization_id, run_id) = (run.organization_id, run.id);
        let mut card_repository = card_repository;
        let mut approval_repository = approval_repository;
        let Some(mut card) = card_repository.find(organization_id, card_id).await? else {
            return Ok(());
        };
        let waiting = stage == TaskStage::Succeeded
            && approval_repository
                .list(organization_id, Some(ApprovalStatus::Pending))
                .await?
                .iter()
                .any(|approval| approval.run_id == Some(run_id));
        let to = match (stage, card.column) {
            _ if waiting => return Ok(()),
            (TaskStage::Started, Column::Todo | Column::Done) => Column::InProgress,
            (TaskStage::Succeeded, Column::Todo | Column::InProgress) => Column::Done,
            (TaskStage::Failed, Column::InProgress) => Column::Todo,
            _ => return Ok(()),
        };
        let from = card.column;
        card.column = to;
        card.touch(Utc::now());
        card_repository.update(&card).await?;
        let author = Author {
            subject: run.initiated_by.clone(),
            actor: None,
        };
        emitter.emit(&CardMoved {
            card_id: card.id.0,
            organization_id: organization_id.0,
            from: from.as_str().to_owned(),
            to: to.as_str().to_owned(),
            version: card.version,
            run_id: Some(run_id.0),
            author: CardAuthor::from(&author),
        })?;
        Ok(())
    }
}

impl CapitainerieUseCase {
    /// The person who answered on the card while `run` was going, the latest
    /// one, with the card's version: their reply has not been read by an agent
    /// yet. Agent comments never count, so an agent never relaunches itself.
    #[transactional(card, card_comment)]
    async fn unread_reply(
        &self,
        run: &Run,
        card_id: CardId,
    ) -> Result<Option<(Subject, i32)>, CoreError> {
        let mut card_repository = card_repository;
        let mut card_comment_repository = card_comment_repository;
        let Some(card) = card_repository.get(run.organization_id, card_id).await? else {
            return Ok(None);
        };
        if matches!(card.column, Column::Proposed | Column::Rejected) {
            return Ok(None);
        }
        let comments = card_comment_repository
            .list(run.organization_id, card_id)
            .await?;
        Ok(comments
            .into_iter()
            .rev()
            .find(|comment| comment.author.actor.is_none() && comment.created_at > run.created_at)
            .map(|comment| {
                (
                    Subject {
                        id: comment.author.subject,
                        organization_id: run.organization_id,
                        actor: None,
                        is_admin: false,
                    },
                    card.version,
                )
            }))
    }

    /// Starts the card's agent again when someone replied while `run` was
    /// going: the reply would otherwise wait for a click. The new run reads
    /// the whole thread, the reply included, as the person who wrote it.
    pub async fn relaunch_on_reply(
        &self,
        run: &Run,
        card_id: CardId,
    ) -> Result<Option<StartedAgentRun>, AgentRunError> {
        let Some((subject, version)) = self.unread_reply(run, card_id).await? else {
            return Ok(None);
        };
        let command = StartAgentRun {
            expected_version: version,
            allow_writes: false,
        };
        match self.start_agent_run(subject, card_id, command).await {
            Ok(started) => Ok(Some(started)),
            Err(AgentRunError::Core(CoreError::Conflict(_))) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
