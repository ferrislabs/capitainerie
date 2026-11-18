use axum::{
    Extension, Json, Router,
    extract::{Query, State},
};
use axum_extra::routing::{RouterExt, TypedPath};
use capitainerie_common::{AgentId, ArtifactId, CardId, RunId};
use capitainerie_core::domain::artifact::{self, ArtifactFilter, ArtifactKind, ArtifactView};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub const TAG: &str = "artifacts";

pub fn router() -> Router<AppState> {
    Router::new().typed_get(list_handler).typed_get(get_handler)
}

#[derive(TypedPath)]
#[typed_path("/api/v1/artifacts")]
pub struct ArtifactsPath;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/artifacts/{artifact_id}")]
pub struct ArtifactPath {
    pub artifact_id: ArtifactId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKindDto {
    Markdown,
    Json,
    Link,
}

impl From<ArtifactKind> for ArtifactKindDto {
    fn from(kind: ArtifactKind) -> Self {
        match kind {
            ArtifactKind::Markdown => Self::Markdown,
            ArtifactKind::Json => Self::Json,
            ArtifactKind::Link => Self::Link,
        }
    }
}

impl From<ArtifactKindDto> for ArtifactKind {
    fn from(kind: ArtifactKindDto) -> Self {
        match kind {
            ArtifactKindDto::Markdown => Self::Markdown,
            ArtifactKindDto::Json => Self::Json,
            ArtifactKindDto::Link => Self::Link,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ArtifactsQuery {
    pub card_id: Option<CardId>,
    pub agent_id: Option<AgentId>,
    pub kind: Option<ArtifactKindDto>,
    /// 1 to 200, 50 by default.
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page.
    pub cursor: Option<ArtifactId>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArtifactSummaryResponse {
    pub id: ArtifactId,
    pub kind: ArtifactKindDto,
    pub title: String,
    pub card_id: Option<CardId>,
    pub card_title: Option<String>,
    pub run_id: Option<RunId>,
    pub agent_id: Option<AgentId>,
    pub agent_name: Option<String>,
    /// The target of a `link`.
    pub url: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArtifactResponse {
    #[serde(flatten)]
    pub summary: ArtifactSummaryResponse,
    /// The text, up to 1 MiB; null for a `link`.
    pub content: Option<String>,
}

fn split(view: ArtifactView) -> (ArtifactSummaryResponse, Option<String>) {
    let ArtifactView {
        artifact,
        agent_name,
        card_title,
    } = view;
    (
        ArtifactSummaryResponse {
            id: artifact.id,
            kind: artifact.kind.into(),
            title: artifact.title,
            card_id: artifact.card_id,
            card_title,
            run_id: artifact.run_id,
            agent_id: artifact.agent_id,
            agent_name,
            url: artifact.url,
            created_by: artifact.created_by,
            created_at: artifact.created_at,
            updated_at: artifact.updated_at,
        },
        artifact.content,
    )
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArtifactListResponse {
    /// Newest first, without their content.
    pub items: Vec<ArtifactSummaryResponse>,
    /// Pass it as `cursor` for the next page; null on the last one.
    pub next_cursor: Option<ArtifactId>,
}

#[utoipa::path(
    get,
    path = "/api/v1/artifacts",
    operation_id = "listArtifacts",
    tag = TAG,
    params(ArtifactsQuery),
    responses(
        (status = 200, description = "The organization's artifacts, newest first", body = ArtifactListResponse),
        (status = 400, description = "A malformed parameter or an unknown kind", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to artifacts", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "A limit out of bounds", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_handler(
    _: ArtifactsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ArtifactsQuery>,
) -> Result<Json<ArtifactListResponse>, ApiError> {
    let filter = ArtifactFilter {
        card_id: query.card_id,
        agent_id: query.agent_id,
        kind: query.kind.map(Into::into),
        before: query.cursor,
        limit: artifact::page(query.limit)?,
    };
    let page = state
        .usecase
        .list_artifacts(caller.subject(), filter)
        .await?;
    Ok(Json(ArtifactListResponse {
        items: page.items.into_iter().map(|view| split(view).0).collect(),
        next_cursor: page.next_cursor,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/artifacts/{artifact_id}",
    operation_id = "getArtifact",
    tag = TAG,
    params(("artifact_id" = ArtifactId, Path, description = "Artifact identifier")),
    responses(
        (status = 200, description = "The artifact with its content", body = ArtifactResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to artifacts", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such artifact in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_handler(
    ArtifactPath { artifact_id }: ArtifactPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ArtifactResponse>, ApiError> {
    let view = state
        .usecase
        .get_artifact(caller.subject(), artifact_id)
        .await?;
    let (summary, content) = split(view);
    Ok(Json(ArtifactResponse { summary, content }))
}
