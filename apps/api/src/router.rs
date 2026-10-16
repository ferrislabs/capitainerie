use anyhow::Context;
use axum::{Json, Router, http::HeaderValue, middleware, routing::get};
use capitainerie_handlers::{AppState, auth::authenticate, health};
use capitainerie_handlers_realtime::Gateway;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::openapi::ApiDoc;

pub fn router(
    state: AppState,
    realtime: Gateway<AppState>,
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
        .merge(capitainerie_handlers_cards::router())
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
