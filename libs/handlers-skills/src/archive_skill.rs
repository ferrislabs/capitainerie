use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::SkillId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::SkillResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/skills/{skill_id}/archive")]
pub struct ArchiveSkillPath {
    pub skill_id: SkillId,
}

#[utoipa::path(
    post,
    path = "/api/v1/skills/{skill_id}/archive",
    operation_id = "archiveSkill",
    tag = super::TAG,
    params(("skill_id" = SkillId, Path, description = "Skill identifier")),
    responses(
        (status = 200, description = "The archived skill; agent runs no longer receive it. Archiving twice changes nothing", body = SkillResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage the catalog", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such skill in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ArchiveSkillPath { skill_id }: ArchiveSkillPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<SkillResponse>, ApiError> {
    let skill = state
        .usecase
        .archive_skill(caller.subject(), skill_id)
        .await?;
    Ok(Json(skill.into()))
}
