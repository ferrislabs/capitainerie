use capitainerie_common::{CoreError, SkillId};
use chrono::Utc;

use super::{
    CONTENT_MAX_CHARS, CatalogSkill, DESCRIPTION_MAX_CHARS, Skill, SkillSource,
    ports::SkillRepository,
};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    card::bounded_text,
    crew::Slug,
    label::Name,
};

#[derive(Debug, Clone)]
pub struct CreateSkill {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSkill {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub source: Option<String>,
    pub archived: Option<bool>,
}

impl UpdateSkill {
    fn changes_nothing(&self) -> bool {
        self.slug.is_none()
            && self.name.is_none()
            && self.description.is_none()
            && self.content.is_none()
            && self.source.is_none()
            && self.archived.is_none()
    }
}

pub struct SkillService<'a, R, A> {
    skills: R,
    authorizer: &'a A,
}

impl<'a, R, A> SkillService<'a, R, A>
where
    R: SkillRepository,
    A: Authorizer,
{
    pub fn new(skills: R, authorizer: &'a A) -> Self {
        Self { skills, authorizer }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<CatalogSkill>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::SkillRead, None).await?;
        self.skills.list(organization_id, archived).await
    }

    pub async fn get(mut self, subject: Subject, id: SkillId) -> Result<CatalogSkill, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::SkillRead, Some(id)).await?;
        self.skills
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    pub async fn create(
        mut self,
        subject: Subject,
        command: CreateSkill,
    ) -> Result<CatalogSkill, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject.clone(), Action::SkillManage, None)
            .await?;
        let now = Utc::now();
        let skill = Skill {
            id: SkillId::new(),
            organization_id,
            slug: Slug::try_from(command.slug)?,
            name: Name::try_from(command.name)?.as_str().to_owned(),
            description: description(command.description)?,
            content: content(command.content)?,
            source: SkillSource::try_from(command.source)?,
            archived: false,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
        };
        self.skills.insert(&skill).await?;
        self.skills
            .find(organization_id, skill.id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    pub async fn update(
        mut self,
        subject: Subject,
        id: SkillId,
        command: UpdateSkill,
    ) -> Result<CatalogSkill, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::SkillManage, Some(id)).await?;
        let mut skill = self
            .skills
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?
            .skill;
        if command.changes_nothing() {
            return Err(CoreError::Invalid(
                "nothing to update: give at least one field".into(),
            ));
        }
        if let Some(slug) = command.slug {
            skill.slug = Slug::try_from(slug)?;
        }
        if let Some(name) = command.name {
            skill.name = Name::try_from(name)?.as_str().to_owned();
        }
        if let Some(value) = command.description {
            skill.description = description(value)?;
        }
        if let Some(value) = command.content {
            skill.content = content(value)?;
        }
        if let Some(source) = command.source {
            skill.source = SkillSource::try_from(source)?;
        }
        if let Some(archived) = command.archived {
            skill.archived = archived;
        }
        self.save(skill).await
    }

    pub async fn archive(
        mut self,
        subject: Subject,
        id: SkillId,
    ) -> Result<CatalogSkill, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::SkillManage, Some(id)).await?;
        let found = self
            .skills
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if found.skill.archived {
            return Ok(found);
        }
        let mut skill = found.skill;
        skill.archived = true;
        self.save(skill).await
    }

    async fn save(&mut self, mut skill: Skill) -> Result<CatalogSkill, CoreError> {
        skill.updated_at = Utc::now();
        self.skills.update(&skill).await?;
        self.skills
            .find(skill.organization_id, skill.id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        id: Option<SkillId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, action, ResourceKind::Skill, id.map(|id| id.0)),
        )
        .await
    }
}

fn description(value: String) -> Result<String, CoreError> {
    bounded_text(
        value.trim().to_owned(),
        DESCRIPTION_MAX_CHARS,
        "description",
        true,
    )
}

