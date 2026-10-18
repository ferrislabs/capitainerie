#![allow(clippy::unwrap_used)]

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        provider::{
            ModelAssignments, ModelRef, ProviderAccess, ProviderKind,
            ports::{CatalogModel, ModelCatalog},
            service::PutProvider,
        },
        run::service::SubmitTranscript,
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use sqlx::PgPool;
use uuid::Uuid;

const SECRET: &str = "sk-ant-api03-supersecretvalue-a1b2";

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

async fn organization(pool: &PgPool) -> OrganizationId {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(format!("test-{}", id.simple()))
    .bind(format!("http://idp.test/realms/{id}"))
    .execute(pool)
    .await
    .unwrap();
    OrganizationId(id)
}

fn subject(organization_id: OrganizationId, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin,
    }
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[7u8; 32]).unwrap())
}

fn anthropic(subject: Subject) -> PutProvider {
    PutProvider {
        subject,
        kind: ProviderKind::Anthropic,
        secret: SecretToken::new(SECRET),
        base_url: None,
    }
}

struct FakeCatalog;

impl ModelCatalog for FakeCatalog {
    async fn list_models(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        let model = |id: &str, chat: bool, embeddings: bool| CatalogModel {
            provider: access.kind,
            id: id.into(),
            display_name: id.into(),
            chat,
            embeddings,
            context_window: Some(200_000),
            input_price_per_mtok_micros: None,
            output_price_per_mtok_micros: None,
        };
        match access.kind {
            ProviderKind::Anthropic => Ok(vec![model("claude-haiku-5-5", true, false)]),
            ProviderKind::Mistral => Err(CoreError::Internal("mistral is down".into())),
            ProviderKind::OpenAiCompatible => Ok(vec![
                model("qwen2.5:7b-instruct", true, false),
                model("bge-m3", false, true),
            ]),
        }
    }
}

fn model_ref(provider: ProviderKind, model: &str) -> ModelRef {
    ModelRef {
        provider,
        model: model.into(),
    }
}

fn assignments(chat: ModelRef) -> ModelAssignments {
    ModelAssignments {
        extractor: chat.clone(),
        judge: chat.clone(),
        writer: chat,
        embeddings: model_ref(ProviderKind::OpenAiCompatible, "bge-m3"),
        executor: Default::default(),
    }
}

async fn configure_local_endpoint(usecase: &CapitainerieUseCase, organization_id: OrganizationId) {
    usecase
        .put_provider(PutProvider {
            subject: subject(organization_id, true),
            kind: ProviderKind::OpenAiCompatible,
            secret: SecretToken::new(""),
            base_url: Some("http://localhost:11434/v1".into()),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn a_key_is_stored_sealed_and_never_listed() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let admin = subject(organization_id, true);

    let stored = usecase
        .put_provider(anthropic(admin.clone()))
        .await
        .unwrap();
    let listed = usecase.list_providers(admin).await.unwrap();

    assert_eq!(stored.preview, "…a1b2");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].kind, ProviderKind::Anthropic);
    assert!(!format!("{listed:?}").contains("supersecret"));
    let ciphertext: Vec<u8> = sqlx::query_scalar(
        "SELECT ciphertext FROM provider_credentials WHERE organization_id = $1",
    )
    .bind(organization_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !ciphertext
            .windows(SECRET.len())
            .any(|window| window == SECRET.as_bytes())
    );
    let access = usecase
        .provider_access(organization_id, ProviderKind::Anthropic)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(access.secret.expose(), SECRET);
}

#[tokio::test]
async fn a_member_who_is_not_admin_is_refused_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let member = subject(organization_id, false);

    let put = usecase.put_provider(anthropic(member.clone())).await;
    let delete = usecase
        .delete_provider(member.clone(), ProviderKind::Anthropic)
        .await;
    let listed = usecase.list_providers(member).await.unwrap();

    assert!(matches!(put, Err(CoreError::Forbidden { .. })));
    assert!(matches!(delete, Err(CoreError::Forbidden { .. })));
    assert!(listed.is_empty());
    let denials: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'provider.manage' AND NOT allowed",
    )
    .bind(organization_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denials, 2);
}

#[tokio::test]
async fn replacing_and_deleting_a_provider() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let admin = subject(organization_id, true);

    usecase
        .put_provider(anthropic(admin.clone()))
        .await
        .unwrap();
    usecase
        .put_provider(PutProvider {
            secret: SecretToken::new("sk-ant-other-key-9z9z"),
            ..anthropic(admin.clone())
        })
        .await
        .unwrap();
    let access = usecase
        .provider_access(organization_id, ProviderKind::Anthropic)
        .await
        .unwrap()
        .unwrap();
    usecase
        .delete_provider(admin.clone(), ProviderKind::Anthropic)
        .await
        .unwrap();
    let again = usecase
        .delete_provider(admin, ProviderKind::Anthropic)
        .await;

    assert_eq!(access.secret.expose(), "sk-ant-other-key-9z9z");
    assert!(matches!(again, Err(CoreError::NotFound)));
    assert!(
        usecase
            .provider_access(organization_id, ProviderKind::Anthropic)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn invalid_providers_are_refused() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let admin = subject(organization_id, true);

    let no_key = usecase
        .put_provider(PutProvider {
            secret: SecretToken::new(""),
            ..anthropic(admin.clone())
        })
        .await;
    let no_url = usecase
        .put_provider(PutProvider {
            kind: ProviderKind::OpenAiCompatible,
            base_url: None,
            ..anthropic(admin)
        })
        .await;

    assert!(matches!(no_key, Err(CoreError::Invalid(_))));
    assert!(matches!(no_url, Err(CoreError::Invalid(_))));
}

