use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::SourceListResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/sources")]
pub struct SourcesPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListSourcesQuery {
    /// Absent lists every source.
    pub archived: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/v1/sources",
    operation_id = "listSources",
    tag = super::TAG,
    params(ListSourcesQuery),
    responses(
        (status = 200, description = "The organization's sources by name, each with its events of the last 24 hours and its last event", body = SourceListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to sources", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: SourcesPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListSourcesQuery>,
) -> Result<Json<SourceListResponse>, ApiError> {
    let sources = state
        .usecase
        .list_sources(caller.subject(), query.archived)
        .await?;
    Ok(Json(SourceListResponse {
        items: sources.into_iter().map(Into::into).collect(),
    }))
}
