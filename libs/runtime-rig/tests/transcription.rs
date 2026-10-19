#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::post,
};
use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    provider::{ModelRef, ProviderAccess, ProviderKind},
    transcription::{
        AudioClip, AudioMime,
        ports::{Transcriber, TranscriberFactory},
    },
};
use capitainerie_runtime_rig::RigTranscribers;
use serde_json::json;
use tokio::net::TcpListener;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Part {
    name: String,
    file_name: Option<String>,
    content_type: Option<String>,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Recorded {
    path: String,
    authorization: Option<String>,
    parts: Vec<Part>,
}

#[derive(Clone)]
struct Fake {
    status: u16,
    body: String,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

impl Fake {
    fn new(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_owned(),
            recorded: Arc::default(),
        }
    }

    async fn serve(&self) -> String {
        let app = Router::new()
            .fallback(post(receive))
            .with_state(self.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("http://{address}")
    }

    fn requests(&self) -> Vec<Recorded> {
        self.recorded.lock().unwrap().clone()
    }
}

async fn receive(
    State(fake): State<Fake>,
    uri: Uri,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let mut parts = Vec::new();
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_owned();
        let file_name = field.file_name().map(str::to_owned);
        let content_type = field.content_type().map(str::to_owned);
        let bytes = field.bytes().await.unwrap().to_vec();
        parts.push(Part {
            name,
            file_name,
            content_type,
            bytes,
        });
    }
    fake.recorded.lock().unwrap().push(Recorded {
        path: uri.path().to_owned(),
        authorization: headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        parts,
    });
    (
        StatusCode::from_u16(fake.status).unwrap(),
        [("content-type", "application/json")],
        fake.body.clone(),
    )
        .into_response()
}

fn access(kind: ProviderKind, secret: &str, base: &str) -> ProviderAccess {
    ProviderAccess {
        kind,
        secret: SecretToken::new(secret),
        base_url: Some(base.to_owned()),
    }
}

fn model(provider: ProviderKind, id: &str) -> ModelRef {
    ModelRef {
        provider,
        model: id.to_owned(),
    }
}

fn clip() -> AudioClip {
    AudioClip {
        file_name: "réunion.m4a".into(),
        mime: AudioMime::Mp4,
        bytes: vec![1u8, 2, 3, 4, 5].into(),
    }
}

#[tokio::test]
async fn mistral_receives_the_file_and_the_model_with_the_key_as_a_bearer() {
    let fake = Fake::new(200, r#"{"text":"Bonjour à tous","language":"fr"}"#);
    let base = fake.serve().await;

    let text = RigTranscribers::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "mistral-key", &base))
        .transcribe(&model(ProviderKind::Mistral, "voxtral-mini-latest"), clip())
        .await
        .unwrap();

    assert_eq!(text.0, "Bonjour à tous");
    let requests = fake.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/audio/transcriptions");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some("Bearer mistral-key")
    );
    let by_name = |name: &str| {
        requests[0]
            .parts
            .iter()
            .find(|part| part.name == name)
            .unwrap()
    };
    assert_eq!(requests[0].parts.len(), 2);
    assert_eq!(by_name("model").bytes, b"voxtral-mini-latest");
    let file = by_name("file");
    assert_eq!(file.bytes, [1, 2, 3, 4, 5]);
    assert_eq!(file.file_name.as_deref(), Some("réunion.m4a"));
    assert_eq!(file.content_type.as_deref(), Some("audio/mp4"));
}

