#[derive(Debug, thiserror::Error)]
pub enum AnalyticsError {
    #[error("analytics storage {0} is not supported, only local directories are")]
    UnsupportedUri(String),
    #[error("analytics io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("parquet write failed: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),
    #[error("arrow failed: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),
    #[error("analytics query failed: {0}")]
    Query(#[from] datafusion::error::DataFusionError),
    #[error("event payload is malformed: {0}")]
    Payload(#[from] serde_json::Error),
    #[error("analytics result is malformed: {0}")]
    Result(String),
}
