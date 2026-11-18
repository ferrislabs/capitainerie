use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::ApprovalId;
use capitainerie_core::domain::approval::Verdict;
use capitainerie_handlers::{ApiError, AppState, Caller};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::response::DecideApprovalResponse;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/approvals/{approval_id}/decide")]
pub struct DecideApprovalPath {
    pub approval_id: ApprovalId,
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionDto {
    Approve,
    Reject,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DecideApprovalRequest {
    pub decision: ApprovalDecisionDto,
    /// Why; at most 2000 characters, blank is dropped.
    pub note: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/approvals/{approval_id}/decide",
    operation_id = "decideApproval",
    tag = super::TAG,
    params(("approval_id" = ApprovalId, Path, description = "Approval identifier")),
    request_body = DecideApprovalRequest,
    responses(
        (status = 200, description = "The decided approval. An approved tool call on a card starts a new run of the card's agent, for the initiator of the run that asked, allowed to make exactly that call once (`resumed_run_id`), or says why it could not (`resume_error`); a rejected one is noted on the card", body = DecideApprovalResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the decision; agents never decide", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such approval in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "The approval is no longer pending", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "A note over 2000 characters", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    DecideApprovalPath { approval_id }: DecideApprovalPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<DecideApprovalRequest>,
) -> Result<Json<DecideApprovalResponse>, ApiError> {
    let verdict = match request.decision {
        ApprovalDecisionDto::Approve => Verdict::Approve,
        ApprovalDecisionDto::Reject => Verdict::Reject,
    };
    let decided = state
        .usecase
        .decide_approval(caller.subject(), approval_id, verdict, request.note)
        .await?;
    Ok(Json(DecideApprovalResponse {
        approval: decided.approval.into(),
        resume_error: decided.resume_error,
    }))
}
