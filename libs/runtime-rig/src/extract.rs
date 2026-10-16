use capitainerie_common::CoreError;
use capitainerie_core::domain::agent::{ExtractedAction, Extraction};
use rig_agent::{
    agent::AgentBuilder, completion::StructuredOutputError, extractor::ExtractorBuilder,
};
use rig_core::{DynModel, operation::Completion};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{Instrument, info_span};

use crate::{post, usage::usage_of};

pub(crate) const AGENT_NAME: &str = "extractor";
pub(crate) const RETRIES: usize = 2;
const TEMPERATURE: f64 = 0.1;

pub(crate) const SYSTEM_PROMPT: &str = "\
Extract the action items from the meeting transcript the user sends: concrete things that someone committed to do or that the meeting says must be done.
Rules:
- Read every speaker turn in order and ask of each one: does it commit to, accept or assign a task? Do not skip a turn because it is short or because another action was already found.
- One action = one commitment. When a task is requested and then accepted (\"Can you open a ticket?\" / \"Yes, I will\"), or announced and then repeated, report it ONCE, and its owner is the person who commits. A general need (\"we should do X\") that a speaker then takes on is the same action as the one taken on. Two different tasks stay two actions.
- Do not report an action that is cancelled, dropped or replaced later in the meeting. Do not report facts, status updates about the past, or opinions.
- Write the answer in the language of the transcript.
- title: a short imperative sentence naming the task, so that it can be understood without the transcript.
- description: the useful context, plus the owner and the deadline when the transcript states them.
- quote: the SHORTEST span of ONE speaker turn that states what is to be done, copied character for character from the transcript. Never include the speaker name or the \"Name :\" label. Never paraphrase, never correct typos or punctuation. When the commitment is only a short answer (\"Yes, I will do it today\"), quote the sentence of the request that names the task instead.
- owner: the person who commits to the task, as named in the transcript. The person who asks or assigns is NOT the owner; the person who accepts is. If the task is stated with \"on\", \"il faut\" or \"il faudrait\" and nobody takes it, omit the owner. Never guess an owner.
- due: the deadline as said in the transcript (\"vendredi\", \"demain matin\"), otherwise omit it.
- Do not report the same action twice.
- Do not invent actions. If the transcript contains no action item, return an empty list.

Example transcript:
Dominique : Elsa, tu peux relire le rapport d'audit avant son envoi au comité ?
Elsa : Oui, je le relis mardi.
Dominique : On devrait aussi archiver les anciens dossiers.
Dominique : Je commande les badges pour la restitution.
Elsa : Attends, la restitution est annulée, plus besoin de badges.

Example actions:
- title: \"Relire le rapport d'audit\", quote: \"tu peux relire le rapport d'audit avant son envoi au comité\", owner: \"Elsa\", due: \"mardi\"
- title: \"Archiver les anciens dossiers\", quote: \"archiver les anciens dossiers\", no owner, no due
(no action for the badges: it was cancelled)";

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
    #[serde(deserialize_with = "text")]
    pub title: String,
    #[schemars(description = "Context of the action, with owner and deadline when stated.")]
    #[serde(default, deserialize_with = "text")]
    pub description: String,
    #[schemars(
        description = "The shortest span of one speaker turn that states what is to be done, copied character for character, without the speaker name."
    )]
    #[serde(deserialize_with = "text")]
    pub quote: String,
    #[schemars(description = "The person who commits to the task; omit for an unassigned task.")]
    #[serde(default)]
    pub owner: Option<String>,
    #[schemars(description = "Deadline, when stated.")]
    #[serde(default)]
    pub due: Option<String>,
}

fn text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Option::<String>::deserialize(deserializer).map(Option::unwrap_or_default)
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
    let extractor = ExtractorBuilder::<Actions>::from_agent_builder(
        AgentBuilder::new(model)
            .name(AGENT_NAME)
            .temperature(TEMPERATURE),
    )
    .append_preamble(SYSTEM_PROMPT)
    .retries(RETRIES)
    .build();
    let response = async { extractor.extract(transcript).await }
        .instrument(span)
        .await
        .map_err(failure)?;
    Ok(Extraction {
        usage: usage_of(model_name, &response.usage),
        actions: post::clean(
            transcript,
            response
                .output
                .actions
                .into_iter()
                .map(ExtractedAction::from)
                .collect(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_text_fields_become_empty_instead_of_failing_the_extraction() {
        let parsed: Actions = serde_json::from_str(
            r#"{"actions":[{"title":"T","description":null,"quote":"Q","owner":null}]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(parsed.actions[0].description, "");
        assert_eq!(parsed.actions[0].owner, None);
    }
}
