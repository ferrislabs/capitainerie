#![allow(clippy::unwrap_used)]

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        provider::{
            ProviderAccess, ProviderKind,
            ports::{CatalogModel, ModelCatalog},
        },
        user_provider::service::PutUserProvider,
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

fn subject(organization_id: OrganizationId, id: &str, actor: Option<&str>) -> Subject {
    Subject {
        id: id.into(),
        organization_id,
        actor: actor.map(str::to_owned),
        is_admin: false,
    }
}

fn usecase(pool: &PgPool, key: u8) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[key; 32]).unwrap())
}

fn command(subject: Subject, kind: ProviderKind, is_default: Option<bool>) -> PutUserProvider {
    PutUserProvider {
        subject,
        kind,
        secret: Some(SecretToken::new(SECRET)),
        base_url: (kind == ProviderKind::OpenAiCompatible)
            .then(|| "https://generativelanguage.googleapis.com/v1beta/openai".into()),
        default_model: "some-model".into(),
        is_default,
    }
}

struct Recording;

impl ModelCatalog for Recording {
    async fn list_models(&self, access: &ProviderAccess) -> Result<Vec<CatalogModel>, CoreError> {
        assert_eq!(access.secret.expose(), SECRET);
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn a_key_is_sealed_at_rest_and_opens_only_for_its_owner() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool, 7);
    let alice = subject(organization_id, "alice", None);
    usecase
        .put_user_provider(command(alice.clone(), ProviderKind::Anthropic, None))
        .await
        .unwrap();

    let (ciphertext, preview): (Vec<u8>, String) = sqlx::query_as(
        "SELECT ciphertext, key_preview FROM user_provider_credentials WHERE organization_id = $1",
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
    assert_eq!(preview, "…a1b2");

    let opened = usecase
        .user_credentials_of(organization_id, "alice")
        .await
        .unwrap();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].secret.expose(), SECRET);
    assert!(opened[0].is_default);

    sqlx::query("UPDATE user_provider_credentials SET subject = 'bob' WHERE organization_id = $1")
        .bind(organization_id.0)
        .execute(&pool)
        .await
        .unwrap();
    let swapped = usecase.user_credentials_of(organization_id, "bob").await;
    assert!(
        swapped.is_err(),
        "a ciphertext moved to another user opened"
    );

    let other_key = self::usecase(&pool, 8);
    sqlx::query(
        "UPDATE user_provider_credentials SET subject = 'alice' WHERE organization_id = $1",
    )
    .bind(organization_id.0)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        other_key
            .user_credentials_of(organization_id, "alice")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn each_user_and_organization_has_its_own_connections() {
    let Some(pool) = pool().await else { return };
    let acme = organization(&pool).await;
    let other = organization(&pool).await;
    let usecase = usecase(&pool, 7);
    let alice = subject(acme, "alice", None);
    let bob = subject(acme, "bob", None);
    let outsider = subject(other, "alice", None);
    usecase
        .put_user_provider(command(alice.clone(), ProviderKind::Anthropic, None))
        .await
        .unwrap();

    assert!(
        usecase
            .list_user_providers(bob.clone())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        usecase
            .list_user_providers(outsider.clone())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        usecase
            .delete_user_provider(bob.clone(), ProviderKind::Anthropic)
            .await,
        Err(CoreError::NotFound)
    ));
    assert!(matches!(
        usecase
            .test_user_provider(outsider, ProviderKind::Anthropic, &Recording)
            .await,
        Err(CoreError::NotFound)
    ));
    assert!(
        usecase
            .user_credentials_of(acme, "bob")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        usecase
            .list_user_providers(alice.clone())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        usecase
            .test_user_provider(alice, ProviderKind::Anthropic, &Recording)
            .await
            .unwrap(),
        Ok(())
    );
}

