#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use axum::{Extension, Router};
use capitainerie_auth::Identity;
use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{
        agent::SecretToken,
        document::{
            DocumentText, MAX_PDF_BYTES, PdfDocument,
            ports::{DocumentReader, DocumentReaderFactory},
        },
        provider::{
            ModelAssignments, ModelRef, ProviderAccess, ProviderKind,
            ports::{CatalogModel, ModelCatalog},
            service::PutProvider,
        },
        transcription::{
            AudioClip, AudioMime, MAX_AUDIO_BYTES, TranscriptionText,
            ports::{Transcriber, TranscriberFactory},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::Caller;
use reqwest::multipart::{Form, Part};
use serde_json::Value;
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Call {
    provider: ProviderKind,
    secret: String,
    model: String,
    file_name: String,
    mime: AudioMime,
    bytes: Vec<u8>,
}

#[derive(Clone, Default)]
struct FakeFactory {
    reply: Arc<Mutex<Option<Result<String, String>>>>,
    calls: Arc<Mutex<Vec<Call>>>,
}

struct FakeTranscriber {
    access: ProviderAccess,
    factory: FakeFactory,
}

impl FakeFactory {
    fn answering(text: &str) -> Self {
        let factory = Self::default();
        *factory.reply.lock().unwrap() = Some(Ok(text.to_owned()));
        factory
    }

    fn failing(message: &str) -> Self {
        let factory = Self::default();
        *factory.reply.lock().unwrap() = Some(Err(message.to_owned()));
        factory
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}

impl TranscriberFactory for FakeFactory {
    type Bound = FakeTranscriber;

    fn bind(&self, access: &ProviderAccess) -> FakeTranscriber {
        FakeTranscriber {
            access: access.clone(),
            factory: self.clone(),
        }
    }
}

impl Transcriber for FakeTranscriber {
    async fn transcribe(
        &self,
        model: &ModelRef,
        audio: AudioClip,
    ) -> Result<TranscriptionText, CoreError> {
        self.factory.calls.lock().unwrap().push(Call {
            provider: self.access.kind,
            secret: self.access.secret.expose().to_owned(),
            model: model.model.clone(),
            file_name: audio.file_name,
            mime: audio.mime,
            bytes: audio.bytes.to_vec(),
        });
        match self.factory.reply.lock().unwrap().clone() {
            Some(Ok(text)) => Ok(TranscriptionText(text)),
            Some(Err(message)) => Err(CoreError::Internal(message)),
            None => Err(CoreError::Internal("no reply".into())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Read {
    secret: String,
    model: String,
    file_name: String,
    bytes: Vec<u8>,
}

type ReaderReply = Result<(String, usize), String>;

#[derive(Clone, Default)]
struct FakeReaders {
    reply: Arc<Mutex<Option<ReaderReply>>>,
    reads: Arc<Mutex<Vec<Read>>>,
}

struct FakeReader {
    access: ProviderAccess,
    readers: FakeReaders,
}

impl FakeReaders {
    fn answering(text: &str, pages: usize) -> Self {
        let readers = Self::default();
        *readers.reply.lock().unwrap() = Some(Ok((text.to_owned(), pages)));
        readers
    }

    fn failing(message: &str) -> Self {
        let readers = Self::default();
        *readers.reply.lock().unwrap() = Some(Err(message.to_owned()));
        readers
    }

    fn reads(&self) -> Vec<Read> {
        self.reads.lock().unwrap().clone()
    }
}

impl DocumentReaderFactory for FakeReaders {
    type Bound = FakeReader;

    fn bind(&self, access: &ProviderAccess) -> FakeReader {
        FakeReader {
            access: access.clone(),
            readers: self.clone(),
        }
    }
}

impl DocumentReader for FakeReader {
    async fn read(
        &self,
        model: &ModelRef,
        document: PdfDocument,
    ) -> Result<DocumentText, CoreError> {
        self.readers.reads.lock().unwrap().push(Read {
            secret: self.access.secret.expose().to_owned(),
            model: model.model.clone(),
            file_name: document.file_name,
            bytes: document.bytes.to_vec(),
        });
        match self.readers.reply.lock().unwrap().clone() {
            Some(Ok((text, pages))) => Ok(DocumentText { text, pages }),
            Some(Err(message)) => Err(CoreError::Internal(message)),
            None => Err(CoreError::Internal("no reply".into())),
        }
    }
}

struct FakeCatalog;

impl ModelCatalog for FakeCatalog {
    async fn list_models(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        let model = |id: &str, chat: bool, embeddings: bool, transcription: bool| CatalogModel {
            provider: access.kind,
            id: id.into(),
            display_name: id.into(),
            chat,
            embeddings,
            transcription,
            ocr: id.contains("ocr"),
            context_window: None,
            input_price_per_mtok_micros: None,
            output_price_per_mtok_micros: None,
        };
        Ok(vec![
            model("mistral-small", true, false, false),
            model("mistral-embed", false, true, false),
            model("voxtral-mini-latest", false, false, true),
            model("mistral-ocr-latest", false, false, false),
        ])
    }
}

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

async fn pool() -> Option<PgPool> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    Some(postgres::connect(&url, 2).await.unwrap())
}

async fn organization(pool: &PgPool) -> Organization {
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = format!("http://idp.test/realms/{id}");
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(&slug)
    .bind(&issuer_url)
    .execute(pool)
    .await
    .unwrap();
    Organization {
        id: OrganizationId(id),
        slug: OrganizationSlug::try_from(slug).unwrap(),
        name: "Test".into(),
        issuer_url,
        created_at: std::time::SystemTime::now().into(),
    }
}

fn caller(organization: &Organization, actor: Option<&str>) -> Caller {
    Caller {
        identity: Identity {
            subject: "user-1".into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: actor.map(|client_id| capitainerie_auth::Actor {
                sub: "agent".into(),
                client_id: Some(client_id.to_owned()),
                act: None,
            }),
            roles: vec!["capitainerie-admin".into()],
        },
        organization: organization.clone(),
        token: SecretToken::new("token"),
    }
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[9u8; 32]).unwrap())
}

async fn configure_transcriber(pool: &PgPool, organization: &Organization) {
    let usecase = usecase(pool);
    let subject = caller(organization, None).subject();
    usecase
        .put_provider(PutProvider {
            subject: subject.clone(),
            kind: ProviderKind::Mistral,
            secret: SecretToken::new("mistral-secret-key"),
            base_url: None,
        })
        .await
        .unwrap();
    let chat = ModelRef {
        provider: ProviderKind::Mistral,
        model: "mistral-small".into(),
    };
    let mut assignments = ModelAssignments::defaults("mistral-small", "mistral-embed");
    assignments.extractor = chat.clone();
    assignments.judge = chat.clone();
    assignments.writer = chat;
    assignments.embeddings.provider = ProviderKind::Mistral;
    assignments.transcriber = Some(ModelRef {
        provider: ProviderKind::Mistral,
        model: "voxtral-mini-latest".into(),
    });
    usecase
        .put_model_assignments(subject, assignments, &FakeCatalog)
        .await
        .unwrap();
}

async fn configure_reader(pool: &PgPool, organization: &Organization) {
    configure_transcriber(pool, organization).await;
    let usecase = usecase(pool);
    let subject = caller(organization, None).subject();
    let mut assignments = usecase
        .get_model_assignments(subject.clone())
        .await
        .unwrap()
        .unwrap();
    assignments.reader = Some(ModelRef {
        provider: ProviderKind::Mistral,
        model: "mistral-ocr-latest".into(),
    });
    usecase
        .put_model_assignments(subject, assignments, &FakeCatalog)
        .await
        .unwrap();
}

async fn serve(pool: &PgPool, caller: Caller, factory: FakeFactory) -> String {
    serve_all(pool, caller, factory, FakeReaders::default()).await
}

async fn serve_documents(pool: &PgPool, caller: Caller, readers: FakeReaders) -> String {
    serve_all(pool, caller, FakeFactory::default(), readers)
        .await
        .replace("/transcriptions", "/documents/text")
}

async fn serve_all(
    pool: &PgPool,
    caller: Caller,
    factory: FakeFactory,
    readers: FakeReaders,
) -> String {
    let app: Router =
        capitainerie_handlers_transcriptions::router::<(), _, _>(usecase(pool), factory, readers)
            .layer(Extension(caller));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}/api/v1/transcriptions")
}

fn file(name: &str, content_type: &str, bytes: Vec<u8>) -> Form {
    Form::new().part(
        "file",
        Part::bytes(bytes)
            .file_name(name.to_owned())
            .mime_str(content_type)
            .unwrap(),
    )
}

async fn post(url: &str, form: Form) -> (u16, Value) {
    let response = reqwest::Client::new()
        .post(url)
        .multipart(form)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    (status, response.json().await.unwrap_or(Value::Null))
}

#[tokio::test]
async fn a_member_gets_the_text_and_the_model_and_the_provider_sees_the_org_key() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let factory = FakeFactory::answering("Bonjour à tous");
    let url = serve(&pool, caller(&organization, None), factory.clone()).await;

    let (status, body) = post(&url, file("réunion.m4a", "audio/mp4", vec![7; 2048])).await;

    assert_eq!(status, 200, "{body}");
    assert_eq!(body["text"], "Bonjour à tous");
    assert_eq!(body["model"], "voxtral-mini-latest");
    let calls = factory.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].provider, ProviderKind::Mistral);
    assert_eq!(calls[0].secret, "mistral-secret-key");
    assert_eq!(calls[0].model, "voxtral-mini-latest");
    assert_eq!(calls[0].file_name, "réunion.m4a");
    assert_eq!(calls[0].mime, AudioMime::Mp4);
    assert_eq!(calls[0].bytes.len(), 2048);
}

