use std::time::Instant;

use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::Usage,
    conversation::turn::{
        ChatMessage, ChatRole, ToolSpec, TurnEnd, TurnEvent, TurnLimits, TurnSink, TurnState,
    },
    conversation::{ProposedCard, ToolRecord},
};
use serde_json::Value;

use crate::usage::truncate_chars;

const ARGUMENTS_MAX_CHARS: usize = 200;
const BUDGET_SPENT: &str = "The tool budget of this turn is spent: answer with what you have.";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Call {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolResult {
    pub id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Msg {
    User(String),
    Assistant { text: String, calls: Vec<Call> },
    Results(Vec<ToolResult>),
}

pub(crate) struct ModelRequest<'a> {
    pub system: &'a str,
    pub messages: &'a [Msg],
    pub tools: &'a [ToolSpec],
    pub max_tokens: u32,
}

#[derive(Debug, Default)]
pub(crate) struct ModelReply {
    pub text: String,
    pub calls: Vec<Call>,
    pub usage: Usage,
    pub stopped: bool,
}

pub(crate) trait DeltaSink: Send {
    fn delta(&mut self, text: &str) -> impl Future<Output = bool> + Send;
}

pub(crate) trait Provider: Send + Sync {
    fn stream<D: DeltaSink>(
        &self,
        request: &ModelRequest<'_>,
        deltas: &mut D,
    ) -> impl Future<Output = Result<ModelReply, CoreError>> + Send;
}

pub(crate) struct ToolOutput {
    pub succeeded: bool,
    pub text: String,
    pub summary: String,
    pub card: Option<ProposedCard>,
}

pub(crate) trait Tools: Send + Sync {
    fn specs(&self) -> Vec<ToolSpec>;

    fn call(&self, name: &str, arguments: &Value) -> impl Future<Output = ToolOutput> + Send;
}

struct Streaming<'a, S> {
    sink: &'a S,
    text: &'a mut String,
    started: bool,
}

impl<S: TurnSink> DeltaSink for Streaming<'_, S> {
    async fn delta(&mut self, text: &str) -> bool {
        let mut piece = text.to_owned();
        if !self.started {
            self.started = true;
            if !self.text.is_empty() && !self.text.ends_with(char::is_whitespace) {
                piece.insert_str(0, "\n\n");
            }
        }
        self.text.push_str(&piece);
        self.sink.emit(TurnEvent::Delta(piece)).await
    }
}

pub(crate) fn messages_of(history: &[ChatMessage]) -> Vec<Msg> {
    history
        .iter()
        .map(|message| match message.role {
            ChatRole::User => Msg::User(message.text.clone()),
            ChatRole::Assistant => Msg::Assistant {
                text: message.text.clone(),
                calls: Vec::new(),
            },
        })
        .collect()
}

pub(crate) fn summarize_arguments(arguments: &Value) -> String {
    truncate_chars(&arguments.to_string(), ARGUMENTS_MAX_CHARS)
}

