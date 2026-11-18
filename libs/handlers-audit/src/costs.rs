use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{AgentId, CardId, CoreError, ProjectId};
use capitainerie_core::domain::cost::{
    AgentSpend, CostReport, ModelSpend, Period, ProjectSpend, Spent, TaskSpend, UserSpend,
};
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(TypedPath)]
#[typed_path("/api/v1/analytics/costs/projects")]
pub struct ProjectCostsPath;

#[derive(TypedPath)]
#[typed_path("/api/v1/analytics/costs/agents")]
pub struct AgentCostsPath;

#[derive(TypedPath)]
#[typed_path("/api/v1/analytics/costs/tasks")]
pub struct TaskCostsPath;

#[derive(TypedPath)]
#[typed_path("/api/v1/analytics/costs/users")]
pub struct UserCostsPath;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CostsQuery {
    /// Start of the window, inclusive; 30 days before `to` by default.
    pub from: Option<DateTime<Utc>>,
    /// End of the window, exclusive; now by default.
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TaskCostsQuery {
    /// Start of the window, inclusive; 30 days before `to` by default.
    pub from: Option<DateTime<Utc>>,
    /// End of the window, exclusive; now by default.
    pub to: Option<DateTime<Utc>>,
    /// Only the tasks of this project.
    pub project_id: Option<ProjectId>,
}

/// Tokens and cost in micro-dollars.
#[derive(Debug, Serialize, ToSchema)]
pub struct SpentResponse {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
}

impl From<Spent> for SpentResponse {
    fn from(spent: Spent) -> Self {
        Self {
            input_tokens: spent.input_tokens,
            output_tokens: spent.output_tokens,
            cost_micros: spent.cost_micros,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentSpendResponse {
    /// Null for work no longer tied to an agent.
    pub agent_id: Option<AgentId>,
    pub name: String,
    pub runs: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
}

impl From<AgentSpend> for AgentSpendResponse {
    fn from(spend: AgentSpend) -> Self {
        Self {
            agent_id: spend.agent.id,
            name: spend.agent.name,
            runs: spend.runs,
            input_tokens: spend.spent.input_tokens,
            output_tokens: spend.spent.output_tokens,
            cost_micros: spend.spent.cost_micros,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectSpendResponse {
    /// Null for work outside any project.
    pub project_id: Option<ProjectId>,
    pub name: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
    pub agents: Vec<AgentSpendResponse>,
}

impl From<ProjectSpend> for ProjectSpendResponse {
    fn from(spend: ProjectSpend) -> Self {
        Self {
            project_id: spend.project_id,
            name: spend.name,
            input_tokens: spend.spent.input_tokens,
            output_tokens: spend.spent.output_tokens,
            cost_micros: spend.spent.cost_micros,
            agents: spend.agents.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TaskSpendResponse {
    pub card_id: CardId,
    pub title: String,
    pub project_id: Option<ProjectId>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
    pub agents: Vec<AgentSpendResponse>,
}

impl From<TaskSpend> for TaskSpendResponse {
    fn from(spend: TaskSpend) -> Self {
        Self {
            card_id: spend.card_id,
            title: spend.title,
            project_id: spend.project_id,
            input_tokens: spend.spent.input_tokens,
            output_tokens: spend.spent.output_tokens,
            cost_micros: spend.spent.cost_micros,
            agents: spend.agents.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModelSpendResponse {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
}

impl From<ModelSpend> for ModelSpendResponse {
    fn from(spend: ModelSpend) -> Self {
        Self {
            model: spend.model,
            input_tokens: spend.spent.input_tokens,
            output_tokens: spend.spent.output_tokens,
            cost_micros: spend.spent.cost_micros,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSpendResponse {
    pub user: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
    pub models: Vec<ModelSpendResponse>,
}

impl From<UserSpend> for UserSpendResponse {
    fn from(spend: UserSpend) -> Self {
        Self {
            user: spend.user,
            input_tokens: spend.spent.input_tokens,
            output_tokens: spend.spent.output_tokens,
            cost_micros: spend.spent.cost_micros,
            models: spend.models.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectCostsResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total: SpentResponse,
    pub rows: Vec<ProjectSpendResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentCostsResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total: SpentResponse,
    pub rows: Vec<AgentSpendResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TaskCostsResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total: SpentResponse,
    pub rows: Vec<TaskSpendResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserCostsResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total: SpentResponse,
    pub rows: Vec<UserSpendResponse>,
}

macro_rules! from_report {
    ($response:ident, $row:ty) => {
        impl From<CostReport<$row>> for $response {
            fn from(report: CostReport<$row>) -> Self {
                Self {
                    from: report.from,
                    to: report.to,
                    total: report.total.into(),
                    rows: report.rows.into_iter().map(Into::into).collect(),
                }
            }
        }
    };
}

from_report!(ProjectCostsResponse, ProjectSpend);
from_report!(AgentCostsResponse, AgentSpend);
from_report!(TaskCostsResponse, TaskSpend);
from_report!(UserCostsResponse, UserSpend);

fn period(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<Period, CoreError> {
    Period::new(from, to, Utc::now())
}

#[utoipa::path(
    get,
    path = "/api/v1/analytics/costs/projects",
    operation_id = "listProjectCosts",
    tag = super::TAG,
    params(CostsQuery),
    responses(
        (status = 200, description = "Spend per project over the window, each with its agents; work outside any project comes under a null project", body = ProjectCostsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to costs", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "`from` is not before `to`", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn projects_handler(
    _: ProjectCostsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<CostsQuery>,
) -> Result<Json<ProjectCostsResponse>, ApiError> {
    let period = period(query.from, query.to)?;
    let report = state
        .usecase
        .project_costs(caller.subject(), period)
        .await?;
    Ok(Json(report.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/analytics/costs/agents",
    operation_id = "listAgentCosts",
    tag = super::TAG,
    params(CostsQuery),
    responses(
        (status = 200, description = "Spend and runs per agent over the window", body = AgentCostsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to costs", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "`from` is not before `to`", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn agents_handler(
    _: AgentCostsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<CostsQuery>,
) -> Result<Json<AgentCostsResponse>, ApiError> {
    let period = period(query.from, query.to)?;
    let report = state.usecase.agent_costs(caller.subject(), period).await?;
    Ok(Json(report.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/analytics/costs/tasks",
    operation_id = "listTaskCosts",
    tag = super::TAG,
    params(TaskCostsQuery),
    responses(
        (status = 200, description = "Spend per task over the window, each with its agents", body = TaskCostsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to costs", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "`project_id` is not a project of the organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "`from` is not before `to`", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn tasks_handler(
    _: TaskCostsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<TaskCostsQuery>,
) -> Result<Json<TaskCostsResponse>, ApiError> {
    let period = period(query.from, query.to)?;
    let report = state
        .usecase
        .task_costs(caller.subject(), period, query.project_id)
        .await?;
    Ok(Json(report.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/analytics/costs/users",
    operation_id = "listUserCosts",
    tag = super::TAG,
    params(CostsQuery),
    responses(
        (status = 200, description = "Spend per user over the window, each with its models", body = UserCostsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no access to costs", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "`from` is not before `to`", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn users_handler(
    _: UserCostsPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<CostsQuery>,
) -> Result<Json<UserCostsResponse>, ApiError> {
    let period = period(query.from, query.to)?;
    let report = state.usecase.user_costs(caller.subject(), period).await?;
    Ok(Json(report.into()))
}
