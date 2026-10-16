mod dataset;
mod error;
mod query;
mod record;
mod schema;
mod writer;

use std::future::Future;

use chrono::{DateTime, Utc};
use uuid::Uuid;

pub use dataset::{AnalyticsArgs, Dataset};
pub use error::AnalyticsError;
pub use query::DataFusionAnalytics;
pub use record::EventRecord;
pub use schema::schema;
pub use writer::ParquetEventWriter;

#[derive(Debug, Clone, PartialEq)]
pub struct CostRow {
    pub key: String,
    pub runs: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
    pub latency_ms_p50: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostDimension {
    Agent,
    User,
    Model,
    Day,
}

#[derive(Debug, Clone)]
pub struct CostQuery {
    pub organization_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub by: CostDimension,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRow {
    pub agent: String,
    pub runs: u64,
    pub last_seen: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micros: i64,
}

#[derive(Debug, Clone)]
pub struct AgentsQuery {
    pub organization_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

pub trait Analytics: Send + Sync {
    fn costs(
        &self,
        query: &CostQuery,
    ) -> impl Future<Output = Result<Vec<CostRow>, AnalyticsError>> + Send;

    fn agents(
        &self,
        query: &AgentsQuery,
    ) -> impl Future<Output = Result<Vec<AgentRow>, AnalyticsError>> + Send;
}
