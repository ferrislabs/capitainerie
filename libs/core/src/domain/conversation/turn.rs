use capitainerie_common::CoreError;
use serde_json::Value;

use super::{
    MAX_OUTPUT_TOKENS_PER_TURN, MAX_TOOL_CALLS_PER_TURN, MessageContent, ProposedCard, ToolRecord,
};
use crate::domain::{
    agent::{SecretToken, Usage},
    provider::ModelTarget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

/// A past message as the model sees it: its text only. The tool calls of
/// earlier turns are summarised in the text the Captain wrote about them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnLimits {
    pub max_tool_calls: usize,
    pub max_output_tokens: u32,
}

impl Default for TurnLimits {
    fn default() -> Self {
        Self {
            max_tool_calls: MAX_TOOL_CALLS_PER_TURN,
            max_output_tokens: MAX_OUTPUT_TOKENS_PER_TURN,
        }
    }
}

/// Everything a runtime needs for one turn. `mcp_token` is the token
/// exchanged for the Captain, valid for this turn only; the member's own token
/// never reaches the runtime.
#[derive(Debug, Clone)]
pub struct TurnRequest {
    pub target: ModelTarget,
    pub history: Vec<ChatMessage>,
    pub mcp_token: SecretToken,
    pub traceparent: Option<String>,
    pub limits: TurnLimits,
}

/// What the member's screen follows while the turn runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEvent {
    Delta(String),
    ToolStarted {
        id: String,
        name: String,
        arguments: String,
    },
    ToolFinished {
        id: String,
        name: String,
        succeeded: bool,
        latency_ms: u64,
        summary: String,
    },
    Card(ProposedCard),
}

/// Everything the turn produced so far. The caller owns it, so a turn that
/// stops half way (client gone, provider error) still leaves its partial
/// answer to keep.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnState {
    pub text: String,
    pub tool_calls: Vec<ToolRecord>,
    pub cards: Vec<ProposedCard>,
    pub usage: Usage,
    pub cost_micros: i64,
}

impl TurnState {
    pub fn content(&self) -> MessageContent {
        MessageContent {
            text: self.text.clone(),
            tool_calls: self.tool_calls.clone(),
            cards: self.cards.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEnd {
    Completed,
    Interrupted,
}

/// A tool the Captain can call, declared once and shown to the model.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub trait TurnSink: Send + Sync {
    /// `false` once the member's screen is gone: the turn must then stop.
    fn emit(&self, event: TurnEvent) -> impl Future<Output = bool> + Send;
}

/// The Captain's tools served by the API itself rather than by the MCP server.
pub trait LocalTools: Send + Sync {
    fn definitions(&self) -> Vec<ToolSpec>;

    /// The text the model reads back, or the reason the call failed.
    fn call(
        &self,
        name: &str,
        arguments: &Value,
    ) -> impl Future<Output = Result<String, String>> + Send;
}

pub trait CaptainRuntime: Send + Sync {
    fn run_turn<L: LocalTools, S: TurnSink>(
        &self,
        request: &TurnRequest,
        local: &L,
        sink: &S,
        state: &mut TurnState,
    ) -> impl Future<Output = Result<TurnEnd, CoreError>> + Send;
}

#[derive(Debug, thiserror::Error)]
pub enum TurnError {
    #[error("another turn is already running in this conversation")]
    InProgress,
    #[error("this conversation reached its token limit: open a new one")]
    Full,
    #[error("no model can serve the Captain for this organization")]
    NotConfigured,
    #[error(transparent)]
    Core(#[from] CoreError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_limits_are_the_c9_ones() {
        let limits = TurnLimits::default();
        assert_eq!(limits.max_tool_calls, 20);
        assert_eq!(limits.max_output_tokens, 16_000);
    }

    #[test]
    fn a_partial_state_becomes_message_content() {
        let state = TurnState {
            text: "Je cherche".into(),
            ..TurnState::default()
        };
        assert_eq!(state.content().text, "Je cherche");
    }
}
