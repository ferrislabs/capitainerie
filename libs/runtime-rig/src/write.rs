use std::time::Instant;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, PoisonError},
};

use capitainerie_common::{CoreError, RunId, TranscriptId};
use capitainerie_core::domain::agent::{
    DuplicateAction, ExtractedAction, LocatedAction, ToolCall, Usage, WriteReport, WriteRequest,
};
use rig_agent::{agent::AgentBuilder, run::PromptError};
use rig_core::{
    DynModel,
    operation::Completion,
    tool::{PortableTool, ToolExecutionError},
};
use rmcp::{RoleClient, service::Peer};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{Instrument, field, info_span};

use crate::anthropic::{Api, Request};
use crate::client::Engine;
use crate::mcp::{
    COMMENT, CREATE_CARD, CardExtras, McpSession, call_create_card, call_tool, comment_arguments,
    create_card_arguments,
};
use crate::usage::{truncate_chars, usage_of};

pub(crate) const AGENT_NAME: &str = "writer";
const TOOL_NAME: &str = "propose_card";
const MAX_TOKENS: u32 = 4096;

pub(crate) const SYSTEM_PROMPT: &str = "\
You are the writer of a meeting assistant. You receive a numbered list of action items found in a meeting.
For each numbered action, call the propose_card tool exactly once with:
- action_index: the number of the action in the list;
- title: a clear title in French, imperative mood, at most 80 characters;
- description: a short, clean description in French. Do not mention or repeat the owner, the deadline or any label such as \"owner:\" or \"due:\": they are added automatically;
- acceptance_criteria: 2 to 5 short French sentences, each a verifiable condition that must be true when the work is done (omit when the action is too vague to say);
- priority: low, normal, high or urgent, only from what was said (urgency, blocking, customer impact); omit for normal;
- estimate: xs, s, m, l or xl, the rough size of the work; omit when you cannot tell;
- due_date: YYYY-MM-DD, only when the action line gives a stated deadline. Turn it into a calendar date using the meeting date; if the deadline is vague (\"bientôt\", \"un jour\") or no meeting date is given for a relative one, omit it. Never invent a date.
Do not invent actions and do not call propose_card twice for the same action.
If propose_card returns an error, read it and fix your call.
When every action has been handled, answer \"done\".";

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ProposeCardArgs {
    #[schemars(description = "The number of the action in the list you were given.")]
    action_index: u32,
    #[schemars(description = "Clear French title, imperative mood, at most 80 characters.")]
    title: String,
    #[schemars(description = "Clean French description. Never repeat the owner or the deadline.")]
    description: String,
    #[serde(default)]
    #[schemars(
        description = "2 to 5 short French sentences, each a verifiable condition for the work to be done. Omit when too vague."
    )]
    acceptance_criteria: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(description = "low, normal, high or urgent. Omit for normal.")]
    priority: Option<String>,
    #[serde(default)]
    #[schemars(description = "xs, s, m, l or xl. Omit when unknown.")]
    estimate: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "YYYY-MM-DD, only when the action has a stated deadline. Never invent one."
    )]
    due_date: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(crate) struct ToolFailure(String);

struct Shared {
    peer: Peer<RoleClient>,
    run_id: RunId,
    transcript_id: TranscriptId,
    actions: Vec<LocatedAction>,
    calls: Mutex<Vec<ToolCall>>,
    /// Indexes already turned into a card, or being: a model that calls the
    /// tool twice for one action must not create two cards.
    proposed: Mutex<HashSet<usize>>,
}

#[derive(Clone)]
pub(crate) struct ProposeCard {
    shared: Arc<Shared>,
}

