use capitainerie_common::{AgentId, CoreError};
use chrono::Utc;

use super::{
    AgentStatus, ConnectorGrants, CrewAgent, DESCRIPTION_MAX_CHARS, Hue, INSTRUCTIONS_MAX_CHARS,
    MODEL_MAX_CHARS, ROLE_MAX_CHARS, Slug, closes_a_loop, ports::CrewRepository,
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    card::bounded_text,
    client::links::Change,
    label::Name,
};

#[derive(Debug, Clone)]
pub struct CreateAgent {
    pub slug: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub model: String,
    pub hue: i16,
    pub reports_to: Option<AgentId>,
    pub instructions: String,
    pub connector_grants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateAgent {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub description: Option<String>,
    pub model: Option<String>,
    pub hue: Option<i16>,
    pub reports_to: Change<AgentId>,
    pub archived: Option<bool>,
    pub instructions: Option<String>,
    pub connector_grants: Option<Vec<String>>,
}

impl Default for UpdateAgent {
    fn default() -> Self {
        Self {
            slug: None,
            name: None,
            role: None,
            description: None,
            model: None,
            hue: None,
            reports_to: Change::Keep,
            archived: None,
            instructions: None,
            connector_grants: None,
        }
    }
}

impl UpdateAgent {
    fn changes_nothing(&self) -> bool {
        self.slug.is_none()
            && self.name.is_none()
            && self.role.is_none()
            && self.description.is_none()
            && self.model.is_none()
            && self.hue.is_none()
            && self.reports_to == Change::Keep
            && self.archived.is_none()
            && self.instructions.is_none()
            && self.connector_grants.is_none()
    }
}

pub struct CrewService<'a, R, A> {
    agents: R,
    authorizer: &'a A,
}

impl<'a, R, A> CrewService<'a, R, A>
where
    R: CrewRepository,
    A: Authorizer,
{
    pub fn new(agents: R, authorizer: &'a A) -> Self {
        Self { agents, authorizer }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<CrewAgent>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::AgentRead, None).await?;
        self.agents
            .list(organization_id, Some(archived.unwrap_or(false)))
            .await
    }

    pub async fn get(mut self, subject: Subject, id: AgentId) -> Result<CrewAgent, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::AgentRead, Some(id)).await?;
        self.agents
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    pub async fn create(
        mut self,
        subject: Subject,
        command: CreateAgent,
    ) -> Result<CrewAgent, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject.clone(), Action::AgentManage, None)
            .await?;
        if let Some(manager) = command.reports_to {
            self.agents
                .find(organization_id, manager)
                .await?
                .ok_or_else(unknown_manager)?;
        }
        let now = Utc::now();
        let agent = CrewAgent {
            id: AgentId::new(),
            organization_id,
            slug: Slug::try_from(command.slug)?,
            name: Name::try_from(command.name)?.as_str().to_owned(),
            role: trimmed(command.role, ROLE_MAX_CHARS, "role")?,
            description: bounded_text(
                command.description,
                DESCRIPTION_MAX_CHARS,
                "description",
                true,
            )?,
            model: trimmed(command.model, MODEL_MAX_CHARS, "model")?,
            status: AgentStatus::Idle,
            hue: Hue::try_from(command.hue)?,
            reports_to: command.reports_to,
            archived: false,
            instructions: bounded_text(
                command.instructions,
                INSTRUCTIONS_MAX_CHARS,
                "instructions",
                true,
            )?,
            connector_grants: ConnectorGrants::try_from(command.connector_grants)?,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
        };
        self.agents.insert(&agent).await?;
        Ok(agent)
    }

    pub async fn update(
        mut self,
        subject: Subject,
        id: AgentId,
        command: UpdateAgent,
    ) -> Result<CrewAgent, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::AgentManage, Some(id)).await?;
        let mut agent = self
            .agents
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if command.changes_nothing() {
            return Err(CoreError::Invalid(
                "nothing to update: give at least one field".into(),
            ));
        }
        if let Some(slug) = command.slug {
            agent.slug = Slug::try_from(slug)?;
        }
        if let Some(name) = command.name {
            agent.name = Name::try_from(name)?.as_str().to_owned();
        }
        if let Some(role) = command.role {
            agent.role = trimmed(role, ROLE_MAX_CHARS, "role")?;
        }
        if let Some(description) = command.description {
            agent.description =
                bounded_text(description, DESCRIPTION_MAX_CHARS, "description", true)?;
        }
        if let Some(model) = command.model {
            agent.model = trimmed(model, MODEL_MAX_CHARS, "model")?;
        }
        if let Some(hue) = command.hue {
            agent.hue = Hue::try_from(hue)?;
        }
        if let Some(instructions) = command.instructions {
            agent.instructions =
                bounded_text(instructions, INSTRUCTIONS_MAX_CHARS, "instructions", true)?;
        }
        if let Some(grants) = command.connector_grants {
            agent.connector_grants = ConnectorGrants::try_from(grants)?;
        }
        if let Some(archived) = command.archived {
            agent.archived = archived;
        }
        match command.reports_to {
            Change::Keep => {}
            Change::Clear => agent.reports_to = None,
            Change::Set(manager) => {
                let all = self.agents.list(organization_id, None).await?;
                if !all.iter().any(|candidate| candidate.id == manager) {
                    return Err(unknown_manager());
                }
                if closes_a_loop(&all, id, manager) {
                    return Err(CoreError::Invalid(
                        "an agent cannot report to itself or to one of its own reports".into(),
                    ));
                }
                agent.reports_to = Some(manager);
            }
        }
        agent.updated_at = Utc::now();
        self.agents.update(&agent).await?;
        Ok(agent)
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        id: Option<AgentId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, action, ResourceKind::Agent, id.map(|id| id.0)),
        )
        .await
    }
}

