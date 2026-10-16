use std::{sync::Arc, time::Duration};

use capitainerie_common::CoreError;
use capitainerie_core::domain::agent::{
    Extraction, WriteReport, WriteRequest, ports::AgentRuntime,
};
use rig_core::providers::openai::{OpenAI, OpenAIConfig};
use rig_reqwest::{ReqwestClient, reqwest};

use crate::{args::LlmArgs, error::RuntimeError, extract, write};

#[derive(Clone)]
pub struct RigAgentRuntime {
    client: OpenAI,
    model: Arc<str>,
    mcp_url: Arc<str>,
}

impl RigAgentRuntime {
    pub fn new(args: &LlmArgs) -> Result<Self, RuntimeError> {
        args.mcp_url
            .parse::<http::Uri>()
            .map_err(|error| RuntimeError::McpUrl(error.to_string()))?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(args.timeout_secs))
            .build()
            .map_err(|error| RuntimeError::HttpClient(error.to_string()))?;
        let client = OpenAIConfig::new(args.api_key.as_str())
            .with_base_url(args.base_url.as_str())
            .connect(ReqwestClient::from(http));
        Ok(Self {
            client,
            model: Arc::from(args.model.as_str()),
            mcp_url: Arc::from(args.mcp_url.as_str()),
        })
    }
}

impl AgentRuntime for RigAgentRuntime {
    async fn extract(&self, transcript: &str) -> Result<Extraction, CoreError> {
        extract::run(
            self.client.chat(&*self.model).into(),
            &self.model,
            transcript,
        )
        .await
    }

    async fn write(&self, request: WriteRequest) -> Result<WriteReport, CoreError> {
        write::run(
            self.client.chat(&*self.model).into(),
            &self.model,
            &self.mcp_url,
            request,
        )
        .await
    }
}
