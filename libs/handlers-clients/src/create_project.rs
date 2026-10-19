use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_common::ClientId;
use capitainerie_core::domain::project::service::CreateProject;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{list_projects::ProjectsPath, response::ProjectResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub client_id: Option<ClientId>,
    pub name: String,
    pub aliases: Option<Vec<String>>,
    pub repository: Option<String>,
    pub description: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/projects",
    operation_id = "createProject",
    tag = super::TAG,
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "The new project", body = ProjectResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage projects", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A project of that name exists for this client, whatever the case", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Invalid input, or an unknown, archived or foreign client", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: ProjectsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), ApiError> {
    let project = state
        .usecase
        .create_project(
            caller.subject(),
            CreateProject {
                client_id: request.client_id,
                name: request.name,
                aliases: request.aliases.unwrap_or_default(),
                repository: request.repository,
                description: request.description.unwrap_or_default(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(project.into())))
}