#[tokio::test]
async fn nothing_of_the_audio_is_written_to_the_database() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let url = serve(
        &pool,
        caller(&organization, None),
        FakeFactory::answering("ok"),
    )
    .await;
    let marker = b"AUDIO-MARKER-0123456789".to_vec();
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM transcripts WHERE organization_id = $1")
            .bind(organization.id.0)
            .fetch_one(&pool)
            .await
            .unwrap();

    let (status, _) = post(&url, file("a.wav", "audio/wav", marker)).await;

    let after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM transcripts WHERE organization_id = $1")
            .bind(organization.id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    let runs: i64 = sqlx::query_scalar("SELECT count(*) FROM runs WHERE organization_id = $1")
        .bind(organization.id.0)
        .fetch_one(&pool)
        .await
        .unwrap();
    let leaked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND reason LIKE '%AUDIO-MARKER%'",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, 200);
    assert_eq!((before, after, runs, leaked), (0, 0, 0, 0));
}

#[tokio::test]
async fn without_a_transcriber_the_answer_is_a_409_with_its_code() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    let factory = FakeFactory::answering("never");
    let url = serve(&pool, caller(&organization, None), factory.clone()).await;

    let (status, body) = post(&url, file("a.mp3", "audio/mpeg", vec![1; 10])).await;

    assert_eq!(status, 409);
    assert_eq!(body["code"], "transcriber_not_configured");
    assert!(factory.calls().is_empty());
}

