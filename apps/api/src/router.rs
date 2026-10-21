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
use capitainerie_integrations::HttpConnector;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
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

pub struct Adapters<T, D, R, X> {
    pub intake: IntakeAdapters<T, D>,
    pub captain: CaptainAdapters<R, X>,
    pub integrations: IntegrationAdapters,
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
        .route_layer(middleware::from_fn_with_state(state.clone(), authenticate));

    Ok(Router::new()
        .merge(protected)
        .merge(health::router())
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
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state))
}
