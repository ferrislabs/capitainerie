use capitainerie_args::{LogArgs, ObservabilityArgs};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use thiserror::Error;
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("invalid log filter: {0}")]
    Filter(String),

    #[error("could not build the OTLP exporter: {0}")]
    Exporter(String),

    #[error("a global subscriber is already installed: {0}")]
    AlreadyInstalled(String),
}

/// Flushes and shuts the tracer provider down when dropped, so the last spans
/// of a stopping process are exported.
#[derive(Default)]
pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer_provider.take() {
            let _ = provider.shutdown();
        }
    }
}

/// Logs go to stderr; spans are also exported over OTLP/gRPC when an endpoint is set.
pub fn init(
    service_name: &'static str,
    log: &LogArgs,
    observability: &ObservabilityArgs,
) -> Result<TelemetryGuard, TelemetryError> {
    let filter =
        EnvFilter::try_new(&log.filter).map_err(|err| TelemetryError::Filter(err.to_string()))?;
    let fmt_layer = if log.json {
        fmt::layer().with_writer(std::io::stderr).json().boxed()
    } else {
        fmt::layer().with_writer(std::io::stderr).boxed()
    };

    let Some(endpoint) = observability.otlp_endpoint.as_deref() else {
        Registry::default()
            .with(fmt_layer)
            .with(filter)
            .try_init()
            .map_err(|err| TelemetryError::AlreadyInstalled(err.to_string()))?;
        return Ok(TelemetryGuard::default());
    };

    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|err| TelemetryError::Exporter(err.to_string()))?;
    let provider = SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name(service_name).build())
        .with_batch_exporter(exporter)
        .build();
    let tracer = provider.tracer(service_name);

    Registry::default()
        .with(fmt_layer)
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(filter)
        .try_init()
        .map_err(|err| TelemetryError::AlreadyInstalled(err.to_string()))?;

    tracing::info!(
        service = service_name,
        otlp_endpoint = endpoint,
        "exporting spans via OTLP/gRPC"
    );

    Ok(TelemetryGuard {
        tracer_provider: Some(provider),
    })
}
