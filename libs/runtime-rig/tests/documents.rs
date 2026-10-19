#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::post,
};
use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    document::{
        PdfDocument,
        ports::{DocumentReader, DocumentReaderFactory},
    },
    provider::{ModelRef, ProviderAccess, ProviderKind},
};
use capitainerie_runtime_rig::RigDocumentReaders;
use serde_json::{Value, json};
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
struct Recorded {
    method: String,
    path: String,
    authorization: Option<String>,
    content_type: Option<String>,
    body: Value,
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

async fn receive(State(fake): State<Fake>, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    fake.recorded.lock().unwrap().push(Recorded {
        method: "POST".into(),
        path: uri.path().to_owned(),
        authorization: header("authorization"),
        content_type: header("content-type"),
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
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

fn model() -> ModelRef {
    ModelRef {
        provider: ProviderKind::Mistral,
        model: "mistral-ocr-latest".into(),
    }
}

fn pdf() -> PdfDocument {
    PdfDocument {
        file_name: "rapport.pdf".into(),
        bytes: b"%PDF-1".to_vec().into(),
    }
}

async fn read(
    fake: &Fake,
    key: &str,
) -> Result<capitainerie_core::domain::document::DocumentText, CoreError> {
    let base = fake.serve().await;
    RigDocumentReaders::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, key, &base))
        .read(&model(), pdf())
        .await
}

#[tokio::test]
async fn mistral_receives_the_model_and_the_pdf_as_a_data_uri_with_the_key_as_a_bearer() {
    let fake = Fake::new(200, r#"{"pages":[{"index":0,"markdown":"Bonjour"}]}"#);

    let text = read(&fake, "mistral-key").await.unwrap();

    assert_eq!((text.text.as_str(), text.pages), ("Bonjour", 1));
    let requests = fake.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/v1/ocr");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some("Bearer mistral-key")
    );
    assert_eq!(
        requests[0].content_type.as_deref(),
        Some("application/json")
    );
    assert_eq!(
        requests[0].body,
        json!({
            "model": "mistral-ocr-latest",
            "document": {
                "type": "document_url",
                "document_url": "data:application/pdf;base64,JVBERi0x",
            },
        })
    );
}

#[tokio::test]
async fn pages_are_joined_in_index_order_by_a_blank_line() {
    let fake = Fake::new(
        200,
        &json!({"pages": [
            {"index": 2, "markdown": "trois"},
            {"index": 0, "markdown": "un"},
            {"index": 1, "markdown": "deux", "images": []},
        ]})
        .to_string(),
    );

    let text = read(&fake, "k").await.unwrap();

    assert_eq!(text.text, "un\n\ndeux\n\ntrois");
    assert_eq!(text.pages, 3);
}

#[tokio::test]
async fn a_refusal_gives_a_readable_error_without_the_key_or_the_url() {
    let fake = Fake::new(
        401,
        &json!({"error": {"message": "invalid api key"}}).to_string(),
    );

    let error = read(&fake, "mistral-secret-key").await.unwrap_err();

    let message = error.to_string();
    assert!(matches!(error, CoreError::Internal(_)), "{message}");
    assert!(message.contains("401"), "{message}");
    assert!(message.contains("invalid api key"), "{message}");
    assert!(!message.contains("mistral-secret-key"), "{message}");
    assert!(!message.contains("127.0.0.1"), "{message}");
}

#[tokio::test]
async fn a_plain_text_failure_never_echoes_the_raw_body() {
    let fake = Fake::new(500, &"x".repeat(2_000));

    let error = read(&fake, "k").await.unwrap_err();

    let message = error.to_string();
    assert!(message.contains("500"), "{message}");
    assert!(!message.contains('x'), "{message}");
}

#[tokio::test]
async fn a_long_json_error_message_is_truncated_to_200_characters() {
    let fake = Fake::new(400, &json!({"message": "m".repeat(1_000)}).to_string());

    let error = read(&fake, "k").await.unwrap_err();

    assert!(error.to_string().matches('m').count() <= 205, "{error}");
}

#[tokio::test]
async fn an_answer_without_pages_is_unreadable() {
    let fake = Fake::new(200, r#"{"model":"mistral-ocr-latest"}"#);

    let error = read(&fake, "k").await.unwrap_err();

    assert!(error.to_string().contains("illisible"), "{error}");
}

#[tokio::test]
async fn an_unreachable_provider_is_a_generic_error() {
    let error = RigDocumentReaders::new()
        .unwrap()
        .bind(&access(ProviderKind::Mistral, "k", "http://127.0.0.1:1"))
        .read(&model(), pdf())
        .await
        .unwrap_err();

    let message = error.to_string();
    assert!(
        matches!(&error, CoreError::Internal(inner) if inner == "Le service de lecture des PDF est injoignable."),
        "{message}"
    );
    assert!(!message.contains("127.0.0.1"), "{message}");
}

#[tokio::test]
async fn other_providers_never_reach_the_network() {
    let readers = RigDocumentReaders::new().unwrap();
    for kind in [ProviderKind::Anthropic, ProviderKind::OpenAiCompatible] {
        let error = readers
            .bind(&access(kind, "k", "http://127.0.0.1:1"))
            .read(&model(), pdf())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }
}

#[tokio::test]
async fn a_provider_answer_over_8_mib_is_refused_and_one_under_it_is_read() {
    let big = Fake::new(
        200,
        &json!({"pages": [{"index": 0, "markdown": "a".repeat(8 * 1024 * 1024 + 1)}]}).to_string(),
    );
    let large = Fake::new(
        200,
        &json!({"pages": [{"index": 0, "markdown": "a".repeat(2 * 1024 * 1024)}]}).to_string(),
    );

    let refused = read(&big, "k").await.unwrap_err();
    let accepted = read(&large, "k").await.unwrap();

    assert!(
        refused.to_string().contains("trop volumineuse"),
        "{refused}"
    );
    assert_eq!(accepted.text.len(), 2 * 1024 * 1024);
}
