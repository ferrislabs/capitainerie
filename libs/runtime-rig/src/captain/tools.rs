use capitainerie_core::domain::conversation::{
    ProposedCard,
    turn::{LocalTools, ToolSpec},
};
use rmcp::{RoleClient, service::Peer};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use super::driver::{ToolOutput, Tools};
use crate::{
    mcp::{RawOutcome, call_tool_raw},
    usage::truncate_chars,
};

const SUMMARY_MAX_CHARS: usize = 200;

pub(crate) trait McpCaller: Send + Sync {
    fn call(
        &self,
        tool: &'static str,
        arguments: Map<String, Value>,
    ) -> impl Future<Output = RawOutcome> + Send;
}

impl McpCaller for Peer<RoleClient> {
    async fn call(&self, tool: &'static str, arguments: Map<String, Value>) -> RawOutcome {
        call_tool_raw(self, tool, arguments).await
    }
}

/// One tool the Captain reaches through the Kanban MCP server. Adding a tool
/// of the MCP server to the Captain is one entry in [`MCP_TOOLS`].
pub(crate) struct McpTool {
    pub name: &'static str,
    pub mcp: &'static str,
    pub description: &'static str,
    pub parameters: fn() -> Value,
    pub keys: &'static [&'static str],
    pub digest: fn(&RawOutcome) -> Digest,
}

pub(crate) struct Digest {
    pub text: String,
    pub summary: String,
    pub card: Option<ProposedCard>,
}

pub(crate) const MCP_TOOLS: &[McpTool] = &[SEARCH_CARDS, PROPOSE_CARD, COMMENT_CARD];

const SEARCH_CARDS: McpTool = McpTool {
    name: "search_cards",
    mcp: "search_cards",
    description: "Search the cards already on the board by words of their title or description, and/or by column (proposed, todo, in_progress, done, rejected). Read-only. Use it before proposing a card, to avoid duplicates.",
    parameters: || {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Words to look for in the title and description."},
                "column": {"type": "string", "enum": ["proposed", "todo", "in_progress", "done", "rejected"]},
                "limit": {"type": "integer", "description": "At most this many cards, 20 by default."},
            },
            "additionalProperties": false,
        })
    },
    keys: &["query", "column", "limit"],
    digest: digest_search,
};

const PROPOSE_CARD: McpTool = McpTool {
    name: "propose_card",
    mcp: "create_card",
    description: "Propose a card on the board. It always lands in the proposed column: a member validates, rejects or hands it to Claude Code. Give a short imperative title in French and a description saying what to do and why.",
    parameters: || {
        json!({
            "type": "object",
            "properties": {
                "title": {"type": "string", "description": "Short imperative title, at most 200 characters."},
                "description": {"type": "string", "description": "What has to be done and why."},
            },
            "required": ["title", "description"],
            "additionalProperties": false,
        })
    },
    keys: &["title", "description"],
    digest: digest_proposal,
};

const COMMENT_CARD: McpTool = McpTool {
    name: "comment_card",
    mcp: "comment",
    description: "Add a comment to an existing card, for example to flag a doubt or a duplicate.",
    parameters: || {
        json!({
            "type": "object",
            "properties": {
                "card_id": {"type": "string", "description": "The id of the card."},
                "body": {"type": "string", "description": "The comment."},
            },
            "required": ["card_id", "body"],
            "additionalProperties": false,
        })
    },
    keys: &["card_id", "body"],
    digest: digest_comment,
};

fn failure(outcome: &RawOutcome) -> Digest {
    Digest {
        text: outcome.text.clone(),
        summary: truncate_chars(&outcome.text, SUMMARY_MAX_CHARS),
        card: None,
    }
}

fn digest_search(outcome: &RawOutcome) -> Digest {
    if !outcome.succeeded {
        return failure(outcome);
    }
    let found = serde_json::from_str::<Value>(&outcome.text)
        .ok()
        .and_then(|value| value.as_array().map(Vec::len));
    let summary = match found {
        Some(0) => "aucune carte".to_owned(),
        Some(1) => "1 carte trouvée".to_owned(),
        Some(count) => format!("{count} cartes trouvées"),
        None => "résultat illisible".to_owned(),
    };
    Digest {
        text: outcome.text.clone(),
        summary,
        card: None,
    }
}