impl ProposeCard {
    fn new(
        peer: Peer<RoleClient>,
        run_id: RunId,
        transcript_id: TranscriptId,
        actions: Vec<LocatedAction>,
    ) -> Self {
        Self {
            shared: Arc::new(Shared {
                peer,
                run_id,
                transcript_id,
                actions,
                calls: Mutex::new(Vec::new()),
                proposed: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn record(&self, call: ToolCall) {
        self.shared
            .calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }

    fn recorded(&self) -> Vec<ToolCall> {
        self.shared
            .calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn reserve(&self, index: usize) -> bool {
        self.shared
            .proposed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(index)
    }

    fn release(&self, index: usize) {
        self.shared
            .proposed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&index);
    }

    async fn propose(&self, args: ProposeCardArgs) -> Result<String, ToolFailure> {
        let Some((index, located)) = usize::try_from(args.action_index)
            .ok()
            .and_then(|index| self.shared.actions.get(index).map(|action| (index, action)))
        else {
            return Err(ToolFailure(out_of_range_message(
                args.action_index,
                self.shared.actions.len(),
            )));
        };
        if !self.reserve(index) {
            return Err(ToolFailure(already_proposed_message(args.action_index)));
        }
        let criteria = args.acceptance_criteria.unwrap_or_default();
        let due_date = located.action.due.as_ref().and(args.due_date.as_deref());
        let arguments = create_card_arguments(
            self.shared.run_id,
            self.shared.transcript_id,
            located,
            &args.title,
            &card_description(&args.description, &located.action),
            &CardExtras {
                acceptance_criteria: &criteria,
                priority: args.priority.as_deref(),
                estimate: args.estimate.as_deref(),
                due_date,
            },
        );
        let started = Instant::now();
        let outcome = call_create_card(&self.shared.peer, arguments).await;
        self.record(ToolCall {
            tool: CREATE_CARD.to_owned(),
            succeeded: outcome.succeeded,
            latency_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            summary: outcome.summary.clone(),
        });
        if outcome.succeeded {
            Ok(outcome.summary)
        } else {
            self.release(index);
            Err(ToolFailure(outcome.summary))
        }
    }
}

impl PortableTool for ProposeCard {
    const NAME: &'static str = TOOL_NAME;
    type Args = ProposeCardArgs;
    type Output = String;
    type Error = ToolFailure;

    fn description(&self) -> String {
        "Propose one Kanban card for one numbered action. Call it exactly once per action."
            .to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ProposeCardArgs)).unwrap_or_default()
    }

    fn map_error(&self, error: ToolFailure) -> ToolExecutionError {
        ToolExecutionError::other(error.0)
    }

    async fn call(&self, args: ProposeCardArgs) -> Result<String, ToolFailure> {
        let span = info_span!(
            "execute_tool",
            otel.name = format!("execute_tool {TOOL_NAME}"),
            gen_ai.operation.name = "execute_tool",
            gen_ai.tool.name = TOOL_NAME,
            error.r#type = field::Empty,
        );
        let result = self.propose(args).instrument(span.clone()).await;
        if let Err(failure) = &result {
            span.record("error.type", error_type(failure));
        }
        result
    }
}

fn error_type(failure: &ToolFailure) -> &'static str {
    if failure.0.starts_with(OUT_OF_RANGE_PREFIX) {
        "out_of_range"
    } else if failure.0.starts_with(ALREADY_PROPOSED_PREFIX) {
        "already_proposed"
    } else {
        "mcp_error"
    }
}

const OUT_OF_RANGE_PREFIX: &str = "action_index ";
const ALREADY_PROPOSED_PREFIX: &str = "action ";

pub(crate) fn already_proposed_message(index: u32) -> String {
    format!(
        "{ALREADY_PROPOSED_PREFIX}{index} already has its card: nothing was created. Move on to the next action, or answer done."
    )
}

pub(crate) fn out_of_range_message(index: u32, count: usize) -> String {
    format!(
        "{OUT_OF_RANGE_PREFIX}{index} does not exist: valid indexes are 0 to {}. Nothing was created.",
        count.saturating_sub(1)
    )
}

const METADATA_MARKERS: [&str; 8] = [
    "_owner:",
    "owner:",
    "_due:",
    "due:",
    "responsable :",
    "responsable:",
    "échéance :",
    "échéance:",
];

fn starts_with_ci(text: &str, marker: &str) -> bool {
    let mut chars = text.chars().flat_map(char::to_lowercase);
    marker
        .chars()
        .all(|expected| chars.next() == Some(expected))
}

fn strip_metadata_line(line: &str) -> &str {
    let mut previous_is_word = false;
    for (index, character) in line.char_indices() {
        if !previous_is_word
            && METADATA_MARKERS
                .iter()
                .any(|marker| starts_with_ci(&line[index..], marker))
        {
            return line[..index]
                .trim_end_matches(|c: char| c.is_whitespace() || c == '|' || c == '_');
        }
        previous_is_word = character.is_alphanumeric();
    }
    line
}

pub(crate) fn strip_metadata(description: &str) -> String {
    description
        .lines()
        .map(strip_metadata_line)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

pub(crate) fn card_description(description: &str, action: &ExtractedAction) -> String {
    let mut text = strip_metadata(description);
    let footer = [
        action
            .owner
            .as_deref()
            .map(|owner| format!("Responsable : {owner}")),
        action.due.as_deref().map(|due| format!("Échéance : {due}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n");
    if !footer.is_empty() {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&footer);
    }
    text
}

pub(crate) fn actions_listing(actions: &[LocatedAction], meeting_date: Option<&str>) -> String {
    let mut listing = String::new();
    if let Some(date) = meeting_date {
        listing.push_str(&format!("Meeting date: {date}\n"));
    }
    listing.push_str("Actions to turn into cards:\n");
    for (index, located) in actions.iter().enumerate() {
        let action = &located.action;
        listing.push_str(&format!(
            "{index}. title: {} | description: {}",
            action.title, action.description
        ));
        if let Some(due) = &action.due {
            listing.push_str(&format!(" | stated deadline: {due}"));
        }
        listing.push('\n');
    }
    listing
}

const QUOTE_MAX_CHARS: usize = 500;

pub(crate) fn duplicate_comment(request: &WriteRequest, duplicate: &DuplicateAction) -> String {
    format!(
        "Mentionné à nouveau dans la réunion du transcript {} : “{}”",
        request.transcript_id,
        truncate_chars(&duplicate.action.action.quote, QUOTE_MAX_CHARS)
    )
}

async fn comment_duplicates(peer: &Peer<RoleClient>, request: &WriteRequest) -> Vec<ToolCall> {
    let mut calls = Vec::with_capacity(request.duplicates.len());
    for duplicate in &request.duplicates {
        let span = info_span!(
            "execute_tool",
            otel.name = format!("execute_tool {COMMENT}"),
            gen_ai.operation.name = "execute_tool",
            gen_ai.tool.name = COMMENT,
        );
        let arguments = comment_arguments(
            duplicate.card_id,
            request.run_id,
            &duplicate_comment(request, duplicate),
        );
        let started = Instant::now();
        let outcome = call_tool(peer, COMMENT, arguments).instrument(span).await;
        calls.push(ToolCall {
            tool: COMMENT.to_owned(),
            succeeded: outcome.succeeded,
            latency_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            summary: if outcome.succeeded {
                format!("comment on card {}", duplicate.card_id)
            } else {
                outcome.summary
            },
        });
    }
    calls
}

fn tool_definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Propose one Kanban card for one numbered action. Call it exactly once per action.",
        "strict": true,
        "input_schema": {
            "type": "object",
            "properties": {
                "action_index": {
                    "type": "integer",
                    "description": "The number of the action in the list you were given.",
                },
                "title": {
                    "type": "string",
                    "description": "Clear French title, imperative mood, at most 80 characters.",
                },
                "description": {
                    "type": "string",
                    "description": "Clean French description. Never repeat the owner or the deadline.",
                },
                "acceptance_criteria": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "2 to 5 short French sentences, each a verifiable condition for the work to be done. Omit when too vague.",
                },
                "priority": {
                    "type": "string",
                    "enum": ["low", "normal", "high", "urgent"],
                    "description": "Omit for normal.",
                },
                "estimate": {
                    "type": "string",
                    "enum": ["xs", "s", "m", "l", "xl"],
                    "description": "Omit when unknown.",
                },
                "due_date": {
                    "type": "string",
                    "description": "YYYY-MM-DD, only when the action has a stated deadline. Never invent one.",
                },
            },
            "required": ["action_index", "title", "description"],
            "additionalProperties": false,
        },
    })
}

