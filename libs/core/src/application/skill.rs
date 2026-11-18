use capitainerie_common::{CoreError, SkillId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    authorization::Subject,
    skill::{
        CatalogSkill,
        service::{CreateSkill, SkillService, UpdateSkill},
    },
};

impl CapitainerieUseCase {
    #[transactional(skill, authz)]
    pub async fn list_skills(
        &self,
        subject: Subject,
        archived: Option<bool>,
    ) -> Result<Vec<CatalogSkill>, CoreError> {
        SkillService::new(skill_repository, authz)
            .list(subject, archived)
            .await
    }

    #[transactional(skill, authz)]
    pub async fn get_skill(
        &self,
        subject: Subject,
        id: SkillId,
    ) -> Result<CatalogSkill, CoreError> {
        SkillService::new(skill_repository, authz)
            .get(subject, id)
            .await
    }

    #[transactional(skill, authz)]
    pub async fn create_skill(
        &self,
        subject: Subject,
        command: CreateSkill,
    ) -> Result<CatalogSkill, CoreError> {
        SkillService::new(skill_repository, authz)
            .create(subject, command)
            .await
    }

    #[transactional(skill, authz)]
    pub async fn update_skill(
        &self,
        subject: Subject,
        id: SkillId,
        command: UpdateSkill,
    ) -> Result<CatalogSkill, CoreError> {
        SkillService::new(skill_repository, authz)
            .update(subject, id, command)
            .await
    }

    #[transactional(skill, authz)]
    pub async fn archive_skill(
        &self,
        subject: Subject,
        id: SkillId,
    ) -> Result<CatalogSkill, CoreError> {
        SkillService::new(skill_repository, authz)
            .archive(subject, id)
            .await
    }
}
