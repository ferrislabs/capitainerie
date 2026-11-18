use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{RoutineId, RoutineTriggerId};
use capitainerie_core::domain::routine::{
    IssuedTrigger,
    service::{AddTrigger, UpdateTrigger},
};
use capitainerie_handlers::{ApiError, AppState, Caller, PublicBaseUrl};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    HOOK_PREFIX,
    response::{IssuedTriggerResponse, RoutineResponse, ScheduleDto, TriggerKindDto},
};

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/routines/{routine_id}/triggers")]
pub struct TriggersPath {
    pub routine_id: RoutineId,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/routines/{routine_id}/triggers/{trigger_id}")]
pub struct TriggerPath {
    pub routine_id: RoutineId,
    pub trigger_id: RoutineTriggerId,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/routines/{routine_id}/triggers/{trigger_id}/rotate-token")]
pub struct RotateTriggerTokenPath {
    pub routine_id: RoutineId,
    pub trigger_id: RoutineTriggerId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddTriggerRequest {
    pub kind: TriggerKindDto,
    /// True by default.
    pub enabled: Option<bool>,
    /// Required for a planning trigger, refused for a webhook.
    pub schedule: Option<ScheduleDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTriggerRequest {
    pub enabled: Option<bool>,
    /// Planning triggers only; replaces the whole schedule.
    pub schedule: Option<ScheduleDto>,
}

fn issued(
    issued: IssuedTrigger,
    base: &PublicBaseUrl,
    headers: &HeaderMap,
) -> IssuedTriggerResponse {
    let token = issued.token.map(|token| token.expose().to_owned());
    IssuedTriggerResponse {
        webhook_url: token
            .as_deref()
            .map(|token| format!("{}{HOOK_PREFIX}{token}", base.resolve(headers))),
        token,
        trigger_id: issued.trigger_id,
        routine: issued.listing.into(),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/routines/{routine_id}/triggers",
    operation_id = "addRoutineTrigger",
    tag = super::TAG,
    params(("routine_id" = RoutineId, Path, description = "Routine identifier")),
    request_body = AddTriggerRequest,
    responses(
        (status = 201, description = "The routine with its new trigger; a webhook's token and URL are shown this once", body = IssuedTriggerResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "A missing, unexpected or invalid schedule", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_trigger(
    TriggersPath { routine_id }: TriggersPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(base): Extension<PublicBaseUrl>,
    headers: HeaderMap,
    Json(request): Json<AddTriggerRequest>,
) -> Result<(StatusCode, Json<IssuedTriggerResponse>), ApiError> {
    let added = state
        .usecase
        .add_routine_trigger(
            caller.subject(),
            routine_id,
            AddTrigger {
                kind: match request.kind {
                    TriggerKindDto::Planning => "planning",
                    TriggerKindDto::Webhook => "webhook",
                }
                .to_owned(),
                enabled: request.enabled.unwrap_or(true),
                schedule: request.schedule.map(Into::into),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(issued(added, &base, &headers))))
}

#[utoipa::path(
    patch,
    path = "/api/v1/routines/{routine_id}/triggers/{trigger_id}",
    operation_id = "updateRoutineTrigger",
    tag = super::TAG,
    params(
        ("routine_id" = RoutineId, Path, description = "Routine identifier"),
        ("trigger_id" = RoutineTriggerId, Path, description = "Trigger identifier"),
    ),
    request_body = UpdateTriggerRequest,
    responses(
        (status = 200, description = "The routine; the trigger's next passage is planned again from now", body = RoutineResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine or trigger in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, or an invalid schedule", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_trigger(
    TriggerPath {
        routine_id,
        trigger_id,
    }: TriggerPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateTriggerRequest>,
) -> Result<Json<RoutineResponse>, ApiError> {
    let routine = state
        .usecase
        .update_routine_trigger(
            caller.subject(),
            routine_id,
            trigger_id,
            UpdateTrigger {
                enabled: request.enabled,
                schedule: request.schedule.map(Into::into),
            },
        )
        .await?;
    Ok(Json(routine.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/routines/{routine_id}/triggers/{trigger_id}",
    operation_id = "removeRoutineTrigger",
    tag = super::TAG,
    params(
        ("routine_id" = RoutineId, Path, description = "Routine identifier"),
        ("trigger_id" = RoutineTriggerId, Path, description = "Trigger identifier"),
    ),
    responses(
        (status = 200, description = "The routine without the trigger; a webhook's URL stops working at once", body = RoutineResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine or trigger in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn remove_trigger(
    TriggerPath {
        routine_id,
        trigger_id,
    }: TriggerPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<RoutineResponse>, ApiError> {
    let routine = state
        .usecase
        .remove_routine_trigger(caller.subject(), routine_id, trigger_id)
        .await?;
    Ok(Json(routine.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/routines/{routine_id}/triggers/{trigger_id}/rotate-token",
    operation_id = "rotateRoutineTriggerToken",
    tag = super::TAG,
    params(
        ("routine_id" = RoutineId, Path, description = "Routine identifier"),
        ("trigger_id" = RoutineTriggerId, Path, description = "Trigger identifier"),
    ),
    responses(
        (status = 200, description = "The new token and webhook URL, shown this once; the previous URL stops working at once", body = IssuedTriggerResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine or trigger in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "The trigger is not a webhook", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn rotate_trigger_token(
    RotateTriggerTokenPath {
        routine_id,
        trigger_id,
    }: RotateTriggerTokenPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(base): Extension<PublicBaseUrl>,
    headers: HeaderMap,
) -> Result<Json<IssuedTriggerResponse>, ApiError> {
    let rotated = state
        .usecase
        .rotate_routine_trigger_token(caller.subject(), routine_id, trigger_id)
        .await?;
    Ok(Json(issued(rotated, &base, &headers)))
}
