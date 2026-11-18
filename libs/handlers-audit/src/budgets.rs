use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::{AgentId, CoreError};
use capitainerie_core::domain::budget::{BudgetOverview, BudgetSettings, BudgetStatus, Limit};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(TypedPath)]
#[typed_path("/api/v1/budgets")]
pub struct BudgetsPath;

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BudgetScopeDto {
    Organization,
    Agent,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BudgetResponse {
    pub scope: BudgetScopeDto,
    /// Null for the organization's budget.
    pub agent_id: Option<AgentId>,
    pub agent_name: Option<String>,
    /// In US cents.
    pub monthly_limit_cents: i64,
    pub alert_threshold_percent: i16,
    /// Spent this month, in micro-dollars.
    pub spent_micros: i64,
    /// Share of the limit spent, rounded down; above 100 once exceeded.
    pub percent: i64,
    /// The spend reached the alert threshold.
    pub alert: bool,
    pub exceeded: bool,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BudgetsResponse {
    /// The current month (UTC), `[month_start, month_end)`.
    pub month_start: DateTime<Utc>,
    pub month_end: DateTime<Utc>,
    /// Spent by the organization this month, budgeted or not, in micro-dollars.
    pub spent_micros: i64,
    /// The organization's budget first, then the agents' by name.
    pub items: Vec<BudgetResponse>,
}

impl From<BudgetStatus> for BudgetResponse {
    fn from(status: BudgetStatus) -> Self {
        let budget = status.budget;
        Self {
            scope: match budget.agent_id {
                None => BudgetScopeDto::Organization,
                Some(_) => BudgetScopeDto::Agent,
            },
            agent_id: budget.agent_id,
            agent_name: budget.agent_name,
            monthly_limit_cents: budget.limit.monthly_limit_cents,
            alert_threshold_percent: budget.limit.alert_threshold_percent,
            spent_micros: status.spent_micros,
            percent: status.percent,
            alert: status.alert,
            exceeded: status.exceeded,
            updated_by: budget.updated_by,
            updated_at: budget.updated_at,
        }
    }
}

impl From<BudgetOverview> for BudgetsResponse {
    fn from(overview: BudgetOverview) -> Self {
        Self {
            month_start: overview.month_start,
            month_end: overview.month_end,
            spent_micros: overview.spent_micros,
            items: overview.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LimitRequest {
    /// In US cents, 1 to 100 000 000 000.
    pub monthly_limit_cents: i64,
    /// 1 to 100, 80 by default.
    pub alert_threshold_percent: Option<i16>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AgentLimitRequest {
    pub agent_id: AgentId,
    pub monthly_limit_cents: i64,
    pub alert_threshold_percent: Option<i16>,
}

/// Every budget of the organization: what is left out is removed.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveBudgetsRequest {
    /// Null removes the organization's budget.
    pub organization: Option<LimitRequest>,
    /// At most 200, one per agent.
    #[serde(default)]
    pub agents: Vec<AgentLimitRequest>,
}

impl TryFrom<SaveBudgetsRequest> for BudgetSettings {
    type Error = CoreError;

    fn try_from(request: SaveBudgetsRequest) -> Result<Self, Self::Error> {
        let organization = request
            .organization
            .map(|limit| Limit::new(limit.monthly_limit_cents, limit.alert_threshold_percent))
            .transpose()?;
        let agents = request
            .agents
            .into_iter()
            .map(|agent| {
                Limit::new(agent.monthly_limit_cents, agent.alert_threshold_percent)
                    .map(|limit| (agent.agent_id, limit))
            })
            .collect::<Result<_, _>>()?;
        BudgetSettings::new(organization, agents)
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/budgets",
    operation_id = "listBudgets",
    tag = super::TAG,
    responses(
        (status = 200, description = "The organization's monthly budgets and what was spent against them this month", body = BudgetsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to budgets", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_handler(
    _: BudgetsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<BudgetsResponse>, ApiError> {
    let overview = state.usecase.budgets(caller.subject()).await?;
    Ok(Json(overview.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/budgets",
    operation_id = "saveBudgets",
    tag = super::TAG,
    request_body = SaveBudgetsRequest,
    responses(
        (status = 200, description = "The saved budgets, with this month's spend", body = BudgetsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Only an organization admin manages budgets; agents never do", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "A limit out of bounds, an agent named twice or not in the organization", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn put_handler(
    _: BudgetsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<SaveBudgetsRequest>,
) -> Result<Json<BudgetsResponse>, ApiError> {
    let settings = BudgetSettings::try_from(request)?;
    let overview = state
        .usecase
        .save_budgets(caller.subject(), settings)
        .await?;
    Ok(Json(overview.into()))
}
