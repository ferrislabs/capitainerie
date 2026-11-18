use axum::{Extension, Json, extract::State};
use capitainerie_common::SkillId;
use capitainerie_core::domain::skill::service::UpdateSkill;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{get_skill::SkillPath, response::SkillResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSkillRequest {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub source: Option<String>,
    /// false brings an archived skill back.
    pub archived: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/v1/skills/{skill_id}",
    operation_id = "updateSkill",
    tag = super::TAG,
    params(("skill_id" = SkillId, Path, description = "Skill identifier")),
    request_body = UpdateSkillRequest,
    responses(
        (status = 200, description = "The updated skill; absent fields are unchanged", body = SkillResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage the catalog", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such skill in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A skill of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, or an invalid field", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    SkillPath { skill_id }: SkillPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateSkillRequest>,
) -> Result<Json<SkillResponse>, ApiError> {
    let skill = state
        .usecase
        .update_skill(
            caller.subject(),
            skill_id,
            UpdateSkill {
                slug: request.slug,
                name: request.name,
                description: request.description,
                content: request.content,
                source: request.source,
                archived: request.archived,
            },
        )
        .await?;
    Ok(Json(skill.into()))
}
