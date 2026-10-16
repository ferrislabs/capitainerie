use capitainerie_args::{LogArgs, ObservabilityArgs};
use std::collections::HashMap;

use opentelemetry::{
    propagation::TextMapPropagator,
    trace::{TraceContextExt, TracerProvider},
};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, propagation::TraceContextPropagator, trace::SdkTracerProvider};
use thiserror::Error;
use tracing_opentelemetry::OpenTelemetrySpanExt;
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

/// Resolves on SIGINT (Ctrl-C) or SIGTERM, the signal Kubernetes sends to
/// stop a pod: both start the graceful shutdown.
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if let Err(error) = result {
                            tracing::error!(%error, "could not listen for SIGINT");
                        }
                    }
                    _ = terminate.recv() => {}
                }
            }
            Err(error) => {
                tracing::error!(%error, "could not listen for SIGTERM, only SIGINT stops the process");
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    tracing::info!("shutdown requested");
}

const TRACEPARENT: &str = "traceparent";

/// W3C `traceparent` of the current span, to carry the trace across a process
/// boundary (an event, an HTTP call). `None` when no trace is being recorded.
pub fn current_traceparent() -> Option<String> {
    let context = tracing::Span::current().context();
    if !context.span().span_context().is_valid() {
        return None;
    }
    let mut carrier = HashMap::new();
    TraceContextPropagator::new().inject_context(&context, &mut carrier);
    carrier.remove(TRACEPARENT)
}

/// Trace id of the current span, as shown by the trace backend.
pub fn current_trace_id() -> Option<String> {
    let context = tracing::Span::current().context();
    let span_context = context.span().span_context().clone();
    span_context
        .is_valid()
        .then(|| span_context.trace_id().to_string())
}

/// Makes `span` a child of the remote span `traceparent` points to.
pub fn adopt_traceparent(span: &tracing::Span, traceparent: &str) {
    let carrier = HashMap::from([(TRACEPARENT.to_owned(), traceparent.to_owned())]);
    let parent = TraceContextPropagator::new().extract(&carrier);
    if parent.span().span_context().is_valid() {
        let _ = span.set_parent(parent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_recorded_trace_there_is_nothing_to_propagate() {
        assert_eq!(current_traceparent(), None);
        assert_eq!(current_trace_id(), None);
    }

    #[test]
    fn a_malformed_traceparent_is_ignored() {
        adopt_traceparent(&tracing::Span::none(), "not-a-traceparent");
    }
}
