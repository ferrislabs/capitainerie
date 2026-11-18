use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_core::domain::skill::service::CreateSkill;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{list_skills::SkillsPath, response::SkillResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSkillRequest {
    /// Kebab-case, unique in the organization.
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    /// Markdown instructions; at most 100 000 characters.
    pub content: Option<String>,
    /// `manual` (the default) or the http(s) URL the skill comes from.
    pub source: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/skills",
    operation_id = "createSkill",
    tag = super::TAG,
    request_body = CreateSkillRequest,
    responses(
        (status = 201, description = "The new skill", body = SkillResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage the catalog", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A skill of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An invalid field", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: SkillsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateSkillRequest>,
) -> Result<(StatusCode, Json<SkillResponse>), ApiError> {
    let skill = state
        .usecase
        .create_skill(
            caller.subject(),
            CreateSkill {
                slug: request.slug,
                name: request.name,
                description: request.description.unwrap_or_default(),
                content: request.content.unwrap_or_default(),
                source: request.source.unwrap_or_default(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(skill.into())))
}
