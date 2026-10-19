use axum::{Extension, Json, extract::State, http::StatusCode};
use capitainerie_core::domain::client::service::CreateClient;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{list_clients::ClientsPath, response::ClientResponse};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateClientRequest {
    pub name: String,
    pub aliases: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/clients",
    operation_id = "createClient",
    tag = super::TAG,
    request_body = CreateClientRequest,
    responses(
        (status = 201, description = "The new client", body = ClientResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage clients", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A client of that name exists, whatever the case", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Invalid name, alias or notes", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: ClientsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateClientRequest>,
) -> Result<(StatusCode, Json<ClientResponse>), ApiError> {
    let client = state
        .usecase
        .create_client(
            caller.subject(),
            CreateClient {
                name: request.name,
                aliases: request.aliases.unwrap_or_default(),
                notes: request.notes.unwrap_or_default(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(client.into())))
}
