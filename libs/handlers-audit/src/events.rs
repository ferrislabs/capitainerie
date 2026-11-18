use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{AgentId, CardId};
use capitainerie_core::domain::audit::{ActorKind, AuditEntry, AuditFilter, Severity};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(TypedPath)]
#[typed_path("/api/v1/audit/events")]
pub struct AuditEventsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AuditEventsQuery {
    /// `runs`, `cards`, `approvals`, `sources` or `routines`.
    pub topic: Option<String>,
    /// The event kind, such as `card.moved`.
    #[serde(rename = "type")]
    #[param(rename = "type")]
    pub kind: Option<String>,
    /// FerrisKey subject of a member: what they did, themselves or through an agent.
    pub actor: Option<String>,
    pub agent_id: Option<AgentId>,
    pub card_id: Option<CardId>,
    /// Inclusive.
    pub from: Option<DateTime<Utc>>,
    /// Exclusive.
    pub to: Option<DateTime<Utc>>,
    /// 1 to 200, 50 by default.
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page.
    pub cursor: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActorKindDto {
    Member,
    Agent,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SeverityDto {
    Critical,
    Warning,
    Info,
    Success,
    Created,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditActorResponse {
    pub kind: ActorKindDto,
    /// A member's subject, an agent's id (its client id when it is not a crew agent); null for the system.
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventResponse {
    pub id: Uuid,
    pub topic: String,
    #[serde(rename = "type")]
    #[schema(rename = "type")]
    pub kind: String,
    pub occurred_at: DateTime<Utc>,
    pub actor: AuditActorResponse,
    /// What the actor did, in French, to print after their name.
    pub summary: String,
    pub severity: SeverityDto,
    pub card_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    /// The event as emitted.
    #[schema(value_type = Object)]
    pub payload: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventListResponse {
    /// Newest first.
    pub items: Vec<AuditEventResponse>,
    /// Pass it as `cursor` for the next page; null on the last one.
    pub next_cursor: Option<Uuid>,
}

fn id_in(payload: &Value, field: &str) -> Option<Uuid> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

impl From<AuditEntry> for AuditEventResponse {
    fn from(entry: AuditEntry) -> Self {
        let AuditEntry {
            event,
            actor,
            summary,
            severity,
        } = entry;
        Self {
            id: event.id,
            card_id: id_in(&event.payload, "card_id"),
            run_id: id_in(&event.payload, "run_id"),
            topic: event.topic,
            kind: event.kind,
            occurred_at: event.occurred_at,
            actor: AuditActorResponse {
                kind: match actor.kind {
                    ActorKind::Member => ActorKindDto::Member,
                    ActorKind::Agent => ActorKindDto::Agent,
                    ActorKind::System => ActorKindDto::System,
                },
                id: actor.id,
                name: actor.name,
            },
            summary,
            severity: match severity {
                Severity::Critical => SeverityDto::Critical,
                Severity::Warning => SeverityDto::Warning,
                Severity::Info => SeverityDto::Info,
                Severity::Success => SeverityDto::Success,
                Severity::Created => SeverityDto::Created,
            },
            payload: event.payload,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/events",
    operation_id = "listAuditEvents",
    tag = super::TAG,
    params(AuditEventsQuery),
    responses(
        (status = 200, description = "The organization's events, newest first, with the actor's name and a summary", body = AuditEventListResponse),
        (status = 400, description = "A malformed parameter", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to the audit log", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "A limit out of bounds, or `from` not before `to`", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: AuditEventsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<AuditEventsQuery>,
) -> Result<Json<AuditEventListResponse>, ApiError> {
    let filter = AuditFilter {
        limit: AuditFilter::page(query.limit)?,
        topic: query.topic,
        kind: query.kind,
        actor: query.actor,
        agent_id: query.agent_id,
        card_id: query.card_id,
        from: query.from,
        to: query.to,
        before: query.cursor,
    };
    let page = state
        .usecase
        .list_audit_events(caller.subject(), filter)
        .await?;
    Ok(Json(AuditEventListResponse {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
