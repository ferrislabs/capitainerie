use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::response::{ApprovalListResponse, ApprovalStatusDto};

#[derive(TypedPath)]
#[typed_path("/api/v1/approvals")]
pub struct ApprovalsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListApprovalsQuery {
    /// Absent lists every approval.
    pub status: Option<ApprovalStatusDto>,
}

#[utoipa::path(
    get,
    path = "/api/v1/approvals",
    operation_id = "listApprovals",
    tag = super::TAG,
    params(ListApprovalsQuery),
    responses(
        (status = 200, description = "The organization's approvals, newest first (the latest 200), with the pending count", body = ApprovalListResponse),
        (status = 400, description = "An unknown status", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to approvals", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: ApprovalsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListApprovalsQuery>,
) -> Result<Json<ApprovalListResponse>, ApiError> {
    let list = state
        .usecase
        .list_approvals(caller.subject(), query.status.map(Into::into))
        .await?;
    Ok(Json(ApprovalListResponse {
        items: list.items.into_iter().map(Into::into).collect(),
        pending: list.pending,
    }))
}
