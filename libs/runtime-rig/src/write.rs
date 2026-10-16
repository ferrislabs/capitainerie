use std::time::Instant;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, PoisonError},
};

use capitainerie_common::{CoreError, RunId, TranscriptId};
use capitainerie_core::domain::agent::{LocatedAction, ToolCall, WriteReport, WriteRequest};
use rig_agent::{agent::AgentBuilder, run::PromptError};
use rig_core::{
    DynModel,
    operation::Completion,
    tool::{PortableTool, ToolExecutionError},
};
use rmcp::{RoleClient, service::Peer};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{Instrument, field, info_span};

use crate::mcp::{CREATE_CARD, McpSession, call_create_card, create_card_arguments};
use crate::usage::usage_of;

pub(crate) const AGENT_NAME: &str = "writer";
const TOOL_NAME: &str = "propose_card";

pub(crate) const SYSTEM_PROMPT: &str = "\
You are the writer of a meeting assistant. You receive a numbered list of action items found in a meeting.
For each numbered action, call the propose_card tool exactly once with:
- action_index: the number of the action in the list;
- title: a clear title in French, imperative mood, at most 80 characters;
- description: a description in French that keeps the owner and the deadline when they are given.
Do not invent actions and do not call propose_card twice for the same action.
If propose_card returns an error, read it and fix your call.
When every action has been handled, answer \"done\".";

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ProposeCardArgs {
    #[schemars(description = "The number of the action in the list you were given.")]
    action_index: u32,
    #[schemars(description = "Clear French title, imperative mood, at most 80 characters.")]
    title: String,
    #[schemars(description = "French description that keeps the owner and the deadline.")]
    description: String,
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
        let arguments = create_card_arguments(
            self.shared.run_id,
            self.shared.transcript_id,
            located,
            &args.title,
            &args.description,
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

pub(crate) fn actions_listing(actions: &[LocatedAction]) -> String {
    let mut listing = String::from("Actions to turn into cards:\n");
    for (index, located) in actions.iter().enumerate() {
        let action = &located.action;
        listing.push_str(&format!(
            "{index}. title: {} | description: {}",
            action.title, action.description
        ));
        if let Some(owner) = &action.owner {
            listing.push_str(&format!(" | owner: {owner}"));
        }
        if let Some(due) = &action.due {
            listing.push_str(&format!(" | due: {due}"));
        }
        listing.push('\n');
    }
    listing
}

pub(crate) async fn run(
    model: DynModel<Completion>,
    model_name: &str,
    mcp_url: &str,
    request: WriteRequest,
) -> Result<WriteReport, CoreError> {
    if request.actions.is_empty() {
        return Ok(WriteReport {
            tool_calls: Vec::new(),
            usage: usage_of(model_name, &Default::default()),
        });
    }
    let span = info_span!(
        "invoke_agent",
        otel.name = format!("invoke_agent {AGENT_NAME}"),
        gen_ai.operation.name = "invoke_agent",
        gen_ai.agent.name = AGENT_NAME,
        gen_ai.request.model = model_name,
    );
    async {
        let session =
            McpSession::connect(mcp_url, &request.mcp_token, request.traceparent.as_deref())
                .await?;
        let max_turns = request.actions.len() + 3;
        let listing = actions_listing(&request.actions);
        let tool = ProposeCard::new(
            session.peer(),
            request.run_id,
            request.transcript_id,
            request.actions,
        );
        let agent = AgentBuilder::new(model)
            .name(AGENT_NAME)
            .preamble(SYSTEM_PROMPT)
            .tool(tool.clone())
            .default_max_turns(max_turns)
            .build();
        let outcome = agent.prompt(listing).max_turns(max_turns).await;
        session.close().await;
        let usage = match outcome {
            Ok(response) => usage_of(model_name, &response.usage),
            Err(PromptError::MaxTurns { .. }) => {
                tracing::warn!(max_turns, "the writer ran out of turns");
                usage_of(model_name, &Default::default())
            }
            Err(error) => {
                return Err(CoreError::Internal(format!(
                    "the language model call failed: {error}"
                )));
            }
        };
        Ok(WriteReport {
            tool_calls: tool.recorded(),
            usage,
        })
    }
    .instrument(span)
    .await
}

#[cfg(test)]
mod tests {
    use capitainerie_core::domain::agent::ExtractedAction;

    use super::*;

    fn located(title: &str, owner: Option<&str>, due: Option<&str>) -> LocatedAction {
        LocatedAction {
            action: ExtractedAction {
                title: title.into(),
                description: "ctx".into(),
                quote: "q".into(),
                owner: owner.map(str::to_owned),
                due: due.map(str::to_owned),
            },
            start: 0,
            end: 1,
        }
    }

    #[test]
    fn listing_numbers_actions_and_keeps_owner_and_due() {
        let listing = actions_listing(&[
            located("A", Some("Marie"), Some("vendredi")),
            located("B", None, None),
        ]);
        assert!(listing.contains("0. title: A | description: ctx | owner: Marie | due: vendredi"));
        assert!(listing.contains("1. title: B | description: ctx\n"));
    }

    #[test]
    fn out_of_range_message_names_the_valid_range() {
        let message = out_of_range_message(7, 3);
        assert!(message.contains("7"));
        assert!(message.contains("0 to 2"));
        assert_eq!(error_type(&ToolFailure(message)), "out_of_range");
    }
}
