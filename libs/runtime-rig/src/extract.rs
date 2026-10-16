use capitainerie_common::CoreError;
use capitainerie_core::domain::agent::{ExtractedAction, Extraction};
use rig_agent::{
    agent::AgentBuilder, completion::StructuredOutputError, extractor::ExtractorBuilder,
};
use rig_core::{DynModel, operation::Completion};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{Instrument, info_span};

use crate::usage::usage_of;

pub(crate) const AGENT_NAME: &str = "extractor";
pub(crate) const RETRIES: usize = 2;

pub(crate) const SYSTEM_PROMPT: &str = "\
Extract every concrete action item from the meeting transcript the user sends: something that someone must do.
Rules:
- Write the answer in the language of the transcript.
- title: a short imperative sentence.
- description: the useful context, plus the owner and the deadline when the transcript states them.
- quote: the SHORTEST span copied character for character from the transcript that states the action. Never paraphrase, never correct typos or punctuation, never add or remove characters.
- owner: the person in charge when stated, otherwise omit it.
- due: the deadline when stated, otherwise omit it.
- Do not report the same action twice.
- Do not invent actions. If the transcript contains no action item, return an empty list.";

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
pub(crate) struct Actions {
    #[schemars(
        description = "Every action item found in the transcript. Empty when there is none."
    )]
    pub actions: Vec<Action>,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
pub(crate) struct Action {
    #[schemars(description = "Short imperative title of the action.")]
    pub title: String,
    #[schemars(description = "Context of the action, with owner and deadline when stated.")]
    pub description: String,
    #[schemars(
        description = "The shortest span copied character for character from the transcript that states the action."
    )]
    pub quote: String,
    #[schemars(description = "Person in charge, when stated.")]
    #[serde(default)]
    pub owner: Option<String>,
    #[schemars(description = "Deadline, when stated.")]
    #[serde(default)]
    pub due: Option<String>,
}

impl From<Action> for ExtractedAction {
    fn from(action: Action) -> Self {
        Self {
            title: action.title,
            description: action.description,
            quote: action.quote,
            owner: action.owner.filter(|owner| !owner.trim().is_empty()),
            due: action.due.filter(|due| !due.trim().is_empty()),
        }
    }
}

pub(crate) fn failure(error: StructuredOutputError) -> CoreError {
    match error {
        StructuredOutputError::Deserialization { .. } | StructuredOutputError::EmptyResponse => {
            CoreError::Invalid(format!(
                "the language model did not return valid actions after {} retries: {error}",
                RETRIES
            ))
        }
        StructuredOutputError::Prompt(error) => {
            CoreError::Internal(format!("the language model call failed: {error}"))
        }
    }
}

pub(crate) async fn run(
    model: DynModel<Completion>,
    model_name: &str,
    transcript: &str,
) -> Result<Extraction, CoreError> {
    let span = info_span!(
        "invoke_agent",
        otel.name = format!("invoke_agent {AGENT_NAME}"),
        gen_ai.operation.name = "invoke_agent",
        gen_ai.agent.name = AGENT_NAME,
        gen_ai.request.model = model_name,
    );
    let extractor =
        ExtractorBuilder::<Actions>::from_agent_builder(AgentBuilder::new(model).name(AGENT_NAME))
            .append_preamble(SYSTEM_PROMPT)
            .retries(RETRIES)
            .build();
    let response = async { extractor.extract(transcript).await }
        .instrument(span)
        .await
        .map_err(failure)?;
    Ok(Extraction {
        usage: usage_of(model_name, &response.usage),
        actions: response
            .output
            .actions
            .into_iter()
            .map(ExtractedAction::from)
            .collect(),
    })
}
