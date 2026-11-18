use std::sync::Arc;

use capitainerie_core::{CapitainerieUseCase, domain::mcp::callback_url};
use capitainerie_handlers::ApiError;

pub struct McpState<A> {
    pub usecase: CapitainerieUseCase,
    pub adapter: Arc<A>,
    console_url: Option<Arc<str>>,
}

impl<A> McpState<A> {
    pub fn new(usecase: CapitainerieUseCase, adapter: A, console_url: Option<String>) -> Self {
        Self {
            usecase,
            adapter: Arc::new(adapter),
            console_url: console_url.map(Arc::from),
        }
    }

    pub(crate) fn require_secrets(&self) -> Result<(), ApiError> {
        if self.usecase.secrets_configured() {
            Ok(())
        } else {
            Err(ApiError::NotConfigured(
                "SECRETS_KEY is unset: MCP connections cannot be stored".into(),
            ))
        }
    }

    pub(crate) fn redirect_uri(&self) -> Result<String, ApiError> {
        self.console_url
            .as_deref()
            .map(callback_url)
            .ok_or_else(|| {
                ApiError::NotConfigured("CONSOLE_URL is unset: no OAuth callback address".into())
            })
    }
}

impl<A> Clone for McpState<A> {
    fn clone(&self) -> Self {
        Self {
            usecase: self.usecase.clone(),
            adapter: Arc::clone(&self.adapter),
            console_url: self.console_url.clone(),
        }
    }
}