#[tokio::test]
async fn an_agent_acting_for_the_user_cannot_touch_their_connections() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool, 7);
    let human = subject(organization_id, "alice", None);
    usecase
        .put_user_provider(command(human.clone(), ProviderKind::Anthropic, None))
        .await
        .unwrap();
    let agent = subject(organization_id, "alice", Some("agent:darnold"));

    assert!(matches!(
        usecase.list_user_providers(agent.clone()).await,
        Err(CoreError::Forbidden { .. })
    ));
    assert!(matches!(
        usecase
            .put_user_provider(command(agent.clone(), ProviderKind::Mistral, None))
            .await,
        Err(CoreError::Forbidden { .. })
    ));
    assert!(matches!(
        usecase
            .delete_user_provider(agent.clone(), ProviderKind::Anthropic)
            .await,
        Err(CoreError::Forbidden { .. })
    ));
    assert!(matches!(
        usecase
            .test_user_provider(agent, ProviderKind::Anthropic, &Recording)
            .await,
        Err(CoreError::Forbidden { .. })
    ));
    assert_eq!(usecase.list_user_providers(human).await.unwrap().len(), 1);
}

#[tokio::test]
async fn exactly_one_connection_is_the_default_and_a_delete_promotes_another() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool, 7);
    let alice = subject(organization_id, "alice", None);
    for (kind, default) in [
        (ProviderKind::Anthropic, None),
        (ProviderKind::Mistral, None),
        (ProviderKind::OpenAiCompatible, Some(true)),
    ] {
        usecase
            .put_user_provider(command(alice.clone(), kind, default))
            .await
            .unwrap();
    }
    let defaults = |listed: &[capitainerie_core::domain::user_provider::UserProviderSummary]| {
        listed
            .iter()
            .filter(|summary| summary.is_default)
            .map(|summary| summary.kind)
            .collect::<Vec<_>>()
    };
    let listed = usecase.list_user_providers(alice.clone()).await.unwrap();
    assert_eq!(defaults(&listed), [ProviderKind::OpenAiCompatible]);

    usecase
        .put_user_provider(command(alice.clone(), ProviderKind::Mistral, Some(true)))
        .await
        .unwrap();
    let listed = usecase.list_user_providers(alice.clone()).await.unwrap();
    assert_eq!(defaults(&listed), [ProviderKind::Mistral]);

    usecase
        .delete_user_provider(alice.clone(), ProviderKind::Mistral)
        .await
        .unwrap();
    let listed = usecase.list_user_providers(alice.clone()).await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(defaults(&listed).len(), 1);

    let second_default = sqlx::query(
        "UPDATE user_provider_credentials SET is_default = TRUE WHERE organization_id = $1 AND subject = 'alice'",
    )
    .bind(organization_id.0)
    .execute(&pool)
    .await;
    assert!(second_default.is_err(), "the index allows two defaults");
}

#[tokio::test]
async fn a_partial_update_keeps_the_sealed_key_and_its_binding() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool, 7);
    let alice = subject(organization_id, "alice", None);
    usecase
        .put_user_provider(command(alice.clone(), ProviderKind::Anthropic, None))
        .await
        .unwrap();
    usecase
        .put_user_provider(command(alice.clone(), ProviderKind::Mistral, None))
        .await
        .unwrap();

    let mut partial = command(alice.clone(), ProviderKind::Mistral, Some(true));
    partial.secret = None;
    partial.default_model = "another-model".into();
    let summary = usecase.put_user_provider(partial).await.unwrap();
    assert_eq!(summary.preview, "…a1b2");
    assert_eq!(summary.default_model, "another-model");
    assert!(summary.is_default);

    let opened = usecase
        .user_credentials_of(organization_id, "alice")
        .await
        .unwrap();
    assert_eq!(opened.len(), 2);
    for credential in &opened {
        assert_eq!(credential.secret.expose(), SECRET);
        assert_eq!(
            credential.is_default,
            credential.kind == ProviderKind::Mistral
        );
    }
    let (preview,): (String,) = sqlx::query_as(
        "SELECT key_preview FROM user_provider_credentials WHERE organization_id = $1 AND kind = 'mistral'",
    )
    .bind(organization_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(preview, "…a1b2");

    let mut missing = command(alice, ProviderKind::OpenAiCompatible, None);
    missing.secret = None;
    missing.base_url = None;
    assert!(matches!(
        usecase.put_user_provider(missing).await,
        Err(CoreError::Invalid(_))
    ));
}
