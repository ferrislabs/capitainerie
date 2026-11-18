use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::SourceId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::SourceEventListResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/sources/{source_id}/events")]
pub struct SourceEventsPath {
    pub source_id: SourceId,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListSourceEventsQuery {
    /// 1 to 100, 50 by default.
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/sources/{source_id}/events",
    operation_id = "listSourceEvents",
    tag = super::TAG,
    params(
        ("source_id" = SourceId, Path, description = "Source identifier"),
        ListSourceEventsQuery,
    ),
    responses(
        (status = 200, description = "The source's events, newest first, without their payload, with the total count", body = SourceEventListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to sources", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such source in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    SourceEventsPath { source_id }: SourceEventsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListSourceEventsQuery>,
) -> Result<Json<SourceEventListResponse>, ApiError> {
    let (events, total) = state
        .usecase
        .list_source_events(
            caller.subject(),
            source_id,
            i64::from(query.limit.unwrap_or(50)),
            i64::from(query.offset.unwrap_or(0)),
        )
        .await?;
    Ok(Json(SourceEventListResponse {
        items: events.into_iter().map(Into::into).collect(),
        total,
    }))
}