#[tokio::test]
async fn without_a_secrets_key_nothing_can_be_stored() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone());

    let put = usecase
        .put_provider(anthropic(subject(organization_id, true)))
        .await;
    let access = usecase
        .provider_access(organization_id, ProviderKind::Anthropic)
        .await
        .unwrap();

    assert!(matches!(put, Err(CoreError::Internal(_))));
    assert!(access.is_none());
}

#[tokio::test]
async fn the_catalog_lists_configured_providers_and_reports_the_ones_that_fail() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let admin = subject(organization_id, true);
    usecase
        .put_provider(anthropic(admin.clone()))
        .await
        .unwrap();
    usecase
        .put_provider(PutProvider {
            kind: ProviderKind::Mistral,
            ..anthropic(admin.clone())
        })
        .await
        .unwrap();

    let listing = usecase
        .list_models(subject(organization_id, false), &FakeCatalog)
        .await
        .unwrap();
    let tested = usecase
        .test_provider(admin.clone(), ProviderKind::Anthropic, &FakeCatalog)
        .await
        .unwrap();
    let refused = usecase
        .test_provider(
            subject(organization_id, false),
            ProviderKind::Anthropic,
            &FakeCatalog,
        )
        .await;

    assert_eq!(listing.models.len(), 1);
    assert_eq!(listing.models[0].id, "claude-haiku-5-5");
    assert_eq!(listing.failures.len(), 1);
    assert_eq!(listing.failures[0].provider, ProviderKind::Mistral);
    assert_eq!(tested.len(), 1);
    assert!(matches!(refused, Err(CoreError::Forbidden { .. })));
}

#[tokio::test]
async fn assignments_are_validated_against_providers_and_the_catalog() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let admin = subject(organization_id, true);
    let haiku = assignments(model_ref(ProviderKind::Anthropic, "claude-haiku-5-5"));

    assert!(
        usecase
            .get_model_assignments(admin.clone())
            .await
            .unwrap()
            .is_none()
    );
    let unconfigured = usecase
        .put_model_assignments(admin.clone(), haiku.clone(), &FakeCatalog)
        .await;
    usecase
        .put_provider(anthropic(admin.clone()))
        .await
        .unwrap();
    configure_local_endpoint(&usecase, organization_id).await;
    let unknown_model = usecase
        .put_model_assignments(
            admin.clone(),
            assignments(model_ref(ProviderKind::Anthropic, "claude-nope")),
            &FakeCatalog,
        )
        .await;
    let chat_as_embeddings = usecase
        .put_model_assignments(
            admin.clone(),
            ModelAssignments {
                embeddings: model_ref(ProviderKind::OpenAiCompatible, "qwen2.5:7b-instruct"),
                ..haiku.clone()
            },
            &FakeCatalog,
        )
        .await;
    let non_admin = usecase
        .put_model_assignments(subject(organization_id, false), haiku.clone(), &FakeCatalog)
        .await;
    let stored = usecase
        .put_model_assignments(admin.clone(), haiku.clone(), &FakeCatalog)
        .await
        .unwrap();
    let read_by_member = usecase
        .get_model_assignments(subject(organization_id, false))
        .await
        .unwrap();

    assert!(matches!(unconfigured, Err(CoreError::Invalid(_))));
    assert!(matches!(unknown_model, Err(CoreError::Invalid(_))));
    assert!(matches!(chat_as_embeddings, Err(CoreError::Invalid(_))));
    assert!(matches!(non_admin, Err(CoreError::Forbidden { .. })));
    assert_eq!(stored, haiku);
    assert_eq!(read_by_member, Some(haiku.clone()));
    assert_eq!(
        usecase
            .model_assignments_for(organization_id)
            .await
            .unwrap(),
        Some(haiku)
    );
}

#[tokio::test]
async fn a_submitted_run_freezes_the_models_in_force() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let admin = subject(organization_id, true);
    let submit = |title: &str| SubmitTranscript {
        subject: subject(organization_id, false),
        title: title.into(),
        body: "Alice : je relance le fournisseur.".into(),
    };
    let models_of = |run_id: Uuid| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, Option<serde_json::Value>>(
                "SELECT models FROM runs WHERE id = $1",
            )
            .bind(run_id)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };

    let before = usecase.submit_transcript(submit("avant")).await.unwrap();
    usecase
        .put_provider(anthropic(admin.clone()))
        .await
        .unwrap();
    configure_local_endpoint(&usecase, organization_id).await;
    let haiku = assignments(model_ref(ProviderKind::Anthropic, "claude-haiku-5-5"));
    usecase
        .put_model_assignments(admin.clone(), haiku.clone(), &FakeCatalog)
        .await
        .unwrap();
    let during = usecase.submit_transcript(submit("pendant")).await.unwrap();
    let local = assignments(model_ref(
        ProviderKind::OpenAiCompatible,
        "qwen2.5:7b-instruct",
    ));
    usecase
        .put_model_assignments(admin, local.clone(), &FakeCatalog)
        .await
        .unwrap();

    assert_eq!(models_of(before.run_id.0).await, None);
    assert_eq!(
        models_of(during.run_id.0).await,
        Some(serde_json::to_value(&haiku).unwrap())
    );
    let after = usecase.submit_transcript(submit("après")).await.unwrap();
    assert_eq!(
        models_of(after.run_id.0).await,
        Some(serde_json::to_value(&local).unwrap())
    );
}
