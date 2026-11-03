use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::ClientId;
use capitainerie_handlers::Caller;
use serde::Deserialize;
use utoipa::IntoParams;

use super::dto::IntegrationInstanceListResponse;
use crate::{IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/clients/{client_id}/integrations")]
pub struct ClientIntegrationsPath {
    pub client_id: ClientId,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListInstancesQuery {
    pub archived: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/v1/clients/{client_id}/integrations",
    operation_id = "listIntegrationInstances",
    tag = crate::TAG,
    params(
        ("client_id" = ClientId, Path, description = "Client identifier"),
        ListInstancesQuery,
    ),
    responses(
        (status = 200, description = "The client's Odoo and n8n instances by name, without their secrets; archived ones only with archived=true", body = IntegrationInstanceListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no integration access", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such client in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    ClientIntegrationsPath { client_id }: ClientIntegrationsPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListInstancesQuery>,
) -> Result<Json<IntegrationInstanceListResponse>, IntegrationError> {
    let instances = state
        .usecase
        .list_integration_instances(caller.subject(), client_id, query.archived.unwrap_or(false))
        .await?;
    Ok(Json(IntegrationInstanceListResponse {
        items: instances.into_iter().map(Into::into).collect(),
    }))
}
