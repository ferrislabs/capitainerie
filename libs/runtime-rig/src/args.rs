#[derive(Clone, clap::Args)]
pub struct LlmArgs {
    #[arg(
        long = "llm-base-url",
        env = "LLM_BASE_URL",
        default_value = "http://localhost:4120/v1"
    )]
    pub base_url: String,

    #[arg(long = "llm-api-key", env = "LLM_API_KEY", default_value = "ollama")]
    pub api_key: String,

    #[arg(
        long = "llm-model",
        env = "LLM_MODEL",
        default_value = "qwen2.5:7b-instruct"
    )]
    pub model: String,

    #[arg(
        long = "llm-timeout-secs",
        env = "LLM_TIMEOUT_SECS",
        default_value_t = 300
    )]
    pub timeout_secs: u64,

    #[arg(
        long = "mcp-url",
        env = "MCP_URL",
        default_value = "http://localhost:4101/mcp"
    )]
    pub mcp_url: String,
}

impl std::fmt::Debug for LlmArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmArgs")
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .field("model", &self.model)
            .field("timeout_secs", &self.timeout_secs)
            .field("mcp_url", &self.mcp_url)
            .finish()
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct EmbeddingArgs {
    #[arg(
        id = "embedding_model",
        long = "embedding-model",
        env = "EMBEDDING_MODEL",
        default_value = "bge-m3"
    )]
    pub model: String,
}
