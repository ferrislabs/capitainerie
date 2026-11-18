use anyhow::Context;
use axum::{Json, Router, http::HeaderValue, middleware, routing::get};
use capitainerie_analytics::Analytics;
use capitainerie_core::domain::{
    conversation::turn::CaptainRuntime, delegation::ports::TokenExchanger,
    document::ports::DocumentReaderFactory, provider::ports::ModelCatalog,
    transcription::ports::TranscriberFactory,
};
use capitainerie_handlers::{AppState, auth::authenticate, health};
use capitainerie_handlers_conversations::ConversationsState;
use capitainerie_handlers_integrations::GithubIntegration;
use capitainerie_handlers_realtime::Gateway;
use capitainerie_integrations::{HttpConnector, McpHttp};
use tower_http::{
    cors::{AllowHeaders, AllowMethods, CorsLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::openapi::ApiDoc;

pub struct CaptainAdapters<R, X> {
    pub runtime: R,
    pub exchanger: Option<X>,
}

pub struct IntakeAdapters<T, D> {
    pub transcribers: T,
    pub document_readers: D,
}

pub struct IntegrationAdapters {
    pub connector: HttpConnector,
}

pub struct McpAdapters {
    pub http: McpHttp,
    pub console_url: Option<String>,
}

pub struct SourceAdapters {
    /// The API's public URL, which the webhook URLs start with.
    pub public_api_url: Option<String>,
}

pub struct Adapters<T, D, R, X> {
    pub intake: IntakeAdapters<T, D>,
    pub captain: CaptainAdapters<R, X>,
    pub integrations: IntegrationAdapters,
    pub mcp: McpAdapters,
    pub sources: SourceAdapters,
}

pub fn router<
    A: Analytics + 'static,
    C: ModelCatalog + 'static,
    T: TranscriberFactory + 'static,
    D: DocumentReaderFactory + 'static,
    R: CaptainRuntime + 'static,
    X: TokenExchanger + 'static,
>(
    state: AppState,
    realtime: Gateway<AppState>,
    analytics: A,
    catalog: C,
    adapters: Adapters<T, D, R, X>,
    github: GithubIntegration,
    allowed_origins: &[String],
) -> anyhow::Result<Router> {
    let origins = allowed_origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin)
                .with_context(|| format!("invalid allowed origin {origin}"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let protected = Router::new()
        .merge(capitainerie_handlers_identity::router())
        .merge(capitainerie_handlers_runs::router())
        .merge(capitainerie_handlers_cards::router(github.apps.clone()))
        .merge(capitainerie_handlers_clients::router())
        .merge(capitainerie_handlers_team::router())
        .merge(capitainerie_handlers_skills::router())
        .merge(capitainerie_handlers_approvals::router())
        .merge(capitainerie_handlers_sources::router(
            adapters.sources.public_api_url.clone(),
        ))
        .merge(capitainerie_handlers_routines::router(
            adapters.sources.public_api_url,
        ))
        .merge(capitainerie_handlers_analytics::router(analytics))
        .merge(capitainerie_handlers_providers::router(
            state.usecase.clone(),
            catalog,
        ))
        .merge(capitainerie_handlers_transcriptions::router(
            state.usecase.clone(),
            adapters.intake.transcribers,
            adapters.intake.document_readers,
        ))
        .merge(capitainerie_handlers_conversations::router(
            ConversationsState::new(
                state.usecase.clone(),
                adapters.captain.runtime,
                github.apps.clone(),
                adapters.captain.exchanger,
            ),
        ))
        .merge(capitainerie_handlers_integrations::router(
            state.usecase.clone(),
            github,
            adapters.integrations.connector,
        ))
        .merge(capitainerie_handlers_mcp::router(
            state.usecase.clone(),
            adapters.mcp.http,
            adapters.mcp.console_url,
        ))
        .route_layer(middleware::from_fn_with_state(state.clone(), authenticate));

    Ok(Router::new()
        .merge(protected)
        .merge(health::router())
        .merge(capitainerie_handlers_sources::hooks_router())
        .merge(capitainerie_handlers_routines::hooks_router())
        .merge(capitainerie_handlers_realtime::router(realtime))
        .route(
            "/api-docs/openapi.json",
            get(|| async { Json(ApiDoc::openapi()) }),
        )
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(AllowMethods::mirror_request())
                .allow_headers(AllowHeaders::mirror_request()),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(redacted_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state))
}

const SOURCE_HOOK_PREFIX: &str = "/hooks/sources/";

fn redacted_path(path: &str) -> &str {
    if path.starts_with(SOURCE_HOOK_PREFIX) {
        "/hooks/sources/[redacted]"
    } else if path.starts_with(capitainerie_handlers_routines::HOOK_PREFIX) {
        "/hooks/routines/[redacted]"
    } else {
        path
    }
}

fn redacted_span<B>(request: &axum::http::Request<B>) -> tracing::Span {
    tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %redacted_path(request.uri().path()),
        version = ?request.version(),
    )
}

#[cfg(test)]
mod redaction_tests {
    use super::redacted_path;

    #[test]
    fn hook_tokens_are_not_logged() {
        assert_eq!(
            redacted_path("/hooks/sources/src_secret"),
            "/hooks/sources/[redacted]"
        );
        assert_eq!(
            redacted_path("/hooks/routines/rtn_secret"),
            "/hooks/routines/[redacted]"
        );
        assert_eq!(redacted_path("/api/v1/cards"), "/api/v1/cards");
    }
}
