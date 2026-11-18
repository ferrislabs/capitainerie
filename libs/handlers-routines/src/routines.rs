use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{AgentId, ProjectId, RoutineId};
use capitainerie_core::domain::{
    card::attributes::Priority,
    routine::{
        IfMissed, IfOpen,
        service::{CreateRoutine, UpdateRoutine},
    },
};
use capitainerie_handlers::{ApiError, AppState, Caller, Patch};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::response::{
    IfMissedDto, IfOpenDto, PriorityDto, RoutineListResponse, RoutineResponse, ScheduleDto,
};

#[derive(TypedPath)]
#[typed_path("/api/v1/routines")]
pub struct RoutinesPath;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/routines/{routine_id}")]
pub struct RoutinePath {
    pub routine_id: RoutineId,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/routines/{routine_id}/archive")]
pub struct ArchiveRoutinePath {
    pub routine_id: RoutineId,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListRoutinesQuery {
    /// Absent lists every routine.
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoutineRequest {
    pub name: String,
    /// Kebab-case, unique in the organization; derived from the name when absent.
    pub slug: Option<String>,
    /// The description of every card, at most 20 000 characters.
    pub prompt: String,
    pub agent_id: AgentId,
    pub project_id: Option<ProjectId>,
    /// `normal` by default.
    pub priority: Option<PriorityDto>,
    /// `{nom} · {date}` by default.
    pub title_template: Option<String>,
    /// `Europe/Paris` by default.
    pub timezone: Option<String>,
    /// True by default.
    pub enabled: Option<bool>,
    /// `skip` by default.
    pub if_open: Option<IfOpenDto>,
    /// `run_once` by default.
    pub if_missed: Option<IfMissedDto>,
    #[serde(default)]
    pub allow_writes: bool,
    /// Creates a planning trigger with the routine.
    pub schedule: Option<ScheduleDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoutineRequest {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub prompt: Option<String>,
    pub agent_id: Option<AgentId>,
    /// null removes the project.
    #[serde(default)]
    #[schema(value_type = Option<ProjectId>)]
    pub project_id: Patch<ProjectId>,
    pub priority: Option<PriorityDto>,
    pub title_template: Option<String>,
    pub timezone: Option<String>,
    /// false pauses every trigger; true plans them again from now.
    pub enabled: Option<bool>,
    pub if_open: Option<IfOpenDto>,
    pub if_missed: Option<IfMissedDto>,
    pub allow_writes: Option<bool>,
    /// false brings an archived routine back.
    pub archived: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/v1/routines",
    operation_id = "listRoutines",
    tag = super::TAG,
    params(ListRoutinesQuery),
    responses(
        (status = 200, description = "The organization's routines by name, each with its triggers, next passage and last passage", body = RoutineListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to routines", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_routines(
    _: RoutinesPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<ListRoutinesQuery>,
) -> Result<Json<RoutineListResponse>, ApiError> {
    let routines = state
        .usecase
        .list_routines(caller.subject(), query.archived)
        .await?;
    Ok(Json(RoutineListResponse {
        items: routines.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/routines",
    operation_id = "createRoutine",
    tag = super::TAG,
    request_body = CreateRoutineRequest,
    responses(
        (status = 201, description = "The new routine, with its planning trigger when a schedule was given", body = RoutineResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A routine of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "An invalid field or schedule, or an agent or project not usable in the organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_routine(
    _: RoutinesPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<CreateRoutineRequest>,
) -> Result<(StatusCode, Json<RoutineResponse>), ApiError> {
    let routine = state
        .usecase
        .create_routine(
            caller.subject(),
            CreateRoutine {
                slug: request.slug,
                name: request.name,
                prompt: request.prompt,
                agent_id: request.agent_id,
                project_id: request.project_id,
                priority: request
                    .priority
                    .map_or(Priority::Normal, PriorityDto::as_domain),
                title_template: request.title_template,
                timezone: request.timezone,
                enabled: request.enabled.unwrap_or(true),
                if_open: request.if_open.map_or(IfOpen::Skip, IfOpenDto::as_domain),
                if_missed: request
                    .if_missed
                    .map_or(IfMissed::RunOnce, IfMissedDto::as_domain),
                allow_writes: request.allow_writes,
                schedule: request.schedule.map(Into::into),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(routine.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/routines/{routine_id}",
    operation_id = "getRoutine",
    tag = super::TAG,
    params(("routine_id" = RoutineId, Path, description = "Routine identifier")),
    responses(
        (status = 200, description = "The routine with its triggers and activity", body = RoutineResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_routine(
    RoutinePath { routine_id }: RoutinePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<RoutineResponse>, ApiError> {
    let routine = state
        .usecase
        .get_routine(caller.subject(), routine_id)
        .await?;
    Ok(Json(routine.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/routines/{routine_id}",
    operation_id = "updateRoutine",
    tag = super::TAG,
    params(("routine_id" = RoutineId, Path, description = "Routine identifier")),
    request_body = UpdateRoutineRequest,
    responses(
        (status = 200, description = "The updated routine; absent fields are unchanged", body = RoutineResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "A routine of that slug exists", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Nothing to update, an invalid field, or an agent or project not usable in the organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_routine(
    RoutinePath { routine_id }: RoutinePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<UpdateRoutineRequest>,
) -> Result<Json<RoutineResponse>, ApiError> {
    let routine = state
        .usecase
        .update_routine(
            caller.subject(),
            routine_id,
            UpdateRoutine {
                slug: request.slug,
                name: request.name,
                prompt: request.prompt,
                agent_id: request.agent_id,
                project_id: request.project_id.change(),
                priority: request.priority.map(PriorityDto::as_domain),
                title_template: request.title_template,
                timezone: request.timezone,
                enabled: request.enabled,
                if_open: request.if_open.map(IfOpenDto::as_domain),
                if_missed: request.if_missed.map(IfMissedDto::as_domain),
                allow_writes: request.allow_writes,
                archived: request.archived,
            },
        )
        .await?;
    Ok(Json(routine.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/routines/{routine_id}/archive",
    operation_id = "archiveRoutine",
    tag = super::TAG,
    params(("routine_id" = RoutineId, Path, description = "Routine identifier")),
    responses(
        (status = 200, description = "The archived routine; nothing starts it any more. Archiving twice changes nothing", body = RoutineResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the change; agents may not manage routines", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such routine in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn archive_routine(
    ArchiveRoutinePath { routine_id }: ArchiveRoutinePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<RoutineResponse>, ApiError> {
    let routine = state
        .usecase
        .archive_routine(caller.subject(), routine_id)
        .await?;
    Ok(Json(routine.into()))
}