async fn tool_result(tool: &ProposeCard, call: &Value) -> Value {
    let name = call["name"].as_str().unwrap_or_default();
    let (content, is_error) = if name == TOOL_NAME {
        match serde_json::from_value::<ProposeCardArgs>(call["input"].clone()) {
            Ok(args) => match PortableTool::call(tool, args).await {
                Ok(summary) => (summary, false),
                Err(failure) => (failure.0, true),
            },
            Err(error) => (format!("invalid arguments: {error}"), true),
        }
    } else {
        (format!("unknown tool {name}"), true)
    };
    json!({
        "type": "tool_result",
        "tool_use_id": call["id"],
        "content": content,
        "is_error": is_error,
    })
}

async fn tool_loop(
    api: &Api,
    model_name: &str,
    tool: &ProposeCard,
    listing: String,
    max_turns: usize,
) -> Result<Usage, CoreError> {
    let tools = [tool_definition()];
    let mut messages = vec![json!({ "role": "user", "content": listing })];
    let mut usage = Usage {
        model: model_name.to_owned(),
        ..Usage::default()
    };
    for _ in 0..max_turns {
        let reply = api
            .send(&Request {
                model: model_name,
                max_tokens: MAX_TOKENS,
                system: SYSTEM_PROMPT,
                messages: &messages,
                tools: Some(&tools),
                schema: None,
            })
            .await?;
        usage.absorb(&reply.usage);
        if reply.stop_reason != "tool_use" {
            return Ok(usage);
        }
        let mut results = Vec::new();
        for call in reply.tool_uses() {
            results.push(tool_result(tool, call).await);
        }
        if results.is_empty() {
            return Ok(usage);
        }
        messages.push(json!({ "role": "assistant", "content": reply.content }));
        messages.push(json!({ "role": "user", "content": results }));
    }
    tracing::warn!(max_turns, "the writer ran out of turns");
    Ok(usage)
}

