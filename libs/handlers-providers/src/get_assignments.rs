use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_handlers::{ApiError, Caller};
use serde::Deserialize;

use crate::{dto::ModelAssignmentsResponse, state::ProvidersState};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/model-assignments")]
pub struct ModelAssignmentsPath;

#[utoipa::path(
    get,
    path = "/api/v1/model-assignments",
    operation_id = "getModelAssignments",
    tag = super::TAG,
    responses(
        (status = 200, description = "The model chosen for each role, `null` when the organization chose none", body = ModelAssignmentsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: Send + Sync + 'static>(
    _: ModelAssignmentsPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<ModelAssignmentsResponse>, ApiError> {
    let assignments = state
        .usecase
        .get_model_assignments(caller.subject())
        .await?;
    Ok(Json(ModelAssignmentsResponse {
        assignments: assignments.map(Into::into),
    }))
}