fn unknown_manager() -> CoreError {
    CoreError::Invalid("the manager is not an agent of the organization".into())
}

fn trimmed(value: String, max: usize, what: &str) -> Result<String, CoreError> {
    bounded_text(value.trim().to_owned(), max, what, true)
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        crew::{ports::MockCrewRepository, tests::agent},
    };

    fn human(organization_id: OrganizationId) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id,
            actor: None,
            is_admin: false,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    fn command(slug: &str) -> CreateAgent {
        CreateAgent {
            slug: slug.into(),
            name: " Dev ".into(),
            role: " Développeur ".into(),
            description: String::new(),
            model: "claude-sonnet".into(),
            hue: 210,
            reports_to: None,
            instructions: "Tu codes.".into(),
            connector_grants: vec!["github".into()],
        }
    }

    fn holding(agents: Vec<CrewAgent>) -> MockCrewRepository {
        let mut repository = MockCrewRepository::new();
        let found = agents.clone();
        repository.expect_find().returning(move |_, id| {
            let found = found.iter().find(|agent| agent.id == id).cloned();
            Box::pin(async move { Ok(found) })
        });
        repository.expect_list().returning(move |_, _| {
            let all = agents.clone();
            Box::pin(async move { Ok(all) })
        });
        repository
            .expect_update()
            .returning(|_| Box::pin(async { Ok(()) }));
        repository
    }

    #[tokio::test]
    async fn an_agent_is_created_idle_and_clean() {
        let mut repository = MockCrewRepository::new();
        repository
            .expect_insert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = allow();
        let organization_id = OrganizationId::new();

        let created = CrewService::new(repository, &authorizer)
            .create(human(organization_id), command("dev"))
            .await
            .unwrap();

        assert_eq!(created.name, "Dev");
        assert_eq!(created.role, "Développeur");
        assert_eq!(created.status, AgentStatus::Idle);
        assert!(!created.archived);
        assert_eq!(created.organization_id, organization_id);
        assert_eq!(created.created_by, "user-1");
    }

    #[tokio::test]
    async fn invalid_input_stores_nothing() {
        let authorizer = allow();
        let mut bad_slug = command("Not Kebab");
        bad_slug.name = "Dev".into();
        let mut bad_hue = command("dev");
        bad_hue.hue = 400;
        let mut bad_name = command("dev");
        bad_name.name = " ".into();
        for invalid in [bad_slug, bad_hue, bad_name] {
            let error = CrewService::new(MockCrewRepository::new(), &authorizer)
                .create(human(OrganizationId::new()), invalid)
                .await
                .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)));
        }
    }

    #[tokio::test]
    async fn a_manager_of_another_organization_is_refused() {
        let authorizer = allow();
        let mut repository = MockCrewRepository::new();
        repository
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        let mut orphan = command("dev");
        orphan.reports_to = Some(AgentId::new());
        let error = CrewService::new(repository, &authorizer)
            .create(human(OrganizationId::new()), orphan)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn reporting_loops_are_refused() {
        let organization_id = OrganizationId::new();
        let boss = agent(organization_id, "boss");
        let mut lead = agent(organization_id, "lead");
        lead.reports_to = Some(boss.id);
        let authorizer = allow();

        for (target, manager) in [(boss.id, lead.id), (boss.id, boss.id)] {
            let error = CrewService::new(holding(vec![boss.clone(), lead.clone()]), &authorizer)
                .update(
                    human(organization_id),
                    target,
                    UpdateAgent {
                        reports_to: Change::Set(manager),
                        ..UpdateAgent::default()
                    },
                )
                .await
                .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)));
        }

        let updated = CrewService::new(holding(vec![boss.clone(), lead.clone()]), &authorizer)
            .update(
                human(organization_id),
                lead.id,
                UpdateAgent {
                    reports_to: Change::Clear,
                    archived: Some(true),
                    ..UpdateAgent::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.reports_to, None);
        assert!(updated.archived);
    }

    #[tokio::test]
    async fn an_empty_update_and_an_unknown_agent_are_refused() {
        let organization_id = OrganizationId::new();
        let dev = agent(organization_id, "dev");
        let authorizer = allow();
        let error = CrewService::new(holding(vec![dev.clone()]), &authorizer)
            .update(human(organization_id), dev.id, UpdateAgent::default())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));

        let error = CrewService::new(holding(vec![]), &authorizer)
            .get(human(organization_id), AgentId::new())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::NotFound));
    }

    #[tokio::test]
    async fn a_denied_manager_changes_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "humans only".into(),
                })
            })
        });
        let error = CrewService::new(MockCrewRepository::new(), &authorizer)
            .create(human(OrganizationId::new()), command("dev"))
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
