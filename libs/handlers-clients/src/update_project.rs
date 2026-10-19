use axum::{Extension, Json, extract::State};
use capitainerie_common::ClientId;
use capitainerie_core::domain::project::service::UpdateProject;
use capitainerie_handlers::{ApiError, AppState, Caller, Patch};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{get_project::ProjectPath, response::ProjectResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProjectRequest {
    #[serde(default)]
    #[schema(value_type = Option<ClientId>)]
    pub client_id: Patch<ClientId>,
    pub name: Option<String>,
    pub aliases: Option<Vec<String>>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub repository: Patch<String>,
    pub description: Option<String>,
    pub archived: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/v1/projects/{project_id}",
    operation_id = "updateProject",
    tag = super::TAG,
    params(("project_id" = capitainerie_common::ProjectId, Path, description = "Project identifier")),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "The updated project; absent fields are unchanged, null clears client_id or repository", body = ProjectResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage projects", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such project in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A project of that name exists for this client, whatever the case", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, invalid input, or an unknown, archived or foreign client", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ProjectPath { project_id }: ProjectPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, ApiError> {
    let project = state
        .usecase
        .update_project(
            caller.subject(),
            project_id,
            UpdateProject {
                client_id: request.client_id.change(),
                name: request.name,
                aliases: request.aliases,
                repository: request.repository.change(),
                description: request.description,
                archived: request.archived,
            },
        )
        .await?;
    Ok(Json(project.into()))
}
