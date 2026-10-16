#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("cannot build the HTTP client for the language model: {0}")]
    HttpClient(String),
    #[error("invalid MCP url: {0}")]
    McpUrl(String),
}
