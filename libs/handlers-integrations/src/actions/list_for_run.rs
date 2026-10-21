use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::RunId;
use capitainerie_handlers::Caller;
use serde::Deserialize;

use super::{describe, dto::IntegrationActionListResponse};
use crate::{IntegrationError, state::IntegrationsState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/runs/{run_id}/integration-actions")]
pub struct RunIntegrationActionsPath {
    pub run_id: RunId,
}

#[utoipa::path(
    get,
    path = "/api/v1/runs/{run_id}/integration-actions",
    operation_id = "listRunIntegrationActions",
    tag = crate::TAG,
    params(("run_id" = RunId, Path, description = "Run identifier")),
    responses(
        (status = 200, description = "Every action of the intervention, oldest first, with the instance each one targeted; the sealed copy kept before a write is never part of it", body = IntegrationActionListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no integration access", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such run in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    RunIntegrationActionsPath { run_id }: RunIntegrationActionsPath,
    State(state): State<IntegrationsState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<IntegrationActionListResponse>, IntegrationError> {
    let subject = caller.subject();
    let actions = state
        .usecase
        .integration_actions_of_run(subject.clone(), run_id)
        .await?;
    Ok(Json(describe(&state.usecase, subject, actions).await?))
}
