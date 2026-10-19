use std::convert::Infallible;

use axum::{
    Extension, Json,
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{ConversationId, CoreError};
use capitainerie_core::domain::{
    authorization::Subject,
    conversation::{
        CAPTAIN_AGENT, MessageStatus,
        service::TurnOutcome,
        turn::{
            CaptainRuntime, TurnEnd, TurnError, TurnEvent, TurnLimits, TurnRequest, TurnSink,
            TurnState,
        },
    },
    delegation::{delegate_to_mcp, ports::TokenExchanger},
    execution::ports::CodeHost,
};
use capitainerie_handlers::{ApiError, Caller};
use chrono::{DateTime, Utc};
use futures_util::{Stream, stream};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::{dto::SendMessageRequest, state::ConversationsState, tools::ApiTools};

const BUFFER: usize = 64;
const ERROR_MAX_CHARS: usize = 300;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/conversations/{id}/messages")]
pub struct MessagesPath {
    pub id: ConversationId,
}

struct ChannelSink(mpsc::Sender<Event>);

impl ChannelSink {
    async fn send(&self, name: &str, data: &Value) -> bool {
        match Event::default().event(name).json_data(data) {
            Ok(event) => self.0.send(event).await.is_ok(),
            Err(_) => true,
        }
    }
}

impl TurnSink for ChannelSink {
    async fn emit(&self, event: TurnEvent) -> bool {
        match event {
            TurnEvent::Delta(text) => self.send("delta", &json!({ "text": text })).await,
            TurnEvent::ToolStarted {
                id,
                name,
                arguments,
            } => {
                self.send(
                    "tool",
                    &json!({ "phase": "start", "id": id, "name": name, "arguments": arguments }),
                )
                .await
            }
            TurnEvent::ToolFinished {
                id,
                name,
                succeeded,
                latency_ms,
                summary,
            } => {
                self.send(
                    "tool",
                    &json!({
                        "phase": "end",
                        "id": id,
                        "name": name,
                        "succeeded": succeeded,
                        "latency_ms": latency_ms,
                        "summary": summary,
                    }),
                )
                .await
            }
            TurnEvent::Card(card) => {
                self.send(
                    "card",
                    &json!({ "card_id": card.card_id, "title": card.title }),
                )
                .await
            }
        }
    }
}

fn refusal(error: TurnError) -> ApiError {
    match error {
        TurnError::InProgress => ApiError::ConflictCode {
            code: "turn_in_progress",
            message: "another turn is already running in this conversation".into(),
        },
        TurnError::Full => ApiError::ConflictCode {
            code: "conversation_full",
            message: "this conversation reached its token limit: open a new one".into(),
        },
        TurnError::NotConfigured => ApiError::ConflictCode {
            code: "captain_not_configured",
            message: "no model serves the Captain: choose one in Settings > Models".into(),
        },
        TurnError::Core(error) => error.into(),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/messages",
    operation_id = "sendConversationMessage",
    tag = super::TAG,
    params(("id" = ConversationId, Path, description = "Conversation identifier")),
    request_body = SendMessageRequest,
    responses(
        (
            status = 200,
            description = "One turn, as a text/event-stream. Events: `delta` {text}; `tool` {phase: start|end, id, name, arguments | succeeded, latency_ms, summary}; `card` {card_id, title}; `done` {status, usage}; `error` {code, message}. The stream ends after `done` or `error`. Closing the connection interrupts the turn.",
            content_type = "text/event-stream",
            body = String
        ),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "Agents may not use conversations", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such conversation for the caller", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "turn_in_progress, conversation_full, captain_not_configured or captain_unavailable", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "Empty or oversized message, archived conversation, or nothing to answer", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler<R, H, X>(
    MessagesPath { id }: MessagesPath,
    State(state): State<ConversationsState<R, H, X>>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<SendMessageRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError>
where
    R: CaptainRuntime + 'static,
    H: CodeHost + 'static,
    X: TokenExchanger + 'static,
{
    let subject = caller.subject();
    state
        .usecase
        .check_conversation_writable(subject.clone(), id)
        .await?;
    let target = state
        .usecase
        .captain_target(subject.clone())
        .await?
        .ok_or_else(|| refusal(TurnError::NotConfigured))?;
    let exchanger = state
        .exchanger
        .clone()
        .ok_or_else(|| ApiError::ConflictCode {
            code: "captain_unavailable",
            message: "the API has no credentials to act as the Captain".into(),
        })?;
    let started = state
        .usecase
        .start_conversation_turn(subject.clone(), id, request.message)
        .await
        .map_err(refusal)?;
    let delegated = match delegate_to_mcp(
        exchanger.as_ref(),
        &caller.organization.issuer_url,
        &caller.token,
        CAPTAIN_AGENT,
    )
    .await
    {
        Ok(delegated) => delegated,
        Err(error) => {
            if let Err(release) = state
                .usecase
                .abandon_conversation_turn(subject, id, started.leased_at)
                .await
            {
                tracing::error!(%release, "the turn slot could not be freed");
            }
            return Err(match error {
                CoreError::Forbidden { reason } => ApiError::ConflictCode {
                    code: "captain_unavailable",
                    message: reason.unwrap_or_else(|| "the Captain may not act for you".into()),
                },
                other => other.into(),
            });
        }
    };
    let turn = TurnRequest {
        target,
        history: started.history,
        mcp_token: delegated.token,
        traceparent: None,
        limits: TurnLimits::default(),
    };
    let (sender, receiver) = mpsc::channel(BUFFER);
    tokio::spawn(run_turn(
        state,
        subject,
        id,
        started.leased_at,
        turn,
        sender,
    ));
    let events = stream::unfold(receiver, |mut receiver| async move {
        receiver
            .recv()
            .await
            .map(|event| (Ok::<_, Infallible>(event), receiver))
    });
    Ok(Sse::new(events).keep_alive(KeepAlive::default()))
}

struct Finished {
    state: TurnState,
    status: MessageStatus,
    closing: Closing,
}

async fn run_turn<R, H, X>(
    state: ConversationsState<R, H, X>,
    subject: Subject,
    id: ConversationId,
    leased_at: DateTime<Utc>,
    turn: TurnRequest,
    sender: mpsc::Sender<Event>,
) where
    R: CaptainRuntime + 'static,
    H: CodeHost + 'static,
    X: TokenExchanger + 'static,
{
    let worker = tokio::spawn(execute(
        state.clone(),
        subject.clone(),
        turn.clone(),
        sender.clone(),
    ));
    let finished = match worker.await {
        Ok(finished) => finished,
        Err(join) => {
            tracing::error!(%join, conversation = %id, "the Captain's turn crashed");
            Finished {
                state: TurnState::default(),
                status: MessageStatus::Failed,
                closing: Closing::Failed(CoreError::Internal("the turn crashed".into())),
            }
        }
    };
    let outcome = TurnOutcome {
        content: finished.state.content(),
        status: finished.status,
        usage: finished.state.usage.clone(),
        cost_micros: finished.state.cost_micros,
        model: turn.target.model.clone(),
    };
    let stored = state
        .usecase
        .finish_conversation_turn(subject, id, leased_at, outcome)
        .await;
    if let Err(error) = &stored {
        tracing::error!(%error, conversation = %id, "the turn could not be stored");
    }
    let event = match (finished.closing, stored) {
        (Closing::Silent, _) => return,
        (_, Err(_)) => error_event("storage_failed", "the answer could not be saved"),
        (Closing::Done, Ok(())) => Event::default().event("done").json_data(json!({
            "status": "complete",
            "usage": {
                "input_tokens": finished.state.usage.input_tokens,
                "output_tokens": finished.state.usage.output_tokens,
                "cost_micros": finished.state.cost_micros,
            },
        })),
        (Closing::Failed(error), Ok(())) => {
            let message: String = error.to_string().chars().take(ERROR_MAX_CHARS).collect();
            error_event("provider_error", &message)
        }
        (Closing::TimedOut, Ok(())) => error_event(
            "turn_timeout",
            "the Captain took too long to answer; what it produced is kept",
        ),
    };
    if let Ok(event) = event {
        let _ = sender.send(event).await;
    }
}

async fn execute<R, H, X>(
    state: ConversationsState<R, H, X>,
    subject: Subject,
    turn: TurnRequest,
    sender: mpsc::Sender<Event>,
) -> Finished
where
    R: CaptainRuntime + 'static,
    H: CodeHost + 'static,
    X: TokenExchanger + 'static,
{
    let sink = ChannelSink(sender);
    let tools = ApiTools {
        usecase: state.usecase.clone(),
        code_host: state.code_host.clone(),
        subject: Subject {
            actor: Some(CAPTAIN_AGENT.to_owned()),
            ..subject
        },
    };
    let mut turn_state = TurnState::default();
    let result = tokio::time::timeout(
        state.deadline,
        state
            .runtime
            .run_turn(&turn, &tools, &sink, &mut turn_state),
    )
    .await;
    let (status, closing) = match result {
        Ok(Ok(TurnEnd::Completed)) => (MessageStatus::Complete, Closing::Done),
        Ok(Ok(TurnEnd::Interrupted)) => (MessageStatus::Interrupted, Closing::Silent),
        Ok(Err(error)) => {
            tracing::warn!(%error, "the Captain's turn failed");
            (MessageStatus::Failed, Closing::Failed(error))
        }
        Err(_) => {
            tracing::warn!("the Captain's turn ran past its deadline");
            (MessageStatus::Failed, Closing::TimedOut)
        }
    };
    Finished {
        state: turn_state,
        status,
        closing,
    }
}

enum Closing {
    Done,
    Silent,
    TimedOut,
    Failed(CoreError),
}

fn error_event(code: &str, message: &str) -> Result<Event, axum::Error> {
    Event::default()
        .event("error")
        .json_data(json!({ "code": code, "message": message }))
}
