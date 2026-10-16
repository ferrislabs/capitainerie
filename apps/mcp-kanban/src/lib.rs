pub mod auth;
pub mod config;
pub mod error;
pub mod tools;

use std::sync::Arc;

use axum::{Json, Router, http::StatusCode, middleware, routing::get};
use capitainerie_auth::TokenVerifier;
use capitainerie_core::CapitainerieUseCase;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use serde_json::Value;

use crate::{
    auth::{AuthState, authenticate, protected_resource},
    tools::KanbanTools,
};

const MAX_REQUEST_BYTES: usize = 1024 * 1024;

pub fn router<V: TokenVerifier + 'static>(
    usecase: CapitainerieUseCase,
    verifier: V,
    public_url: String,
) -> Router {
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_max_request_body_bytes(MAX_REQUEST_BYTES)
        .disable_allowed_hosts();
    let tools_usecase = usecase.clone();
    let service = StreamableHttpService::new(
        move || Ok(KanbanTools::new(tools_usecase.clone())),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    let metadata = protected_resource(&public_url);
    let state = Arc::new(AuthState {
        usecase,
        verifier,
        public_url,
    });
    Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(state, authenticate::<V>))
        .route("/health/live", get(|| async { StatusCode::NO_CONTENT }))
        .route(
            "/.well-known/oauth-protected-resource",
            get(move || {
                let metadata: Value = metadata.clone();
                async move { Json(metadata) }
            }),
        )
}
