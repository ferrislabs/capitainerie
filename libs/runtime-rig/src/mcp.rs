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

#[derive(Default)]
pub(crate) struct CardExtras<'a> {
    pub acceptance_criteria: &'a [String],
    pub priority: Option<&'a str>,
    pub estimate: Option<&'a str>,
    pub due_date: Option<&'a str>,
}

pub(crate) fn create_card_arguments(
    run_id: RunId,
    transcript_id: TranscriptId,
    located: &LocatedAction,
    title: &str,
    description: &str,
    extras: &CardExtras<'_>,
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
    let criteria: Vec<&str> = extras
        .acceptance_criteria
        .iter()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .collect();
    if !criteria.is_empty() {
        arguments.insert("acceptance_criteria".into(), json!(criteria));
    }
    for (key, value) in [
        ("priority", extras.priority),
        ("estimate", extras.estimate),
        ("due_date", extras.due_date),
    ] {
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            arguments.insert(key.into(), json!(value));
        }
    }
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

pub(crate) const RAW_MAX_CHARS: usize = 12_000;

pub(crate) struct RawOutcome {
    pub succeeded: bool,
    pub text: String,
}

pub(crate) async fn call_tool_raw(
    peer: &Peer<RoleClient>,
    tool: &'static str,
    arguments: Map<String, Value>,
) -> RawOutcome {
    let request = CallToolRequestParams::new(tool).with_arguments(arguments);
    match peer.call_tool(request).await {
        Ok(result) => match serde_json::to_value(&result) {
            Ok(value) => RawOutcome {
                succeeded: !value["isError"].as_bool().unwrap_or(false),
                text: truncate_chars(
                    value["content"][0]["text"].as_str().unwrap_or_default(),
                    RAW_MAX_CHARS,
                ),
            },
            Err(error) => RawOutcome {
                succeeded: false,
                text: format!("unreadable MCP result: {error}"),
            },
        },
        Err(error) => RawOutcome {
            succeeded: false,
            text: truncate_chars(&error.to_string(), SUMMARY_MAX_CHARS),
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
                client_ref: None,
                project_ref: None,
            },
            start: 4,
            end: 17,
        }
    }

    #[test]
    fn arguments_carry_provenance_and_run() {
        let run = RunId(Uuid::from_u128(1));
        let transcript = TranscriptId(Uuid::from_u128(2));
        let arguments = create_card_arguments(
            run,
            transcript,
            &located(),
            "Relancer Acme",
            "Avant lundi",
            &CardExtras::default(),
        );
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
    fn create_card_arguments_pass_the_details_that_are_given() {
        let criteria = vec!["  L'email part ".to_owned(), "  ".to_owned()];
        let arguments = create_card_arguments(
            RunId(Uuid::from_u128(1)),
            TranscriptId(Uuid::from_u128(2)),
            &located(),
            "T",
            "D",
            &CardExtras {
                acceptance_criteria: &criteria,
                priority: Some("high"),
                estimate: Some("m"),
                due_date: Some("2026-10-31"),
            },
        );
        assert_eq!(arguments["acceptance_criteria"], json!(["L'email part"]));
        assert_eq!(arguments["priority"], "high");
        assert_eq!(arguments["estimate"], "m");
        assert_eq!(arguments["due_date"], "2026-10-31");
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
