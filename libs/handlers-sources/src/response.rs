use axum::http::HeaderMap;
use capitainerie_common::{AgentId, CardId, ProjectId, RunId, SourceEventId, SourceId};
use capitainerie_core::domain::source::{
    IssuedSource, SourceEvent, SourceEventStatus, SourceKind, SourceListing,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::PublicBaseUrl;

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceKindDto {
    Webhook,
}

impl From<SourceKind> for SourceKindDto {
    fn from(kind: SourceKind) -> Self {
        match kind {
            SourceKind::Webhook => Self::Webhook,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceEventStatusDto {
    Received,
    CardCreated,
    Ignored,
    Failed,
}

impl From<SourceEventStatus> for SourceEventStatusDto {
    fn from(status: SourceEventStatus) -> Self {
        match status {
            SourceEventStatus::Received => Self::Received,
            SourceEventStatus::CardCreated => Self::CardCreated,
            SourceEventStatus::Ignored => Self::Ignored,
            SourceEventStatus::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SourceResponse {
    pub id: SourceId,
    pub slug: String,
    pub name: String,
    pub kind: SourceKindDto,
    pub description: String,
    pub enabled: bool,
    /// Free settings kept for the console (patterns, task template…).
    #[schema(value_type = Object)]
    pub config: Value,
    /// The agent each new card is assigned to.
    pub default_agent_id: Option<AgentId>,
    pub project_id: Option<ProjectId>,
    /// Starts the default agent's run on each new card, on behalf of the creator.
    pub auto_run: bool,
    /// Whether those runs may use the tools that write.
    pub allow_writes: bool,
    pub archived: bool,
    pub events_last_24h: i64,
    pub last_event_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SourceListing> for SourceResponse {
    fn from(listing: SourceListing) -> Self {
        let source = listing.source;
        Self {
            id: source.id,
            slug: source.slug.as_str().to_owned(),
            name: source.name,
            kind: source.kind.into(),
            description: source.description,
            enabled: source.enabled,
            config: source.config.as_value().clone(),
            default_agent_id: source.default_agent_id,
            project_id: source.project_id,
            auto_run: source.auto_run,
            allow_writes: source.allow_writes,
            archived: source.archived,
            events_last_24h: listing.events_last_24h,
            last_event_at: listing.last_event_at,
            created_by: source.created_by,
            created_at: source.created_at,
            updated_at: source.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SourceListResponse {
    pub items: Vec<SourceResponse>,
}

/// Returned once, on creation or rotation: only a hash of the token is kept.
#[derive(Debug, Serialize, ToSchema)]
pub struct IssuedSourceResponse {
    pub source: SourceResponse,
    pub token: String,
    /// Where the external tool posts its events.
    pub webhook_url: String,
}

impl IssuedSourceResponse {
    pub fn new(issued: IssuedSource, base: &PublicBaseUrl, headers: &HeaderMap) -> Self {
        let token = issued.token.expose().to_owned();
        Self {
            webhook_url: format!("{}/hooks/sources/{token}", base.resolve(headers)),
            source: issued.listing.into(),
            token,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SourceEventResponse {
    pub id: SourceEventId,
    pub source_id: SourceId,
    pub received_at: DateTime<Utc>,
    pub status: SourceEventStatusDto,
    /// What the event is about: the title of its card.
    pub summary: String,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    /// Why no card was created, or why no run was started.
    pub error: Option<String>,
}

impl From<SourceEvent> for SourceEventResponse {
    fn from(event: SourceEvent) -> Self {
        Self {
            id: event.id,
            source_id: event.source_id,
            received_at: event.received_at,
            status: event.status.into(),
            summary: event.summary,
            card_id: event.card_id,
            run_id: event.run_id,
            error: event.error,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SourceEventListResponse {
    pub items: Vec<SourceEventResponse>,
    pub total: i64,
}
