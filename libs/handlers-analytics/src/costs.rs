use axum::{
    Extension, Json,
    extract::{Query, State},
};
use axum_extra::routing::TypedPath;
use capitainerie_analytics::{Analytics, CostDimension, CostQuery, CostRow};
use capitainerie_handlers::{ApiError, Caller};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{human_only, internal, state::AnalyticsState};

pub const DEFAULT_WINDOW_DAYS: i64 = 30;

#[derive(TypedPath)]
#[typed_path("/api/v1/analytics/costs")]
pub struct CostsPath;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CostDimensionParam {
    #[default]
    Agent,
    User,
    Model,
    Day,
}

impl From<CostDimensionParam> for CostDimension {
    fn from(value: CostDimensionParam) -> Self {
        match value {
            CostDimensionParam::Agent => Self::Agent,
            CostDimensionParam::User => Self::User,
            CostDimensionParam::Model => Self::Model,
            CostDimensionParam::Day => Self::Day,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CostsQuery {
    #[serde(default)]
    pub by: CostDimensionParam,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CostRowResponse {
    pub key: String,
    pub runs: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
    pub latency_ms_p50: Option<f64>,
}

impl From<CostRow> for CostRowResponse {
    fn from(row: CostRow) -> Self {
        Self {
            key: row.key,
            runs: row.runs,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            cost_micros: row.cost_micros,
            latency_ms_p50: row.latency_ms_p50,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct CostTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CostsResponse {
    pub rows: Vec<CostRowResponse>,
    pub total: CostTotals,
}

pub fn window(
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(DateTime<Utc>, DateTime<Utc>), ApiError> {
    let to = to.unwrap_or(now);
    let from = from.unwrap_or(to - Duration::days(DEFAULT_WINDOW_DAYS));
    if from >= to {
        return Err(ApiError::BadRequest(
            "`from` must be before `to`".to_owned(),
        ));
    }
    Ok((from, to))
}

fn totals(rows: &[CostRow]) -> CostTotals {
    rows.iter()
        .fold(CostTotals::default(), |total, row| CostTotals {
            input_tokens: total.input_tokens + row.input_tokens,
            output_tokens: total.output_tokens + row.output_tokens,
            cost_micros: total.cost_micros + row.cost_micros,
        })
}

#[utoipa::path(
    get,
    description = "Reserved to human callers until the PDP gains an analytics.read action.",
    path = "/api/v1/analytics/costs",
    operation_id = "getCosts",
    tag = super::TAG,
    params(CostsQuery),
    responses(
        (status = 200, description = "Costs of the organization grouped by the requested dimension", body = CostsResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The caller is an agent, not a human", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Invalid window", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<A: Analytics + 'static>(
    _: CostsPath,
    State(state): State<AnalyticsState<A>>,
    Extension(caller): Extension<Caller>,
    Query(query): Query<CostsQuery>,
) -> Result<Json<CostsResponse>, ApiError> {
    human_only(&caller)?;
    let (from, to) = window(query.from, query.to, Utc::now())?;
    let rows = state
        .analytics
        .costs(&CostQuery {
            organization_id: caller.organization.id.0,
            from,
            to,
            by: query.by.into(),
        })
        .await
        .map_err(internal)?;
    let total = totals(&rows);
    Ok(Json(CostsResponse {
        rows: rows.into_iter().map(CostRowResponse::from).collect(),
        total,
    }))
}