#[tokio::test]
async fn an_openai_compatible_server_is_called_under_its_base_url_and_needs_no_key() {
    let fake = Fake::new(200, r#"{"text":"hello"}"#);
    let base = fake.serve().await;

    let text = RigTranscribers::new()
        .unwrap()
        .bind(&access(
            ProviderKind::OpenAiCompatible,
            "",
            &format!("{base}/v1/"),
        ))
        .transcribe(
            &model(ProviderKind::OpenAiCompatible, "whisper-large-v3"),
            clip(),
        )
        .await
        .unwrap();

    assert_eq!(text.0, "hello");
    let request = &fake.requests()[0];
    assert_eq!(request.path, "/v1/audio/transcriptions");
    assert_eq!(request.authorization, None);
}

#[tokio::test]
async fn a_refusal_gives_a_readable_error_without_the_key() {
    let fake = Fake::new(
        401,
        &json!({"error": {"message": "invalid api key"}}).to_string(),
    );
    let base = fake.serve().await;

    let error = RigTranscribers::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "mistral-secret-key", &base))
        .transcribe(&model(ProviderKind::Mistral, "voxtral-mini-latest"), clip())
        .await
        .unwrap_err();

    let message = error.to_string();
    assert!(matches!(error, CoreError::Internal(_)), "{message}");
    assert!(message.contains("401"), "{message}");
    assert!(message.contains("invalid api key"), "{message}");
    assert!(!message.contains("mistral-secret-key"), "{message}");
}

#[tokio::test]
async fn a_plain_text_failure_is_truncated_and_a_server_error_is_reported() {
    let fake = Fake::new(500, &"x".repeat(2_000));
    let base = fake.serve().await;

    let error = RigTranscribers::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "k", &base))
        .transcribe(&model(ProviderKind::Mistral, "voxtral-mini-latest"), clip())
        .await
        .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("500"), "{message}");
    assert!(!message.contains('x'), "{message}");
}

#[tokio::test]
async fn an_answer_without_text_is_an_error() {
    let fake = Fake::new(200, r#"{"segments":[]}"#);
    let base = fake.serve().await;

    let error = RigTranscribers::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "k", &base))
        .transcribe(&model(ProviderKind::Mistral, "voxtral-mini-latest"), clip())
        .await
        .unwrap_err();

    assert!(error.to_string().contains("illisible"), "{error}");
}

#[tokio::test]
async fn an_unreachable_provider_is_an_error() {
    let error = RigTranscribers::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "k", "http://127.0.0.1:1"))
        .transcribe(&model(ProviderKind::Mistral, "voxtral-mini-latest"), clip())
        .await
        .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("injoignable"), "{message}");
    assert!(!message.contains("127.0.0.1"), "{message}");
}

#[tokio::test]
async fn anthropic_and_a_missing_base_url_never_reach_the_network() {
    let factory = RigTranscribers::new().unwrap();
    let anthropic = factory
        .bind(&access(ProviderKind::Anthropic, "k", "http://127.0.0.1:1"))
        .transcribe(&model(ProviderKind::Anthropic, "claude"), clip())
        .await
        .unwrap_err();
    let no_base = factory
        .bind(&ProviderAccess {
            kind: ProviderKind::OpenAiCompatible,
            secret: SecretToken::new(""),
            base_url: None,
        })
        .transcribe(&model(ProviderKind::OpenAiCompatible, "whisper"), clip())
        .await
        .unwrap_err();

    assert!(matches!(anthropic, CoreError::Invalid(_)));
    assert!(matches!(no_base, CoreError::Invalid(_)));
}

#[tokio::test]
async fn a_provider_answer_over_1_mib_is_refused() {
    let fake = Fake::new(200, &format!(r#"{{"text":"{}"}}"#, "a".repeat(1_100_000)));
    let base = fake.serve().await;

    let error = RigTranscribers::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "k", &base))
        .transcribe(&model(ProviderKind::Mistral, "voxtral-mini-latest"), clip())
        .await
        .unwrap_err();

    assert!(error.to_string().contains("trop volumineuse"), "{error}");
}

#[tokio::test]
async fn a_long_json_error_message_is_truncated_to_200_characters() {
    let fake = Fake::new(
        400,
        &json!({"error": {"message": "m".repeat(1_000)}}).to_string(),
    );
    let base = fake.serve().await;

    let error = RigTranscribers::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "k", &base))
        .transcribe(&model(ProviderKind::Mistral, "voxtral-mini-latest"), clip())
        .await
        .unwrap_err();

    assert!(error.to_string().matches('m').count() <= 205, "{error}");
}