#[tokio::test]
async fn a_file_over_25_mib_is_a_413_and_never_reaches_the_provider() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let factory = FakeFactory::answering("never");
    let url = serve(&pool, caller(&organization, None), factory.clone()).await;

    let (just_over, _) = post(
        &url,
        file("a.mp3", "audio/mpeg", vec![0; MAX_AUDIO_BYTES + 1]),
    )
    .await;
    let (far_over, body) = post(
        &url,
        file("a.mp3", "audio/mpeg", vec![0; MAX_AUDIO_BYTES + 256 * 1024]),
    )
    .await;
    let (exactly, _) = post(&url, file("a.mp3", "audio/mpeg", vec![0; MAX_AUDIO_BYTES])).await;

    assert_eq!(just_over, 413);
    assert_eq!(far_over, 413, "{body}");
    assert_eq!(exactly, 200, "the exact limit passes");
    assert_eq!(factory.calls().len(), 1);
}

#[tokio::test]
async fn an_unsupported_format_is_a_415() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let factory = FakeFactory::answering("never");
    let url = serve(&pool, caller(&organization, None), factory.clone()).await;

    let (pdf, body) = post(&url, file("notes.pdf", "application/pdf", vec![1; 10])).await;
    let (bare, _) = post(&url, file("notes", "application/octet-stream", vec![1; 10])).await;
    let (by_type, _) = post(&url, file("notes", "audio/ogg", vec![1; 10])).await;

    assert_eq!(pdf, 415);
    assert_eq!(body["code"], "unsupported_media_type");
    assert_eq!(bare, 415);
    assert_eq!(by_type, 200);
    assert_eq!(factory.calls().len(), 1);
}

