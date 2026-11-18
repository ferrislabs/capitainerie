use axum::{
    Extension, Json, Router,
    extract::{Query, State},
};
use axum_extra::routing::{RouterExt, TypedPath};
use capitainerie_common::{CardId, InboxItemId, RunId};
use capitainerie_core::domain::inbox::{self, InboxItem, InboxKind, ReadSelection};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub const TAG: &str = "inbox";

pub fn router() -> Router<AppState> {
    Router::new()
        .typed_get(list_handler)
        .typed_post(read_handler)
}

#[derive(TypedPath)]
#[typed_path("/api/v1/inbox")]
pub struct InboxPath;

#[derive(TypedPath)]
#[typed_path("/api/v1/inbox/read")]
pub struct InboxReadPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct InboxQuery {
    #[serde(default)]
    pub unread_only: bool,
    /// 1 to 200, 50 by default.
    pub limit: Option<u32>,
    /// The `next_cursor` of the previous page.
    pub cursor: Option<InboxItemId>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InboxKindDto {
    Approval,
    Run,
    Card,
    Source,
    Routine,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InboxItemResponse {
    pub id: InboxItemId,
    pub kind: InboxKindDto,
    pub title: String,
    pub body: String,
    /// A console path, such as `/tasks/{card_id}`.
    pub link: Option<String>,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub unread: bool,
}

impl From<InboxItem> for InboxItemResponse {
    fn from(item: InboxItem) -> Self {
        Self {
            id: item.id,
            kind: match item.kind {
                InboxKind::Approval => InboxKindDto::Approval,
                InboxKind::Run => InboxKindDto::Run,
                InboxKind::Card => InboxKindDto::Card,
                InboxKind::Source => InboxKindDto::Source,
                InboxKind::Routine => InboxKindDto::Routine,
            },
            title: item.title,
            body: item.body,
            link: item.link,
            card_id: item.card_id,
            run_id: item.run_id,
            created_at: item.created_at,
            unread: item.read_at.is_none(),
            read_at: item.read_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InboxResponse {
    /// Newest first.
    pub items: Vec<InboxItemResponse>,
    /// Pass it as `cursor` for the next page; null on the last one.
    pub next_cursor: Option<InboxItemId>,
    /// Every item of the caller, whatever the filter.
    pub total: i64,
    /// Unread items of the caller, whatever the filter.
    pub unread: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MarkReadRequest {
    /// The items to mark read, at most 500; or set `all`.
    pub ids: Option<Vec<InboxItemId>>,
    #[serde(default)]
    pub all: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MarkReadResponse {
    /// Items that were unread and are now read.
    pub updated: u64,
    pub unread: i64,
}

#[utoipa::path(
    get,
    path = "/api/v1/inbox",
    operation_id = "listInbox",
    tag = TAG,
    params(InboxQuery),
    responses(
        (status = 200, description = "The caller's notifications, newest first, with their counts", body = InboxResponse),
        (status = 400, description = "A malformed parameter", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents have no inbox", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "A limit out of bounds", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_handler(
    _: InboxPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<InboxQuery>,
) -> Result<Json<InboxResponse>, ApiError> {
    let limit = inbox::page(query.limit)?;
    let page = state
        .usecase
        .list_inbox(caller.subject(), query.unread_only, query.cursor, limit)
        .await?;
    Ok(Json(InboxResponse {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
        total: page.total,
        unread: page.unread,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/inbox/read",
    operation_id = "markInboxRead",
    tag = TAG,
    request_body = MarkReadRequest,
    responses(
        (status = 200, description = "The caller's items are read; ids of other members' items change nothing", body = MarkReadResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents have no inbox", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Neither or both of `ids` and `all`, or too many ids", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn read_handler(
    _: InboxReadPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<MarkReadRequest>,
) -> Result<Json<MarkReadResponse>, ApiError> {
    let selection = ReadSelection::new(request.ids, request.all)?;
    let (updated, unread) = state
        .usecase
        .mark_inbox_read(caller.subject(), selection)
        .await?;
    Ok(Json(MarkReadResponse { updated, unread }))
}
