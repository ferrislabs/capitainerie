use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::client::ListFilter;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::ClientListResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/clients")]
pub struct ClientsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListClientsQuery {
    pub q: Option<String>,
    pub archived: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/v1/clients",
    operation_id = "listClients",
    tag = super::TAG,
    params(ListClientsQuery),
    responses(
        (status = 200, description = "The organization's clients by name; without archived, the active ones only", body = ClientListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no client access", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: ClientsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListClientsQuery>,
) -> Result<Json<ClientListResponse>, ApiError> {
    let listings = state
        .usecase
        .list_clients(
            caller.subject(),
            ListFilter {
                query: query.q,
                archived: query.archived,
            },
        )
        .await?;
    Ok(Json(ClientListResponse {
        items: listings.into_iter().map(Into::into).collect(),
    }))
}
