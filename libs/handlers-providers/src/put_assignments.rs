use axum::{Extension, Json, extract::State};
use capitainerie_core::domain::provider::ports::ModelCatalog;
use capitainerie_handlers::{ApiError, Caller};

use crate::{
    dto::{ModelAssignmentsDto, ModelAssignmentsResponse},
    get_assignments::ModelAssignmentsPath,
    state::ProvidersState,
};

#[utoipa::path(
    put,
    path = "/api/v1/model-assignments",
    operation_id = "putModelAssignments",
    tag = super::TAG,
    request_body = ModelAssignmentsDto,
    responses(
        (status = 200, description = "The model chosen for each role; runs queued from now on use it", body = ModelAssignmentsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is not an organization admin", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "A provider is not configured, or a model is unknown or lacks the capability of its role", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<C: ModelCatalog + 'static>(
    _: ModelAssignmentsPath,
    State(state): State<ProvidersState<C>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<ModelAssignmentsDto>,
) -> Result<Json<ModelAssignmentsResponse>, ApiError> {
    let current = state
        .usecase
        .get_model_assignments(caller.subject())
        .await?;
    let assignments = request.into_assignments(current.as_ref())?;
    let stored = state
        .usecase
        .put_model_assignments(caller.subject(), assignments, state.catalog.as_ref())
        .await?;
    Ok(Json(ModelAssignmentsResponse {
        assignments: Some(stored.into()),
    }))
}
