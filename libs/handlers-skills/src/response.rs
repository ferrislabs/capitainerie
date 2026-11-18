use capitainerie_common::SkillId;
use capitainerie_core::domain::skill::CatalogSkill;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct SkillResponse {
    pub id: SkillId,
    /// Kebab-case; what an agent's `skills` list names.
    pub slug: String,
    pub name: String,
    pub description: String,
    /// The instructions, in Markdown, added to the system prompt of the agents using the skill.
    pub content: String,
    /// `manual`, or the URL the skill was imported from.
    pub source: String,
    pub archived: bool,
    /// Active agents whose `skills` list names the slug.
    pub used_by: i64,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SkillListResponse {
    pub items: Vec<SkillResponse>,
}

impl From<CatalogSkill> for SkillResponse {
    fn from(CatalogSkill { skill, used_by }: CatalogSkill) -> Self {
        Self {
            id: skill.id,
            slug: skill.slug.as_str().to_owned(),
            name: skill.name,
            description: skill.description,
            content: skill.content,
            source: skill.source.as_str().to_owned(),
            archived: skill.archived,
            used_by,
            created_by: skill.created_by,
            created_at: skill.created_at,
            updated_at: skill.updated_at,
        }
    }
}
