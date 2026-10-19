use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{ClientId, ProjectId};
use capitainerie_core::domain::card::attributes::{CardFilter, Priority};
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::response::CardResponse;

#[derive(TypedPath)]
#[typed_path("/api/v1/cards")]
pub struct CardsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCardsQuery {
    pub client_id: Option<ClientId>,
    pub project_id: Option<ProjectId>,
    /// `low`, `normal`, `high` or `urgent`.
    pub priority: Option<String>,
    /// FerrisKey subject of the assignee.
    pub assignee: Option<String>,
    /// One label, matched regardless of case.
    pub label: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CardListResponse {
    pub items: Vec<CardResponse>,
}

#[utoipa::path(
    get,
    path = "/api/v1/cards",
    operation_id = "listCards",
    tag = super::TAG,
    params(ListCardsQuery),
    responses(
        (status = 200, description = "The organization's board, most recently changed first; every filter given must match", body = CardListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An unknown priority", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: CardsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListCardsQuery>,
) -> Result<Json<CardListResponse>, ApiError> {
    let filter = CardFilter {
        client_id: query.client_id,
        project_id: query.project_id,
        priority: query
            .priority
            .as_deref()
            .map(Priority::try_from)
            .transpose()?,
        assignee: query.assignee,
        label: query.label,
    };
    let cards = state.usecase.list_cards(caller.subject(), filter).await?;
    Ok(Json(CardListResponse {
        items: cards.into_iter().map(CardResponse::from).collect(),
    }))
}
