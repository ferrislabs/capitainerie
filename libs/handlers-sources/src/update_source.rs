use axum::{Extension, Json, extract::State};
use capitainerie_common::{AgentId, ProjectId, SourceId};
use capitainerie_core::domain::source::service::UpdateSource;
use capitainerie_handlers::{ApiError, AppState, Caller, Patch};
use serde::Deserialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::{get_source::SourcePath, response::SourceResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSourceRequest {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    /// Replaces the whole object; at most 64 KiB.
    #[schema(value_type = Option<Object>)]
    pub config: Option<Value>,
    /// null removes the default agent.
    #[serde(default)]
    #[schema(value_type = Option<AgentId>)]
    pub default_agent_id: Patch<AgentId>,
    /// null removes the project.
    #[serde(default)]
    #[schema(value_type = Option<ProjectId>)]
    pub project_id: Patch<ProjectId>,
    pub auto_run: Option<bool>,
    pub allow_writes: Option<bool>,
    /// false brings an archived source back.
    pub archived: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/v1/sources/{source_id}",
    operation_id = "updateSource",
    tag = super::TAG,
    params(("source_id" = SourceId, Path, description = "Source identifier")),
    request_body = UpdateSourceRequest,
    responses(
        (status = 200, description = "The updated source; absent fields are unchanged", body = SourceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage sources", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such source in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A source of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, an invalid field, or an agent or project not usable in the organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    SourcePath { source_id }: SourcePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateSourceRequest>,
) -> Result<Json<SourceResponse>, ApiError> {
    let source = state
        .usecase
        .update_source(
            caller.subject(),
            source_id,
            UpdateSource {
                slug: request.slug,
                name: request.name,
                description: request.description,
                enabled: request.enabled,
                config: request.config,
                default_agent_id: request.default_agent_id.change(),
                project_id: request.project_id.change(),
                auto_run: request.auto_run,
                allow_writes: request.allow_writes,
                archived: request.archived,
            },
        )
        .await?;
    Ok(Json(source.into()))
}
