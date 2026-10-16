use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_analytics::{AgentRow, AgentsQuery, Analytics};
use capitainerie_handlers::{ApiError, Caller};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{costs::DEFAULT_WINDOW_DAYS, human_only, internal, state::AnalyticsState};

#[derive(TypedPath)]
#[typed_path("/api/v1/agents")]
pub struct AgentsPath;

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentResponse {
    pub agent: String,
    pub runs: u64,
    pub last_seen: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
}

impl From<AgentRow> for AgentResponse {
    fn from(row: AgentRow) -> Self {
        Self {
            agent: row.agent,
            runs: row.runs,
            last_seen: row.last_seen,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            cost_micros: row.cost_micros,
        }
    }
}

#[utoipa::path(
    get,
    description = "Reserved to human callers until the PDP gains an analytics.read action.",
    path = "/api/v1/agents",
    operation_id = "listAgents",
    tag = super::TAG,
    responses(
        (status = 200, description = "The agents that ran steps, with their activity over the last 30 days", body = Vec<AgentResponse>),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent, not a human", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: Analytics + 'static>(
    _: AgentsPath,
    State(state): State<AnalyticsState<A>>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<Vec<AgentResponse>>, ApiError> {
    human_only(&caller)?;
    let to = Utc::now();
    let rows = state
        .analytics
        .agents(&AgentsQuery {
            organization_id: caller.organization.id.0,
            from: to - Duration::days(DEFAULT_WINDOW_DAYS),
            to,
        })
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(AgentResponse::from).collect()))
}
