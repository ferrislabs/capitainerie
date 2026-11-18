use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::SkillListResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/skills")]
pub struct SkillsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListSkillsQuery {
    /// Absent lists every skill.
    pub archived: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/v1/skills",
    operation_id = "listSkills",
    tag = super::TAG,
    params(ListSkillsQuery),
    responses(
        (status = 200, description = "The organization's skills by name, each with the number of active agents using it", body = SkillListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to the catalog", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: SkillsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListSkillsQuery>,
) -> Result<Json<SkillListResponse>, ApiError> {
    let skills = state
        .usecase
        .list_skills(caller.subject(), query.archived)
        .await?;
    Ok(Json(SkillListResponse {
        items: skills.into_iter().map(Into::into).collect(),
    }))
}