fn digest_proposal(outcome: &RawOutcome) -> Digest {
    if !outcome.succeeded {
        return failure(outcome);
    }
    let card = serde_json::from_str::<Value>(&outcome.text).ok();
    let id = card
        .as_ref()
        .and_then(|card| card["id"].as_str())
        .and_then(|id| Uuid::parse_str(id).ok());
    let title = card
        .as_ref()
        .and_then(|card| card["title"].as_str())
        .map(str::to_owned);
    match (id, title) {
        (Some(card_id), Some(title)) => Digest {
            text: format!(
                "Card {card_id} \"{title}\" proposed in the proposed column. A member has to validate it: do not say it is done."
            ),
            summary: truncate_chars(&title, SUMMARY_MAX_CHARS),
            card: Some(ProposedCard { card_id, title }),
        },
        _ => Digest {
            text: outcome.text.clone(),
            summary: "carte proposée".to_owned(),
            card: None,
        },
    }
}

fn digest_comment(outcome: &RawOutcome) -> Digest {
    if !outcome.succeeded {
        return failure(outcome);
    }
    Digest {
        text: "Comment added.".to_owned(),
        summary: "commentaire ajouté".to_owned(),
        card: None,
    }
}

pub(crate) struct CaptainTools<'a, M, L> {
    pub mcp: &'a M,
    pub local: &'a L,
}

impl<M: McpCaller, L: LocalTools> Tools for CaptainTools<'_, M, L> {
    fn specs(&self) -> Vec<ToolSpec> {
        let mut specs: Vec<ToolSpec> = MCP_TOOLS
            .iter()
            .map(|tool| ToolSpec {
                name: tool.name.to_owned(),
                description: tool.description.to_owned(),
                parameters: (tool.parameters)(),
            })
            .collect();
        for spec in self.local.definitions() {
            if !specs.iter().any(|known| known.name == spec.name) {
                specs.push(spec);
            }
        }
        specs
    }

    async fn call(&self, name: &str, arguments: &Value) -> ToolOutput {
        if let Some(tool) = MCP_TOOLS.iter().find(|tool| tool.name == name) {
            let Some(object) = arguments.as_object() else {
                return refused("the arguments must be a JSON object");
            };
            let forwarded: Map<String, Value> = object
                .iter()
                .filter(|(key, _)| tool.keys.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let outcome = self.mcp.call(tool.mcp, forwarded).await;
            let digest = (tool.digest)(&outcome);
            return ToolOutput {
                succeeded: outcome.succeeded,
                text: digest.text,
                summary: digest.summary,
                card: digest.card,
            };
        }
        if self
            .local
            .definitions()
            .iter()
            .any(|spec| spec.name == name)
        {
            return match self.local.call(name, arguments).await {
                Ok(text) => ToolOutput {
                    succeeded: true,
                    summary: truncate_chars(&text, SUMMARY_MAX_CHARS),
                    text,
                    card: None,
                },
                Err(message) => refused(&message),
            };
        }
        refused(&format!("unknown tool {name}"))
    }
}

