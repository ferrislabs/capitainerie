use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::{Judgement, LocatedAction},
    similarity::SimilarCard,
};
use rig_agent::{
    agent::AgentBuilder, completion::StructuredOutputError, extractor::ExtractorBuilder,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{Instrument, info_span};

use crate::{client::Engine, usage::usage_of};

pub(crate) const AGENT_NAME: &str = "deduplicator";
pub(crate) const RETRIES: usize = 1;
const TEMPERATURE: f64 = 0.0;
const MAX_TOKENS: u32 = 1024;

pub(crate) const SYSTEM_PROMPT: &str = "\
You decide whether two descriptions of work are the same task. The user sends a NEW action taken from a meeting and an EXISTING card of a task board. Both are usually in French.
Answer same_task = true when both ask for the same deliverable or outcome, even if they are worded differently, or mention a different owner, deadline or level of detail.
Different words for the same thing are the same task: synonyms and rephrasings (\"démo\" and \"présentation\", \"page de prix\" and \"grille tarifaire\") do not make a different task, and neither do extra details on one side.
Answer same_task = false when they concern a different object, client, system, person or deliverable, or when they apply a different verb to the same object (write, review, fix, open a ticket, announce), because each produces a different outcome, or when one is a step that precedes or follows the other without being the same deliverable.
Examples:
- \"Relancer Acme\" and \"Relancer Globex\": false (different clients).
- \"Ouvrir un ticket pour le bug de facturation\" and \"Corriger le bug de facturation\": false (a ticket is not the fix).
- \"Actualiser la grille tarifaire sur le site\" and \"Mettre à jour la page de prix sur le site\": true (same deliverable, other words).
- \"Écrire la documentation de l'API\" and \"Relire la documentation de l'API\": false (writing is not reviewing).
- \"Préparer la démo de jeudi\" and \"Organiser la présentation de jeudi\": true (a demo is a presentation).
- \"Envoyer le devis à Acme avant mardi\" and \"Envoyer le devis à Acme\": true (the deadline does not make a new task).
reason: one short sentence in French justifying the answer.";

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
pub(crate) struct Verdict {
    #[schemars(
        description = "true when the new action and the existing card are the same task, false otherwise."
    )]
    pub same_task: bool,
    #[schemars(description = "One short sentence in French justifying the answer.")]
    #[serde(default, deserialize_with = "text")]
    pub reason: String,
}

fn failure(error: StructuredOutputError) -> CoreError {
    match error {
        StructuredOutputError::Deserialization { .. } | StructuredOutputError::EmptyResponse => {
            CoreError::Invalid(format!(
                "the language model did not return a valid judgement after {RETRIES} retry: {error}"
            ))
        }
        StructuredOutputError::Prompt(error) => {
            CoreError::Internal(format!("the language model call failed: {error}"))
        }
    }
}

fn text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Option::<String>::deserialize(deserializer).map(Option::unwrap_or_default)
}

fn question(action: &LocatedAction, candidate: &SimilarCard) -> String {
    format!(
        "NEW ACTION\ntitle: {}\ndescription: {}\n\nEXISTING CARD\n{}",
        action.action.title, action.action.description, candidate.text
    )
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "same_task": {
                "type": "boolean",
                "description": "true when the new action and the existing card are the same task, false otherwise.",
            },
            "reason": {
                "type": "string",
                "description": "One short sentence in French justifying the answer.",
            },
        },
        "required": ["same_task", "reason"],
        "additionalProperties": false,
    })
}

pub(crate) async fn run(
    engine: Engine,
    provider: &str,
    model_name: &str,
    action: &LocatedAction,
    candidate: &SimilarCard,
) -> Result<Judgement, CoreError> {
    let span = info_span!(
        "invoke_agent",
        otel.name = format!("invoke_agent {AGENT_NAME}"),
        gen_ai.operation.name = "invoke_agent",
        gen_ai.provider.name = provider,
        gen_ai.agent.name = AGENT_NAME,
        gen_ai.request.model = model_name,
    );
    let (verdict, usage) = match engine {
        Engine::Chat(model) => {
            let extractor = ExtractorBuilder::<Verdict>::from_agent_builder(
                AgentBuilder::new(model)
                    .name(AGENT_NAME)
                    .temperature(TEMPERATURE),
            )
            .append_preamble(SYSTEM_PROMPT)
            .retries(RETRIES)
            .build();
            let response = async { extractor.extract(question(action, candidate)).await }
                .instrument(span)
                .await
                .map_err(failure)?;
            (response.output, usage_of(model_name, &response.usage))
        }
        Engine::Anthropic(api) => {
            api.structured::<Verdict>(
                model_name,
                SYSTEM_PROMPT,
                &question(action, candidate),
                &schema(),
                MAX_TOKENS,
                RETRIES,
            )
            .instrument(span)
            .await?
        }
    };
    Ok(Judgement {
        same_task: verdict.same_task,
        reason: verdict.reason,
        usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_null_reason_does_not_fail_the_judgement() {
        let parsed: Verdict = serde_json::from_str(r#"{"same_task":true,"reason":null}"#).unwrap();
        assert!(parsed.same_task);
        assert_eq!(parsed.reason, "");
    }
}
