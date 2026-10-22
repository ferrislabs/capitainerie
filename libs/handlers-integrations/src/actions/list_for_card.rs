use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::CardId;
use capitainerie_handlers::Caller;
use serde::Deserialize;

use super::{describe, dto::IntegrationActionListResponse};
use crate::{IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}/integration-actions")]
pub struct CardIntegrationActionsPath {
    pub card_id: CardId,
}

#[utoipa::path(
    get,
    path = "/api/v1/cards/{card_id}/integration-actions",
    operation_id = "listCardIntegrationActions",
    tag = crate::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    responses(
        (status = 200, description = "Every action the Intervenant took for the card on a client's instances, oldest first, with the instance each one targeted; the sealed copy kept before a write is never part of it", body = IntegrationActionListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no integration access", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    CardIntegrationActionsPath { card_id }: CardIntegrationActionsPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<IntegrationActionListResponse>, IntegrationError> {
    let subject = caller.subject();
    let actions = state
        .usecase
        .integration_actions_of_card(subject.clone(), card_id)
        .await?;
    Ok(Json(describe(&state.usecase, subject, actions).await?))
}