fn latency_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub(crate) async fn run<P: Provider, T: Tools, S: TurnSink>(
    provider: &P,
    tools: &T,
    system: &str,
    history: &[ChatMessage],
    limits: TurnLimits,
    sink: &S,
    state: &mut TurnState,
) -> Result<TurnEnd, CoreError> {
    let specs = tools.specs();
    let mut messages = messages_of(history);
    let mut calls_made = 0usize;
    loop {
        let spent = u32::try_from(state.usage.output_tokens).unwrap_or(u32::MAX);
        let remaining = limits.max_output_tokens.saturating_sub(spent);
        if remaining == 0 {
            return Ok(TurnEnd::Completed);
        }
        let offered: &[ToolSpec] = if calls_made < limits.max_tool_calls {
            &specs
        } else {
            &[]
        };
        let request = ModelRequest {
            system,
            messages: &messages,
            tools: offered,
            max_tokens: remaining,
        };
        let reply = {
            let mut deltas = Streaming {
                sink,
                text: &mut state.text,
                started: false,
            };
            provider.stream(&request, &mut deltas).await
        };
        let reply = reply?;
        state.usage.absorb(&reply.usage);
        if reply.stopped {
            return Ok(TurnEnd::Interrupted);
        }
        if reply.calls.is_empty() {
            return Ok(TurnEnd::Completed);
        }
        messages.push(Msg::Assistant {
            text: reply.text,
            calls: reply.calls.clone(),
        });
        let mut results = Vec::with_capacity(reply.calls.len());
        for call in reply.calls {
            if calls_made >= limits.max_tool_calls {
                results.push(ToolResult {
                    id: call.id,
                    content: BUDGET_SPENT.to_owned(),
                    is_error: true,
                });
                continue;
            }
            calls_made += 1;
            let arguments = summarize_arguments(&call.input);
            let mut alive = sink
                .emit(TurnEvent::ToolStarted {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    arguments: arguments.clone(),
                })
                .await;
            let started = Instant::now();
            let output = tools.call(&call.name, &call.input).await;
            let latency = latency_ms(started);
            state.tool_calls.push(ToolRecord {
                name: call.name.clone(),
                arguments,
                succeeded: output.succeeded,
                latency_ms: latency,
                summary: output.summary.clone(),
            });
            alive = sink
                .emit(TurnEvent::ToolFinished {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    succeeded: output.succeeded,
                    latency_ms: latency,
                    summary: output.summary,
                })
                .await
                && alive;
            if let Some(card) = output.card {
                state.cards.push(card.clone());
                alive = sink.emit(TurnEvent::Card(card)).await && alive;
            }
            if !alive {
                return Ok(TurnEnd::Interrupted);
            }
            results.push(ToolResult {
                id: call.id,
                content: output.text,
                is_error: !output.succeeded,
            });
        }
        messages.push(Msg::Results(results));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
pub(crate) mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    pub(crate) struct Script(
        pub Mutex<Vec<Vec<Step>>>,
        pub Mutex<Vec<(Vec<Msg>, usize, u32)>>,
    );

    pub(crate) enum Step {
        Text(&'static str),
        Call(&'static str, &'static str, Value),
        Many(usize),
        Usage(u64, u64),
        Fail(&'static str),
    }

    impl Script {
        pub(crate) fn of(rounds: Vec<Vec<Step>>) -> Self {
            Self(Mutex::new(rounds), Mutex::new(Vec::new()))
        }
    }

    impl Provider for Script {
        async fn stream<D: DeltaSink>(
            &self,
            request: &ModelRequest<'_>,
            deltas: &mut D,
        ) -> Result<ModelReply, CoreError> {
            self.1.lock().unwrap().push((
                request.messages.to_vec(),
                request.tools.len(),
                request.max_tokens,
            ));
            let round = {
                let mut rounds = self.0.lock().unwrap();
                if rounds.is_empty() {
                    Vec::new()
                } else {
                    rounds.remove(0)
                }
            };
            let mut reply = ModelReply::default();
            for step in round {
                match step {
                    Step::Text(text) => {
                        reply.text.push_str(text);
                        if !deltas.delta(text).await {
                            reply.stopped = true;
                            return Ok(reply);
                        }
                    }
                    Step::Call(id, name, input) => reply.calls.push(Call {
                        id: id.into(),
                        name: name.into(),
                        input,
                    }),
                    Step::Many(count) => {
                        for n in 0..count {
                            reply.calls.push(Call {
                                id: format!("c{n}"),
                                name: "search_cards".into(),
                                input: json!({}),
                            });
                        }
                    }
                    Step::Usage(input, output) => {
                        reply.usage.input_tokens += input;
                        reply.usage.output_tokens += output;
                    }
                    Step::Fail(message) => return Err(CoreError::Internal(message.into())),
                }
            }
            Ok(reply)
        }
    }

    #[derive(Default)]
    pub(crate) struct Sink {
        pub events: Mutex<Vec<TurnEvent>>,
        pub gone_after: Option<usize>,
    }

    impl TurnSink for Sink {
        async fn emit(&self, event: TurnEvent) -> bool {
            let mut events = self.events.lock().unwrap();
            events.push(event);
            self.gone_after.is_none_or(|limit| events.len() <= limit)
        }
    }

    struct Backend {
        calls: AtomicUsize,
        card: Option<ProposedCard>,
    }

    impl Tools for Backend {
        fn specs(&self) -> Vec<ToolSpec> {
            vec![ToolSpec {
                name: "search_cards".into(),
                description: "d".into(),
                parameters: json!({"type": "object"}),
            }]
        }

        async fn call(&self, name: &str, _: &Value) -> ToolOutput {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ToolOutput {
                succeeded: name != "broken",
                text: format!("result of {name}"),
                summary: format!("summary of {name}"),
                card: self.card.clone(),
            }
        }
    }

    fn backend(card: Option<ProposedCard>) -> Backend {
        Backend {
            calls: AtomicUsize::new(0),
            card,
        }
    }

    fn history() -> Vec<ChatMessage> {
        vec![ChatMessage {
            role: ChatRole::User,
            text: "Cherche les cartes Acme".into(),
        }]
    }

    #[tokio::test]
    async fn text_tools_and_cards_stream_in_order_and_the_state_keeps_them() {
        let card = ProposedCard {
            card_id: Uuid::from_u128(5),
            title: "Ajouter un README".into(),
        };
        let provider = Script::of(vec![
            vec![
                Step::Text("Je cherche."),
                Step::Call("c1", "search_cards", json!({"query": "acme"})),
                Step::Usage(100, 10),
            ],
            vec![Step::Text("Voilà."), Step::Usage(150, 20)],
        ]);
        let sink = Sink::default();
        let mut state = TurnState::default();

        let end = run(
            &provider,
            &backend(Some(card.clone())),
            "system",
            &history(),
            TurnLimits::default(),
            &sink,
            &mut state,
        )
        .await
        .unwrap();

        assert_eq!(end, TurnEnd::Completed);
        let events = sink.events.lock().unwrap().clone();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0], TurnEvent::Delta("Je cherche.".into()));
        assert_eq!(
            events[1],
            TurnEvent::ToolStarted {
                id: "c1".into(),
                name: "search_cards".into(),
                arguments: "{\"query\":\"acme\"}".into(),
            }
        );
        assert!(
            matches!(&events[2], TurnEvent::ToolFinished { id, succeeded: true, summary, .. }
            if id == "c1" && summary == "summary of search_cards")
        );
        assert_eq!(events[3], TurnEvent::Card(card.clone()));
        assert_eq!(events[4], TurnEvent::Delta("\n\nVoilà.".into()));
        assert_eq!(state.text, "Je cherche.\n\nVoilà.");
        assert_eq!(state.cards, [card]);
        assert_eq!(state.tool_calls.len(), 1);
        assert_eq!(state.tool_calls[0].summary, "summary of search_cards");
        assert_eq!(
            (state.usage.input_tokens, state.usage.output_tokens),
            (250, 30)
        );
        let seen = provider.1.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert!(matches!(&seen[1].0[2], Msg::Results(results)
            if results[0].content == "result of search_cards" && !results[0].is_error));
    }

    #[tokio::test]
    async fn the_twenty_first_tool_call_is_refused_and_the_model_answers_without_tools() {
        let provider = Script::of(vec![vec![Step::Many(25)], vec![Step::Text("Fini.")]]);
        let tools = backend(None);
        let sink = Sink::default();
        let mut state = TurnState::default();

        let end = run(
            &provider,
            &tools,
            "system",
            &history(),
            TurnLimits::default(),
            &sink,
            &mut state,
        )
        .await
        .unwrap();

        assert_eq!(end, TurnEnd::Completed);
        assert_eq!(tools.calls.load(Ordering::SeqCst), 20);
        assert_eq!(state.tool_calls.len(), 20);
        let seen = provider.1.lock().unwrap();
        assert_eq!(seen[0].1, 1, "tools are offered while the budget lasts");
        assert_eq!(seen[1].1, 0, "and withdrawn once it is spent");
        let Msg::Results(results) = &seen[1].0[2] else {
            panic!("results expected");
        };
        assert_eq!(results.len(), 25, "every tool_use gets a result");
        assert!(results[20..].iter().all(|result| result.is_error));
    }

    #[tokio::test]
    async fn the_output_token_budget_shrinks_each_request_and_stops_the_turn() {
        let provider = Script::of(vec![
            vec![
                Step::Call("c1", "search_cards", json!({})),
                Step::Usage(10, 9_000),
            ],
            vec![
                Step::Call("c2", "search_cards", json!({})),
                Step::Usage(10, 7_000),
            ],
            vec![Step::Text("jamais")],
        ]);
        let sink = Sink::default();
        let mut state = TurnState::default();

        let end = run(
            &provider,
            &backend(None),
            "system",
            &history(),
            TurnLimits::default(),
            &sink,
            &mut state,
        )
        .await
        .unwrap();

        assert_eq!(end, TurnEnd::Completed);
        let seen = provider.1.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!((seen[0].2, seen[1].2), (16_000, 7_000));
        assert!(!state.text.contains("jamais"));
    }

    #[tokio::test]
    async fn a_client_that_leaves_stops_the_turn_and_the_partial_text_is_kept() {
        let provider = Script::of(vec![vec![
            Step::Text("Je cher"),
            Step::Text("che"),
            Step::Usage(5, 2),
        ]]);
        let sink = Sink {
            gone_after: Some(0),
            ..Sink::default()
        };
        let mut state = TurnState::default();

        let end = run(
            &provider,
            &backend(None),
            "system",
            &history(),
            TurnLimits::default(),
            &sink,
            &mut state,
        )
        .await
        .unwrap();

        assert_eq!(end, TurnEnd::Interrupted);
        assert_eq!(state.text, "Je cher");
    }

    #[tokio::test]
    async fn a_failing_provider_leaves_what_was_streamed_in_the_state() {
        let provider = Script::of(vec![vec![Step::Text("Début"), Step::Fail("boom")]]);
        let sink = Sink::default();
        let mut state = TurnState::default();

        let error = run(
            &provider,
            &backend(None),
            "system",
            &history(),
            TurnLimits::default(),
            &sink,
            &mut state,
        )
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Internal(message) if message == "boom"));
        assert_eq!(state.text, "Début");
    }

    #[tokio::test]
    async fn a_failed_tool_is_reported_to_the_model_and_recorded_as_failed() {
        let provider = Script::of(vec![
            vec![Step::Call("c1", "broken", json!({}))],
            vec![Step::Text("Désolé.")],
        ]);
        let sink = Sink::default();
        let mut state = TurnState::default();

        run(
            &provider,
            &backend(None),
            "system",
            &history(),
            TurnLimits::default(),
            &sink,
            &mut state,
        )
        .await
        .unwrap();

        assert!(!state.tool_calls[0].succeeded);
        let seen = provider.1.lock().unwrap();
        assert!(matches!(&seen[1].0[2], Msg::Results(results) if results[0].is_error));
    }

    #[tokio::test]
    async fn a_tool_call_in_flight_when_the_client_leaves_finishes_and_is_recorded() {
        let card = ProposedCard {
            card_id: Uuid::from_u128(8),
            title: "Ajouter un README".into(),
        };
        let provider = Script::of(vec![
            vec![
                Step::Text("Je propose."),
                Step::Call("c1", "propose_card", json!({"title": "Ajouter un README"})),
            ],
            vec![Step::Text("jamais")],
        ]);
        let tools = backend(Some(card.clone()));
        let sink = Sink {
            gone_after: Some(1),
            ..Sink::default()
        };
        let mut state = TurnState::default();

        let end = run(
            &provider,
            &tools,
            "system",
            &history(),
            TurnLimits::default(),
            &sink,
            &mut state,
        )
        .await
        .unwrap();

        assert_eq!(end, TurnEnd::Interrupted);
        assert_eq!(tools.calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.tool_calls.len(), 1);
        assert_eq!(state.cards, [card]);
        assert_eq!(provider.1.lock().unwrap().len(), 1, "no further model call");
    }
}
