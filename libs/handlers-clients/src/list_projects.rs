use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::ClientId;
use capitainerie_core::domain::project::ProjectFilter;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::ProjectListResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/projects")]
pub struct ProjectsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListProjectsQuery {
    pub q: Option<String>,
    pub archived: Option<bool>,
    pub client_id: Option<ClientId>,
}

#[utoipa::path(
    get,
    path = "/api/v1/projects",
    operation_id = "listProjects",
    tag = super::TAG,
    params(ListProjectsQuery),
    responses(
        (status = 200, description = "The organization's projects by name; without archived, the active ones only", body = ProjectListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no client access", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: ProjectsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<ProjectListResponse>, ApiError> {
    let listings = state
        .usecase
        .list_projects(
            caller.subject(),
            ProjectFilter {
                query: query.q,
                archived: query.archived,
                client_id: query.client_id,
            },
        )
        .await?;
    Ok(Json(ProjectListResponse {
        items: listings.into_iter().map(Into::into).collect(),
    }))
}
