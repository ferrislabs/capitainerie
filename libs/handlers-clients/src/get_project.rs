use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::ProjectId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::ProjectDetailResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/projects/{project_id}")]
pub struct ProjectPath {
    pub project_id: ProjectId,
}

#[utoipa::path(
    get,
    path = "/api/v1/projects/{project_id}",
    operation_id = "getProject",
    tag = super::TAG,
    params(("project_id" = ProjectId, Path, description = "Project identifier")),
    responses(
        (status = 200, description = "The project with its card count per column", body = ProjectDetailResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no client access", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such project in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ProjectPath { project_id }: ProjectPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ProjectDetailResponse>, ApiError> {
    let detail = state
        .usecase
        .get_project(caller.subject(), project_id)
        .await?;
    Ok(Json(detail.into()))
}
