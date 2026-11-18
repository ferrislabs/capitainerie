use capitainerie_common::{CoreError, OrganizationId, SkillId};

use super::{CatalogSkill, Skill};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait SkillRepository: Send {
    fn insert(&mut self, skill: &Skill) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn update(&mut self, skill: &Skill) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: SkillId,
    ) -> impl Future<Output = Result<Option<CatalogSkill>, CoreError>> + Send;

    /// By name; `archived` of `None` lists every skill.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        archived: Option<bool>,
    ) -> impl Future<Output = Result<Vec<CatalogSkill>, CoreError>> + Send;

    /// The active skills among `slugs`, in no particular order.
    fn active_by_slugs(
        &mut self,
        organization_id: OrganizationId,
        slugs: &[String],
    ) -> impl Future<Output = Result<Vec<Skill>, CoreError>> + Send;
}
