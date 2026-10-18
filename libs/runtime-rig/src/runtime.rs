use std::sync::Arc;

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::{Extraction, Judgement, LocatedAction, WriteReport, WriteRequest, ports::AgentRuntime},
    provider::ModelTarget,
    similarity::SimilarCard,
};

use crate::{
    args::LlmArgs,
    client::{Clients, Resolved},
    error::RuntimeError,
    extract, judge, write,
};

#[derive(Clone)]
pub struct RigAgentRuntime {
    clients: Arc<Clients>,
    mcp_url: Arc<str>,
}

impl RigAgentRuntime {
    pub fn new(args: &LlmArgs) -> Result<Self, RuntimeError> {
        args.mcp_url
            .parse::<http::Uri>()
            .map_err(|error| RuntimeError::McpUrl(error.to_string()))?;
        Ok(Self {
            clients: Arc::new(Clients::new(args)?),
            mcp_url: Arc::from(args.mcp_url.as_str()),
        })
    }

    fn resolve(&self, target: &ModelTarget) -> Result<Resolved, CoreError> {
        self.clients.resolve(target)
    }
}

impl AgentRuntime for RigAgentRuntime {
    async fn extract(
        &self,
        target: &ModelTarget,
        transcript: &str,
    ) -> Result<Extraction, CoreError> {
        let resolved = self.resolve(target)?;
        extract::run(
            resolved.engine(),
            resolved.provider,
            &resolved.model,
            transcript,
        )
        .await
    }

    async fn write(
        &self,
        target: &ModelTarget,
        request: WriteRequest,
    ) -> Result<WriteReport, CoreError> {
        let resolved = self.resolve(target)?;
        write::run(
            resolved.engine(),
            resolved.provider,
            &resolved.model,
            &self.mcp_url,
            request,
        )
        .await
    }

    async fn judge_duplicate(
        &self,
        target: &ModelTarget,
        action: &LocatedAction,
        candidate: &SimilarCard,
    ) -> Result<Judgement, CoreError> {
        let resolved = self.resolve(target)?;
        judge::run(
            resolved.engine(),
            resolved.provider,
            &resolved.model,
            action,
            candidate,
        )
        .await
    }
}