#[tokio::test]
async fn a_body_that_is_not_a_single_file_part_is_a_422() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let url = serve(
        &pool,
        caller(&organization, None),
        FakeFactory::answering("never"),
    )
    .await;

    let (no_file, _) = post(&url, Form::new().text("other", "x")).await;
    let (two, _) = post(
        &url,
        file("a.mp3", "audio/mpeg", vec![1; 10]).part(
            "file",
            Part::bytes(vec![2; 10])
                .file_name("b.mp3")
                .mime_str("audio/mpeg")
                .unwrap(),
        ),
    )
    .await;
    let (empty, _) = post(&url, file("a.mp3", "audio/mpeg", Vec::new())).await;

    assert_eq!((no_file, two, empty), (422, 422, 422));
}

#[tokio::test]
async fn an_agent_is_refused_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let factory = FakeFactory::answering("never");
    let url = serve(
        &pool,
        caller(&organization, Some("capitainerie-agent-writer")),
        factory.clone(),
    )
    .await;

    let (status, _) = post(&url, file("a.mp3", "audio/mpeg", vec![1; 10])).await;

    assert_eq!(status, 403);
    assert!(factory.calls().is_empty());
    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'transcription.create' AND NOT allowed",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denied, 1);
}

#[tokio::test]
async fn a_provider_failure_is_a_502_that_carries_its_message() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let url = serve(
        &pool,
        caller(&organization, None),
        FakeFactory::failing("the provider answered 401 Unauthorized (check the API key): bad key"),
    )
    .await;

    let (status, body) = post(&url, file("a.mp3", "audio/mpeg", vec![1; 10])).await;

    assert_eq!(status, 502);
    assert_eq!(body["code"], "bad_gateway");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("check the API key")
    );
    assert!(!body.to_string().contains("mistral-secret-key"));
}

fn pdf_bytes(total: usize) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    bytes.resize(total, b'0');
    bytes
}

#[tokio::test]
async fn a_member_gets_the_text_the_pages_and_the_model_and_the_reader_sees_the_org_key() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let readers = FakeReaders::answering("# Titre\n\nTexte", 2);
    let url = serve_documents(&pool, caller(&organization, None), readers.clone()).await;

    let (status, body) = post(
        &url,
        file("rapport.pdf", "application/pdf", pdf_bytes(2048)),
    )
    .await;

    assert_eq!(status, 200, "{body}");
    assert_eq!(body["text"], "# Titre\n\nTexte");
    assert_eq!(body["model"], "mistral-ocr-latest");
    assert_eq!(body["pages"], 2);
    let reads = readers.reads();
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].secret, "mistral-secret-key");
    assert_eq!(reads[0].model, "mistral-ocr-latest");
    assert_eq!(reads[0].file_name, "rapport.pdf");
    assert_eq!(reads[0].bytes.len(), 2048);
}

#[tokio::test]
async fn a_pdf_declared_by_its_content_type_only_is_read() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let readers = FakeReaders::answering("ok", 1);
    let url = serve_documents(&pool, caller(&organization, None), readers.clone()).await;

    let (status, _) = post(&url, file("scan", "application/pdf", pdf_bytes(64))).await;

    assert_eq!(status, 200);
    assert_eq!(readers.reads().len(), 1);
}

#[tokio::test]
async fn nothing_of_the_pdf_is_written_to_the_database() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let url = serve_documents(
        &pool,
        caller(&organization, None),
        FakeReaders::answering("ok", 1),
    )
    .await;
    let mut marker = b"%PDF-1.4 PDF-MARKER-0123456789".to_vec();
    marker.extend_from_slice(&[0; 32]);

    let (status, _) = post(&url, file("a.pdf", "application/pdf", marker)).await;

    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM transcripts WHERE organization_id = $1), (SELECT count(*) FROM runs WHERE organization_id = $1), (SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND reason LIKE '%PDF-MARKER%')",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, 200);
    assert_eq!(counts, (0, 0, 0));
}

#[tokio::test]
async fn without_a_reader_the_answer_is_a_409_before_the_body_is_judged() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_transcriber(&pool, &organization).await;
    let readers = FakeReaders::answering("never", 1);
    let url = serve_documents(&pool, caller(&organization, None), readers.clone()).await;

    let (valid, body) = post(&url, file("a.pdf", "application/pdf", pdf_bytes(64))).await;
    let (not_even_a_pdf, _) = post(&url, file("a.txt", "text/plain", vec![1; 10])).await;

    assert_eq!(valid, 409);
    assert_eq!(body["code"], "reader_not_configured");
    assert_eq!(not_even_a_pdf, 409);
    assert!(readers.reads().is_empty());
}

