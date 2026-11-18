use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use capitainerie_common::{AgentId, ProjectId};
use capitainerie_core::domain::source::service::CreateSource;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::{PublicBaseUrl, list_sources::SourcesPath, response::IssuedSourceResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSourceRequest {
    /// Kebab-case, unique in the organization.
    pub slug: String,
    pub name: String,
    /// `webhook`, the only kind for now and the default.
    pub kind: Option<String>,
    pub description: Option<String>,
    /// True by default.
    pub enabled: Option<bool>,
    /// A JSON object of at most 64 KiB; `{}` by default.
    #[schema(value_type = Option<Object>)]
    pub config: Option<Value>,
    pub default_agent_id: Option<AgentId>,
    pub project_id: Option<ProjectId>,
    #[serde(default)]
    pub auto_run: bool,
    #[serde(default)]
    pub allow_writes: bool,
}

#[utoipa::path(
    post,
    path = "/api/v1/sources",
    operation_id = "createSource",
    tag = super::TAG,
    request_body = CreateSourceRequest,
    responses(
        (status = 201, description = "The new source with its webhook token and URL, shown this once", body = IssuedSourceResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage sources", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A source of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An invalid field, or an agent or project not usable in the organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: SourcesPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(base): Extension<PublicBaseUrl>,
    headers: HeaderMap,
    Json(request): Json<CreateSourceRequest>,
) -> Result<(StatusCode, Json<IssuedSourceResponse>), ApiError> {
    let issued = state
        .usecase
        .create_source(
            caller.subject(),
            CreateSource {
                slug: request.slug,
                name: request.name,
                kind: request.kind.unwrap_or_else(|| "webhook".into()),
                description: request.description.unwrap_or_default(),
                enabled: request.enabled.unwrap_or(true),
                config: request.config,
                default_agent_id: request.default_agent_id,
                project_id: request.project_id,
                auto_run: request.auto_run,
                allow_writes: request.allow_writes,
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(IssuedSourceResponse::new(issued, &base, &headers)),
    ))
}
