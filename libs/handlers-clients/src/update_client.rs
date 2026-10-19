use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::client::service::UpdateClient;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{get_client::ClientPath, response::ClientResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateClientRequest {
    pub name: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub notes: Option<String>,
    pub archived: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/v1/clients/{client_id}",
    operation_id = "updateClient",
    tag = super::TAG,
    params(("client_id" = capitainerie_common::ClientId, Path, description = "Client identifier")),
    request_body = UpdateClientRequest,
    responses(
        (status = 200, description = "The updated client; absent fields are unchanged", body = ClientResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage clients", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such client in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A client of that name exists, whatever the case", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, or an invalid name, alias or notes", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ClientPath { client_id }: ClientPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateClientRequest>,
) -> Result<Json<ClientResponse>, ApiError> {
    let client = state
        .usecase
        .update_client(
            caller.subject(),
            client_id,
            UpdateClient {
                name: request.name,
                aliases: request.aliases,
                notes: request.notes,
                archived: request.archived,
            },
        )
        .await?;
    Ok(Json(client.into()))
}