#[tokio::test]
async fn a_file_over_25_mib_is_a_413_and_never_reaches_the_reader() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let readers = FakeReaders::answering("ok", 1);
    let url = serve_documents(&pool, caller(&organization, None), readers.clone()).await;

    let (just_over, _) = post(
        &url,
        file("a.pdf", "application/pdf", pdf_bytes(MAX_PDF_BYTES + 1)),
    )
    .await;
    let (far_over, body) = post(
        &url,
        file(
            "a.pdf",
            "application/pdf",
            pdf_bytes(MAX_PDF_BYTES + 256 * 1024),
        ),
    )
    .await;
    let (exactly, _) = post(
        &url,
        file("a.pdf", "application/pdf", pdf_bytes(MAX_PDF_BYTES)),
    )
    .await;

    assert_eq!(just_over, 413);
    assert_eq!(far_over, 413, "{body}");
    assert_eq!(exactly, 200, "the exact limit passes");
    assert_eq!(readers.reads().len(), 1);
}

#[tokio::test]
async fn a_file_that_is_not_a_pdf_is_a_415() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let readers = FakeReaders::answering("ok", 1);
    let url = serve_documents(&pool, caller(&organization, None), readers.clone()).await;

    let (fake_pdf, body) = post(
        &url,
        file("faux.pdf", "application/pdf", b"just text".to_vec()),
    )
    .await;
    let (almost, _) = post(&url, file("faux.pdf", "application/pdf", b"%PDF".to_vec())).await;
    let (text, _) = post(&url, file("notes.txt", "text/plain", pdf_bytes(64))).await;
    let (audio, _) = post(&url, file("a.mp3", "audio/mpeg", vec![1; 10])).await;
    let (bare, _) = post(
        &url,
        file("notes", "application/octet-stream", pdf_bytes(64)),
    )
    .await;

    assert_eq!(fake_pdf, 415);
    assert_eq!(body["code"], "unsupported_media_type");
    assert_eq!((almost, text, audio, bare), (415, 415, 415, 415));
    assert!(readers.reads().is_empty());
}

#[tokio::test]
async fn a_pdf_body_that_is_not_a_single_file_part_is_a_422() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let url = serve_documents(
        &pool,
        caller(&organization, None),
        FakeReaders::answering("never", 1),
    )
    .await;

    let (no_file, _) = post(&url, Form::new().text("other", "x")).await;
    let (two, _) = post(
        &url,
        file("a.pdf", "application/pdf", pdf_bytes(64)).part(
            "file",
            Part::bytes(pdf_bytes(64))
                .file_name("b.pdf")
                .mime_str("application/pdf")
                .unwrap(),
        ),
    )
    .await;
    let (empty, _) = post(&url, file("a.pdf", "application/pdf", Vec::new())).await;

    assert_eq!((no_file, two, empty), (422, 422, 422));
}

#[tokio::test]
async fn an_agent_is_refused_and_the_denial_is_audited_before_the_body_is_read() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let readers = FakeReaders::answering("never", 1);
    let url = serve_documents(
        &pool,
        caller(&organization, Some("capitainerie-agent-writer")),
        readers.clone(),
    )
    .await;

    let (status, _) = post(&url, file("a.pdf", "application/pdf", pdf_bytes(64))).await;

    assert_eq!(status, 403);
    assert!(readers.reads().is_empty());
    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'document.read' AND NOT allowed",
    )
    .bind(organization.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denied, 1);
}

#[tokio::test]
async fn a_reader_failure_is_a_502_that_carries_its_message() {
    let Some(pool) = pool().await else { return };
    let organization = organization(&pool).await;
    configure_reader(&pool, &organization).await;
    let url = serve_documents(
        &pool,
        caller(&organization, None),
        FakeReaders::failing("Le service de lecture des PDF est injoignable."),
    )
    .await;

    let (status, body) = post(&url, file("a.pdf", "application/pdf", pdf_bytes(64))).await;

    assert_eq!(status, 502);
    assert_eq!(body["code"], "bad_gateway");
    assert_eq!(
        body["message"],
        "Le service de lecture des PDF est injoignable."
    );
    assert!(!body.to_string().contains("mistral-secret-key"));
}
