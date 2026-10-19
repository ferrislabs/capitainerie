use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::ClientId;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;

use crate::response::ClientDetailResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/clients/{client_id}")]
pub struct ClientPath {
    pub client_id: ClientId,
}

#[utoipa::path(
    get,
    path = "/api/v1/clients/{client_id}",
    operation_id = "getClient",
    tag = super::TAG,
    params(("client_id" = ClientId, Path, description = "Client identifier")),
    responses(
        (status = 200, description = "The client with its active projects and its card count per column", body = ClientDetailResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no client access", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such client in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ClientPath { client_id }: ClientPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ClientDetailResponse>, ApiError> {
    let detail = state
        .usecase
        .get_client(caller.subject(), client_id)
        .await?;
    Ok(Json(detail.into()))
}