fn content(value: String) -> Result<String, CoreError> {
    bounded_text(value, CONTENT_MAX_CHARS, "content", true)
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        skill::{ports::MockSkillRepository, tests::skill},
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

    fn command(slug: &str) -> CreateSkill {
        CreateSkill {
            slug: slug.into(),
            name: " Relecture ".into(),
            description: " Relit les PR. ".into(),
            content: "# Relecture\nRelis tout.".into(),
            source: String::new(),
        }
    }

    fn holding(found: Skill) -> MockSkillRepository {
        let mut repository = MockSkillRepository::new();
        repository.expect_find().returning(move |_, _| {
            let skill = found.clone();
            Box::pin(async move { Ok(Some(CatalogSkill { skill, used_by: 2 })) })
        });
        repository
    }

    #[tokio::test]
    async fn a_skill_is_created_clean_and_comes_back_with_its_usage() {
        let mut repository = MockSkillRepository::new();
        let stored = std::sync::Arc::new(std::sync::Mutex::new(None::<Skill>));
        let kept = stored.clone();
        repository.expect_insert().times(1).returning(move |skill| {
            *kept.lock().unwrap() = Some(skill.clone());
            Box::pin(async { Ok(()) })
        });
        let read = stored.clone();
        repository.expect_find().times(1).returning(move |_, _| {
            let skill = read.lock().unwrap().clone().unwrap();
            Box::pin(async move { Ok(Some(CatalogSkill { skill, used_by: 3 })) })
        });
        let authorizer = allow();
        let organization_id = OrganizationId::new();

        let created = SkillService::new(repository, &authorizer)
            .create(human(organization_id), command("review"))
            .await
            .unwrap();

        assert_eq!(created.used_by, 3);
        assert_eq!(created.skill.name, "Relecture");
        assert_eq!(created.skill.description, "Relit les PR.");
        assert_eq!(created.skill.source, SkillSource::Manual);
        assert_eq!(created.skill.created_by, "user-1");
        assert!(!created.skill.archived);
    }

    #[tokio::test]
    async fn invalid_input_stores_nothing() {
        let authorizer = allow();
        let mut bad_slug = command("Not Kebab");
        bad_slug.name = "Relecture".into();
        let mut bad_name = command("review");
        bad_name.name = " ".into();
        let mut too_long = command("review");
        too_long.content = "a".repeat(CONTENT_MAX_CHARS + 1);
        let mut bad_source = command("review");
        bad_source.source = "ftp://example.com".into();
        for invalid in [bad_slug, bad_name, too_long, bad_source] {
            let error = SkillService::new(MockSkillRepository::new(), &authorizer)
                .create(human(OrganizationId::new()), invalid)
                .await
                .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)));
        }
    }

    #[tokio::test]
    async fn archiving_saves_once_and_an_archived_skill_stays_as_it_is() {
        let organization_id = OrganizationId::new();
        let review = skill(organization_id, "review", "Relis.");
        let mut repository = holding(review.clone());
        repository
            .expect_update()
            .times(1)
            .withf(|skill| skill.archived)
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = allow();
        SkillService::new(repository, &authorizer)
            .archive(human(organization_id), review.id)
            .await
            .unwrap();

        let mut archived = review;
        archived.archived = true;
        let mut repository = holding(archived.clone());
        repository.expect_update().never();
        let again = SkillService::new(repository, &authorizer)
            .archive(human(organization_id), archived.id)
            .await
            .unwrap();
        assert!(again.skill.archived);
    }

    #[tokio::test]
    async fn an_update_changes_only_the_given_fields() {
        let organization_id = OrganizationId::new();
        let review = skill(organization_id, "review", "Relis.");
        let mut repository = holding(review.clone());
        repository
            .expect_update()
            .times(1)
            .withf(|skill| {
                skill.content == "Relis deux fois."
                    && skill.slug.as_str() == "review"
                    && matches!(&skill.source, SkillSource::Url(url) if url == "https://example.com/s.md")
            })
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = allow();
        SkillService::new(repository, &authorizer)
            .update(
                human(organization_id),
                review.id,
                UpdateSkill {
                    content: Some("Relis deux fois.".into()),
                    source: Some("https://example.com/s.md".into()),
                    ..UpdateSkill::default()
                },
            )
            .await
            .unwrap();

        let error = SkillService::new(holding(review.clone()), &authorizer)
            .update(human(organization_id), review.id, UpdateSkill::default())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn a_denied_member_changes_nothing() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "humans only".into(),
                })
            })
        });
        let error = SkillService::new(MockSkillRepository::new(), &authorizer)
            .create(human(OrganizationId::new()), command("review"))
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
