use std::collections::HashMap;

use capitainerie_common::{CardId, CoreError, RunId, TranscriptId};
use capitainerie_core::domain::agent::{LocatedAction, SecretToken};
use http::{HeaderName, HeaderValue};
use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParams, ClientConfig},
    service::{Peer, RunningService},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::{Map, Value, json};

use crate::usage::truncate_chars;

pub(crate) const CREATE_CARD: &str = "create_card";
pub(crate) const COMMENT: &str = "comment";
pub(crate) const SUMMARY_MAX_CHARS: usize = 300;

pub(crate) struct McpSession {
    service: RunningService<RoleClient, ClientConfig>,
}

impl McpSession {
    pub(crate) async fn connect(
        url: &str,
        token: &SecretToken,
        traceparent: Option<&str>,
    ) -> Result<Self, CoreError> {
        let mut headers = HashMap::new();
        if let Some(traceparent) = traceparent {
            let value = HeaderValue::from_str(traceparent)
                .map_err(|_| CoreError::Invalid("the traceparent is not a valid header".into()))?;
            headers.insert(HeaderName::from_static("traceparent"), value);
        }
        let config = StreamableHttpClientTransportConfig::with_uri(url)
            .auth_header(token.expose())
            .custom_headers(headers);
        let transport = StreamableHttpClientTransport::from_config(config);
        let service = ClientConfig::default()
            .serve(transport)
            .await
            .map_err(|error| {
                CoreError::Internal(format!("cannot reach the Kanban MCP server: {error}"))
            })?;
        Ok(Self { service })
    }

    pub(crate) fn peer(&self) -> Peer<RoleClient> {
        self.service.peer().clone()
    }

    pub(crate) async fn close(self) {
        let _ = self.service.cancel().await;
    }
}

pub(crate) fn create_card_arguments(
    run_id: RunId,
    transcript_id: TranscriptId,
    located: &LocatedAction,
    title: &str,
    description: &str,
) -> Map<String, Value> {
    let mut arguments = Map::new();
    arguments.insert("title".into(), json!(title));
    arguments.insert("description".into(), json!(description));
    arguments.insert("run_id".into(), json!(run_id.0.to_string()));
    arguments.insert(
        "provenance".into(),
        json!({
            "transcript_id": transcript_id.0.to_string(),
            "start": located.start,
            "end": located.end,
            "quote": located.action.quote,
        }),
    );
    arguments
}

pub(crate) fn comment_arguments(card_id: CardId, run_id: RunId, body: &str) -> Map<String, Value> {
    let mut arguments = Map::new();
    arguments.insert("card_id".into(), json!(card_id.0.to_string()));
    arguments.insert("body".into(), json!(body));
    arguments.insert("run_id".into(), json!(run_id.0.to_string()));
    arguments
}

pub(crate) struct CardOutcome {
    pub succeeded: bool,
    pub summary: String,
}

pub(crate) async fn call_create_card(
    peer: &Peer<RoleClient>,
    arguments: Map<String, Value>,
) -> CardOutcome {
    call_tool(peer, CREATE_CARD, arguments).await
}

pub(crate) async fn call_tool(
    peer: &Peer<RoleClient>,
    tool: &'static str,
    arguments: Map<String, Value>,
) -> CardOutcome {
    let request = CallToolRequestParams::new(tool).with_arguments(arguments);
    match peer.call_tool(request).await {
        Ok(result) => match serde_json::to_value(&result) {
            Ok(value) => outcome_of(&value),
            Err(error) => CardOutcome {
                succeeded: false,
                summary: truncate_chars(
                    &format!("unreadable MCP result: {error}"),
                    SUMMARY_MAX_CHARS,
                ),
            },
        },
        Err(error) => CardOutcome {
            succeeded: false,
            summary: truncate_chars(&error.to_string(), SUMMARY_MAX_CHARS),
        },
    }
}

pub(crate) fn outcome_of(result: &Value) -> CardOutcome {
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    let failed = result["isError"].as_bool().unwrap_or(false);
    if failed {
        return CardOutcome {
            succeeded: false,
            summary: truncate_chars(text, SUMMARY_MAX_CHARS),
        };
    }
    let summary = match serde_json::from_str::<Value>(text) {
        Ok(card) => match (card["id"].as_str(), card["title"].as_str()) {
            (Some(id), Some(title)) => format!("card {id}: {title}"),
            _ => text.to_owned(),
        },
        Err(_) => text.to_owned(),
    };
    CardOutcome {
        succeeded: true,
        summary: truncate_chars(&summary, SUMMARY_MAX_CHARS),
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_core::domain::agent::ExtractedAction;
    use uuid::Uuid;

    use super::*;

    fn located() -> LocatedAction {
        LocatedAction {
            action: ExtractedAction {
                title: "Relancer Acme".into(),
                description: "d".into(),
                quote: "relancer Acme".into(),
                owner: None,
                due: None,
            },
            start: 4,
            end: 17,
        }
    }

    #[test]
    fn arguments_carry_provenance_and_run() {
        let run = RunId(Uuid::from_u128(1));
        let transcript = TranscriptId(Uuid::from_u128(2));
        let arguments =
            create_card_arguments(run, transcript, &located(), "Relancer Acme", "Avant lundi");
        assert_eq!(arguments["title"], "Relancer Acme");
        assert_eq!(arguments["description"], "Avant lundi");
        assert_eq!(arguments["run_id"], run.0.to_string());
        assert_eq!(
            arguments["provenance"],
            json!({
                "transcript_id": transcript.0.to_string(),
                "start": 4,
                "end": 17,
                "quote": "relancer Acme",
            })
        );
        assert_eq!(arguments.len(), 4);
    }

    #[test]
    fn comment_arguments_name_the_card_and_the_run() {
        let card = CardId(Uuid::from_u128(3));
        let run = RunId(Uuid::from_u128(1));
        let arguments = comment_arguments(card, run, "salut");
        assert_eq!(arguments["card_id"], card.0.to_string());
        assert_eq!(arguments["run_id"], run.0.to_string());
        assert_eq!(arguments["body"], "salut");
        assert_eq!(arguments.len(), 3);
    }

    #[test]
    fn success_summary_names_the_card() {
        let result =
            json!({"content": [{"type": "text", "text": "{\"id\":\"abc\",\"title\":\"T\"}"}]});
        let outcome = outcome_of(&result);
        assert!(outcome.succeeded);
        assert_eq!(outcome.summary, "card abc: T");
    }

    #[test]
    fn failure_summary_is_the_mcp_text_cut_at_300_chars() {
        let text = "é".repeat(400);
        let result = json!({"isError": true, "content": [{"type": "text", "text": text}]});
        let outcome = outcome_of(&result);
        assert!(!outcome.succeeded);
        assert_eq!(outcome.summary.chars().count(), SUMMARY_MAX_CHARS);
    }
}
