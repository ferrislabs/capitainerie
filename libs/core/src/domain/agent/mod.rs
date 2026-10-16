pub mod ports;

use capitainerie_common::{RunId, TranscriptId};

/// A bearer token that never shows up in `Debug` output or logs.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretToken(String);

impl SecretToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretToken(***)")
    }
}

/// An action item the extractor found in a transcript. `quote` is copied
/// verbatim from the transcript; its position is located afterwards, never
/// trusted from the model.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtractedAction {
    pub title: String,
    pub description: String,
    pub quote: String,
    pub owner: Option<String>,
    pub due: Option<String>,
}

/// An extracted action anchored in the transcript.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocatedAction {
    pub action: ExtractedAction,
    /// Character offsets `[start, end)` of `action.quote` in the transcript body.
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extraction {
    pub actions: Vec<ExtractedAction>,
    pub usage: Usage,
}

#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub run_id: RunId,
    pub transcript_id: TranscriptId,
    pub actions: Vec<LocatedAction>,
    /// Delegated token addressed to the Kanban MCP (`sub` = user, `act` = agent).
    pub mcp_token: SecretToken,
    /// W3C `traceparent` to forward to the MCP server.
    pub traceparent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub succeeded: bool,
    pub latency_ms: u64,
    /// What the tool returned, cut to a few hundred characters: a card id and
    /// title, or the error the agent saw.
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}
