mod anthropic_stream;
mod driver;
mod openai_stream;
mod sse;
mod tools;

use std::sync::Arc;

use capitainerie_common::CoreError;
use capitainerie_core::domain::conversation::{
    CAPTAIN_AGENT,
    turn::{CaptainRuntime, LocalTools, TurnEnd, TurnRequest, TurnSink, TurnState},
};
use tracing::{Instrument, info_span};

use self::{anthropic_stream::AnthropicStream, openai_stream::OpenAiStream, tools::CaptainTools};
use crate::{
    args::LlmArgs,
    client::{Clients, Endpoint},
    error::RuntimeError,
    mcp::McpSession,
    prices::price_of,
};

const SYSTEM_PROMPT: &str = "\
Tu es le Capitaine de Capitainerie, l'assistant d'une équipe qui pilote son travail sur un tableau Kanban. Tu réponds toujours en français, de façon brève et concrète.
Un membre te confie une mission. Si elle est floue, pose une seule question précise avant d'agir. Sinon, agis.
Tes outils :
- search_cards : cherche les cartes du tableau. Appelle-le avant de proposer une carte, pour ne pas créer de doublon ; si une carte proche existe, dis-le et propose plutôt un commentaire.
- list_repositories : liste les dépôts GitHub auxquels l'organisation donne accès. Utilise-le quand la mission vise un dépôt.
- propose_card : propose une carte. Elle arrive toujours dans la colonne « proposée » : seul un membre la valide, la rejette ou la confie à Claude Code, avec les boutons sous ta réponse. Ne dis jamais qu'une carte est validée, démarrée ou terminée.
- comment_card : ajoute un commentaire à une carte existante.
Un titre de carte est à l'impératif, court, sans point final. La description dit quoi faire et pourquoi, et nomme le dépôt quand il y en a un. Une mission, une carte, sauf si le membre en demande plusieurs.
Tu ne peux ni déplacer, ni modifier, ni exécuter une carte. Si on te le demande, explique que le membre le fait lui-même depuis la carte.
Quand un outil échoue, lis l'erreur : corrige ton appel une fois, sinon explique le problème au membre.";

#[derive(Clone)]
pub struct RigCaptain {
    clients: Arc<Clients>,
    mcp_url: Arc<str>,
}

impl RigCaptain {
    pub fn new(args: &LlmArgs) -> Result<Self, RuntimeError> {
        args.mcp_url
            .parse::<http::Uri>()
            .map_err(|error| RuntimeError::McpUrl(error.to_string()))?;
        Ok(Self {
            clients: Arc::new(Clients::new(args)?),
            mcp_url: Arc::from(args.mcp_url.as_str()),
        })
    }
}

impl CaptainRuntime for RigCaptain {
    async fn run_turn<L: LocalTools, S: TurnSink>(
        &self,
        request: &TurnRequest,
        local: &L,
        sink: &S,
        state: &mut TurnState,
    ) -> Result<TurnEnd, CoreError> {
        let endpoint = self.clients.endpoint(&request.target)?;
        let model = request.target.model.model.as_str();
        let kind = request.target.model.provider;
        let span = info_span!(
            "invoke_agent",
            otel.name = "invoke_agent captain",
            gen_ai.operation.name = "invoke_agent",
            gen_ai.provider.name = crate::client::provider_name(kind),
            gen_ai.agent.name = CAPTAIN_AGENT,
            gen_ai.request.model = model,
        );
        async {
            let session = McpSession::connect(
                &self.mcp_url,
                &request.mcp_token,
                request.traceparent.as_deref(),
            )
            .await?;
            let peer = session.peer();
            let tools = CaptainTools { mcp: &peer, local };
            let outcome = match endpoint {
                Endpoint::Anthropic(api) => {
                    let provider = AnthropicStream {
                        api,
                        model: model.to_owned(),
                    };
                    driver::run(
                        &provider,
                        &tools,
                        SYSTEM_PROMPT,
                        &request.history,
                        request.limits,
                        sink,
                        state,
                    )
                    .await
                }
                Endpoint::OpenAi { http, url, key } => {
                    let provider = OpenAiStream {
                        http,
                        url,
                        key,
                        model: model.to_owned(),
                    };
                    driver::run(
                        &provider,
                        &tools,
                        SYSTEM_PROMPT,
                        &request.history,
                        request.limits,
                        sink,
                        state,
                    )
                    .await
                }
            };
            session.close().await;
            state.usage.model = model.to_owned();
            if let Some(price) = price_of(kind, model) {
                let micros = (u128::from(state.usage.input_tokens)
                    * u128::from(price.input_per_mtok_micros)
                    + u128::from(state.usage.output_tokens)
                        * u128::from(price.output_per_mtok_micros))
                    / 1_000_000;
                state.cost_micros = i64::try_from(micros).unwrap_or(i64::MAX);
            }
            outcome
        }
        .instrument(span)
        .await
    }
}