fn refused(message: &str) -> ToolOutput {
    ToolOutput {
        succeeded: false,
        text: message.to_owned(),
        summary: truncate_chars(message, SUMMARY_MAX_CHARS),
        card: None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeMcp {
        calls: Mutex<Vec<(&'static str, Map<String, Value>)>>,
        answer: Mutex<Option<RawOutcome>>,
    }

    impl McpCaller for FakeMcp {
        async fn call(&self, tool: &'static str, arguments: Map<String, Value>) -> RawOutcome {
            self.calls.lock().unwrap().push((tool, arguments));
            self.answer.lock().unwrap().take().unwrap_or(RawOutcome {
                succeeded: true,
                text: "[]".into(),
            })
        }
    }

    struct Repositories;

    impl LocalTools for Repositories {
        fn definitions(&self) -> Vec<ToolSpec> {
            vec![ToolSpec {
                name: "list_repositories".into(),
                description: "d".into(),
                parameters: json!({"type": "object"}),
            }]
        }

        async fn call(&self, _: &str, _: &Value) -> Result<String, String> {
            Ok("acme/site\nacme/api".into())
        }
    }

    fn tools<'a>(
        mcp: &'a FakeMcp,
        local: &'a Repositories,
    ) -> CaptainTools<'a, FakeMcp, Repositories> {
        CaptainTools { mcp, local }
    }

    #[test]
    fn the_captain_has_no_tool_that_moves_validates_or_executes_a_card() {
        let mcp = FakeMcp::default();
        let mut names: Vec<_> = tools(&mcp, &Repositories)
            .specs()
            .into_iter()
            .map(|spec| spec.name)
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "comment_card",
                "list_repositories",
                "propose_card",
                "search_cards"
            ]
        );
        for forbidden in ["move_card", "update_card", "execute_card", "validate_card"] {
            assert!(!names.iter().any(|name| name == forbidden));
            assert!(MCP_TOOLS.iter().all(|tool| tool.mcp != forbidden));
        }
    }

    #[tokio::test]
    async fn propose_card_maps_to_create_card_and_forwards_only_its_own_keys() {
        let mcp = FakeMcp::default();
        let id = Uuid::from_u128(9);
        *mcp.answer.lock().unwrap() = Some(RawOutcome {
            succeeded: true,
            text: json!({"id": id.to_string(), "title": "Ajouter un README", "column": "proposed"})
                .to_string(),
        });

        let output = tools(&mcp, &Repositories)
            .call(
                "propose_card",
                &json!({"title": "Ajouter un README", "description": "d", "run_id": "x", "column": "todo"}),
            )
            .await;

        assert!(output.succeeded);
        assert_eq!(
            output.card,
            Some(ProposedCard {
                card_id: id,
                title: "Ajouter un README".into()
            })
        );
        assert!(output.text.contains("A member has to validate"));
        let calls = mcp.calls.lock().unwrap();
        assert_eq!(calls[0].0, "create_card");
        let mut keys: Vec<_> = calls[0].1.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["description", "title"]);
    }

    #[tokio::test]
    async fn search_results_are_given_whole_to_the_model_and_counted_for_the_member() {
        let mcp = FakeMcp::default();
        *mcp.answer.lock().unwrap() = Some(RawOutcome {
            succeeded: true,
            text: json!([{"id": "a"}, {"id": "b"}, {"id": "c"}]).to_string(),
        });

        let output = tools(&mcp, &Repositories)
            .call("search_cards", &json!({"query": "acme"}))
            .await;

        assert_eq!(output.summary, "3 cartes trouvées");
        assert!(output.text.contains("\"b\""));
        assert_eq!(mcp.calls.lock().unwrap()[0].0, "search_cards");
    }

    #[tokio::test]
    async fn a_refusal_of_the_server_is_a_failed_call_with_its_reason() {
        let mcp = FakeMcp::default();
        *mcp.answer.lock().unwrap() = Some(RawOutcome {
            succeeded: false,
            text: "forbidden: agent may not move".into(),
        });

        let output = tools(&mcp, &Repositories)
            .call("comment_card", &json!({"card_id": "x", "body": "y"}))
            .await;

        assert!(!output.succeeded);
        assert_eq!(output.summary, "forbidden: agent may not move");
        assert!(output.card.is_none());
    }

    #[tokio::test]
    async fn local_tools_unknown_tools_and_bad_arguments_never_reach_the_server() {
        let mcp = FakeMcp::default();
        let tools = tools(&mcp, &Repositories);

        let local = tools.call("list_repositories", &json!({})).await;
        let unknown = tools.call("move_card", &json!({})).await;
        let malformed = tools.call("propose_card", &json!("title")).await;

        assert!(local.succeeded && local.text.contains("acme/api"));
        assert!(!unknown.succeeded && unknown.text.contains("unknown tool"));
        assert!(!malformed.succeeded);
        assert!(mcp.calls.lock().unwrap().is_empty());
    }
}