async fn rig_loop(
    model: DynModel<Completion>,
    model_name: &str,
    tool: &ProposeCard,
    listing: String,
    max_turns: usize,
) -> Result<Usage, CoreError> {
    let agent = AgentBuilder::new(model)
        .name(AGENT_NAME)
        .preamble(SYSTEM_PROMPT)
        .tool(tool.clone())
        .default_max_turns(max_turns)
        .build();
    match agent.prompt(listing).max_turns(max_turns).await {
        Ok(response) => Ok(usage_of(model_name, &response.usage)),
        Err(PromptError::MaxTurns { .. }) => {
            tracing::warn!(max_turns, "the writer ran out of turns");
            Ok(usage_of(model_name, &Default::default()))
        }
        Err(error) => Err(CoreError::Internal(format!(
            "the language model call failed: {error}"
        ))),
    }
}

pub(crate) async fn run(
    engine: Engine,
    provider: &str,
    model_name: &str,
    mcp_url: &str,
    request: WriteRequest,
) -> Result<WriteReport, CoreError> {
    if request.actions.is_empty() && request.duplicates.is_empty() {
        return Ok(WriteReport {
            tool_calls: Vec::new(),
            usage: usage_of(model_name, &Default::default()),
        });
    }
    let span = info_span!(
        "invoke_agent",
        otel.name = format!("invoke_agent {AGENT_NAME}"),
        gen_ai.operation.name = "invoke_agent",
        gen_ai.provider.name = provider,
        gen_ai.agent.name = AGENT_NAME,
        gen_ai.request.model = model_name,
    );
    async {
        let session =
            McpSession::connect(mcp_url, &request.mcp_token, request.traceparent.as_deref())
                .await?;
        let mut comments = comment_duplicates(&session.peer(), &request).await;
        if request.actions.is_empty() {
            session.close().await;
            return Ok(WriteReport {
                tool_calls: comments,
                usage: usage_of(model_name, &Default::default()),
            });
        }
        let max_turns = request.actions.len() + 3;
        let listing = actions_listing(
            &request.actions,
            request.meeting_date.map(|date| date.to_string()).as_deref(),
        );
        let tool = ProposeCard::new(
            session.peer(),
            request.run_id,
            request.transcript_id,
            request.actions,
        );
        let outcome = match engine {
            Engine::Chat(model) => rig_loop(model, model_name, &tool, listing, max_turns).await,
            Engine::Anthropic(api) => tool_loop(&api, model_name, &tool, listing, max_turns).await,
        };
        session.close().await;
        let usage = outcome?;
        comments.extend(tool.recorded());
        Ok(WriteReport {
            tool_calls: comments,
            usage,
        })
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn located(title: &str, owner: Option<&str>, due: Option<&str>) -> LocatedAction {
        LocatedAction {
            action: ExtractedAction {
                title: title.into(),
                description: "ctx".into(),
                quote: "q".into(),
                owner: owner.map(str::to_owned),
                due: due.map(str::to_owned),
                client_ref: None,
                project_ref: None,
            },
            start: 0,
            end: 1,
        }
    }

    #[test]
    fn listing_numbers_actions_without_owner_and_due() {
        let listing = actions_listing(
            &[
                located("A", Some("Marie"), Some("vendredi")),
                located("B", None, None),
            ],
            None,
        );
        assert!(listing.contains("0. title: A | description: ctx | stated deadline: vendredi\n"));
        assert!(!listing.contains("Marie"));
        assert!(listing.contains("1. title: B | description: ctx\n"));
        assert!(!listing.contains("Meeting date"));
    }

    #[test]
    fn the_listing_gives_the_meeting_date_when_known() {
        let listing = actions_listing(&[located("A", None, None)], Some("2026-10-19"));
        assert!(listing.starts_with("Meeting date: 2026-10-19\n"));
    }

    #[test]
    fn the_prompt_and_tool_ask_for_the_details_and_forbid_invented_dates() {
        for word in ["acceptance_criteria", "priority", "estimate", "due_date"] {
            assert!(SYSTEM_PROMPT.contains(word), "{word}");
            assert!(tool_definition()["input_schema"]["properties"][word].is_object());
        }
        assert!(SYSTEM_PROMPT.contains("Never invent a date"));
        let required = tool_definition()["input_schema"]["required"].clone();
        assert_eq!(required, json!(["action_index", "title", "description"]));
    }

    #[test]
    fn the_footer_carries_only_the_known_lines() {
        let both = located("A", Some("Lina"), Some("lundi")).action;
        assert_eq!(
            card_description("Envoyer l'email.", &both),
            "Envoyer l'email.\n\nResponsable : Lina\nÉchéance : lundi"
        );
        let owner = located("A", Some("Lina"), None).action;
        assert_eq!(
            card_description("Envoyer l'email.", &owner),
            "Envoyer l'email.\n\nResponsable : Lina"
        );
        let due = located("A", None, Some("lundi")).action;
        assert_eq!(
            card_description("Envoyer l'email.", &due),
            "Envoyer l'email.\n\nÉchéance : lundi"
        );
        let none = located("A", None, None).action;
        assert_eq!(
            card_description("Envoyer l'email.", &none),
            "Envoyer l'email."
        );
    }

    #[test]
    fn metadata_fragments_are_stripped_from_the_model_description() {
        for (input, expected) in [
            (
                "Rédiger l'email._owner: Lina _due: lundi",
                "Rédiger l'email.",
            ),
            ("Rédiger l'email | Owner: Sophie", "Rédiger l'email"),
            ("Rédiger l'email. owner: Sophie", "Rédiger l'email."),
            ("Rédiger l'email. Due: lundi", "Rédiger l'email."),
            (
                "Rédiger l'email.\nResponsable : Lina\nÉchéance : lundi",
                "Rédiger l'email.",
            ),
            (
                "Premier point.\nSecond | due: demain\nTroisième",
                "Premier point.\nSecond\nTroisième",
            ),
            (
                "Prévoir le budget overdue: oui",
                "Prévoir le budget overdue: oui",
            ),
            ("Relancer le fournisseur", "Relancer le fournisseur"),
        ] {
            assert_eq!(strip_metadata(input), expected, "{input}");
        }
    }

    #[test]
    fn out_of_range_message_names_the_valid_range() {
        let message = out_of_range_message(7, 3);
        assert!(message.contains("7"));
        assert!(message.contains("0 to 2"));
        assert_eq!(error_type(&ToolFailure(message)), "out_of_range");
    }
}
