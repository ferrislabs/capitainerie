#![allow(clippy::unwrap_used)]

use std::sync::atomic::{AtomicUsize, Ordering};

use capitainerie_common::{CoreError, McpConnectorId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        mcp::{
            CodeExchange, Discovery, RefreshExchange, TokenSet,
            ports::McpOAuth,
            service::{CreateConnector, UpdateConnector},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

const REDIRECT: &str = "https://console.example/connectors/oauth/callback";

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
    Some(postgres::connect(&url, 3).await.unwrap())
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

fn user(organization_id: OrganizationId, id: &str) -> Subject {
    Subject {
        id: id.into(),
        organization_id,
        actor: None,
        is_admin: false,
    }
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[7u8; 32]).unwrap())
}

#[derive(Default)]
struct FakeOAuth {
    first_lifetime: i64,
    refreshes: AtomicUsize,
    exchanges: AtomicUsize,
    refresh_fails: bool,
}

impl McpOAuth for FakeOAuth {
    async fn discover(&self, url: &str) -> Result<Discovery, CoreError> {
        Ok(Discovery {
            resource: url.to_owned(),
            issuer: "https://auth.example/realms/mestier".into(),
            authorization_endpoint: "https://auth.example/realms/mestier/auth".into(),
            token_endpoint: "https://auth.example/realms/mestier/token".into(),
            registration_endpoint: "https://auth.example/realms/mestier/register".into(),
            scopes: vec!["mestier.read".into()],
        })
    }

    async fn register_client(
        &self,
        _: &str,
        redirect_uri: &str,
        _: &[String],
    ) -> Result<String, CoreError> {
        assert_eq!(redirect_uri, REDIRECT);
        Ok("client-1".into())
    }

    async fn exchange_code(&self, request: &CodeExchange<'_>) -> Result<TokenSet, CoreError> {
        assert_eq!(request.code, "the-code");
        assert_eq!(request.client_id, "client-1");
        assert_eq!(request.redirect_uri, REDIRECT);
        assert_eq!(request.code_verifier.expose().len(), 43);
        let call = self.exchanges.fetch_add(1, Ordering::SeqCst);
        Ok(TokenSet {
            access_token: SecretToken::new(format!("access-{call}")),
            refresh_token: Some(SecretToken::new("refresh-0")),
            expires_at: Some(Utc::now() + Duration::seconds(self.first_lifetime)),
            scope: Some("mestier.read".into()),
        })
    }

    async fn refresh(&self, request: &RefreshExchange<'_>) -> Result<TokenSet, CoreError> {
        let call = self.refreshes.fetch_add(1, Ordering::SeqCst) + 1;
        if self.refresh_fails {
            return Err(CoreError::Conflict("la connexion a expiré".into()));
        }
        assert_eq!(
            request.refresh_token.expose(),
            format!("refresh-{}", call - 1)
        );
        Ok(TokenSet {
            access_token: SecretToken::new(format!("refreshed-{call}")),
            refresh_token: Some(SecretToken::new(format!("refresh-{call}"))),
            expires_at: Some(Utc::now() + Duration::hours(1)),
            scope: None,
        })
    }
}

async fn connector(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    oauth: &FakeOAuth,
) -> McpConnectorId {
    usecase
        .create_mcp_connector(
            user(organization_id, "user-1"),
            CreateConnector {
                url: "https://demo.mestier.fr/mcp".into(),
                name: Some("Mestier".into()),
                slug: None,
            },
            REDIRECT,
            oauth,
        )
        .await
        .unwrap()
        .id
}

fn state_of(authorization_url: &str) -> String {
    url::Url::parse(authorization_url)
        .unwrap()
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned()
}

async fn connect(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    id: McpConnectorId,
    oauth: &FakeOAuth,
) {
    let subject = user(organization_id, "user-1");
    let start = usecase
        .start_mcp_authorization(subject.clone(), id, REDIRECT)
        .await
        .unwrap();
    usecase
        .complete_mcp_authorization(subject, &start.state, "the-code", oauth)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_state_is_single_use_bound_to_its_user_and_organization_and_expires() {
    let Some(pool) = pool().await else { return };
    let usecase = usecase(&pool);
    let organization_id = organization(&pool).await;
    let oauth = FakeOAuth {
        first_lifetime: 3600,
        ..FakeOAuth::default()
    };
    let id = connector(&usecase, organization_id, &oauth).await;
    let subject = user(organization_id, "user-1");

    let start = usecase
        .start_mcp_authorization(subject.clone(), id, REDIRECT)
        .await
        .unwrap();
    assert_eq!(state_of(&start.authorization_url), start.state);
    let stranger = usecase
        .complete_mcp_authorization(
            user(organization_id, "user-2"),
            &start.state,
            "the-code",
            &oauth,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(stranger, CoreError::Forbidden { .. }),
        "{stranger}"
    );
    let replay = usecase
        .complete_mcp_authorization(subject.clone(), &start.state, "the-code", &oauth)
        .await
        .unwrap_err();
    assert!(
        matches!(replay, CoreError::Invalid(_)),
        "a refused state is burnt"
    );
    assert_eq!(oauth.exchanges.load(Ordering::SeqCst), 0);

    let other_org = organization(&pool).await;
    let start = usecase
        .start_mcp_authorization(subject.clone(), id, REDIRECT)
        .await
        .unwrap();
    let foreign = usecase
        .complete_mcp_authorization(user(other_org, "user-1"), &start.state, "the-code", &oauth)
        .await
        .unwrap_err();
    assert!(matches!(foreign, CoreError::Invalid(_)), "{foreign}");

    let start = usecase
        .start_mcp_authorization(subject.clone(), id, REDIRECT)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE mcp_oauth_states SET expires_at = now() - interval '1 second' WHERE state = $1",
    )
    .bind(&start.state)
    .execute(&pool)
    .await
    .unwrap();
    let late = usecase
        .complete_mcp_authorization(subject.clone(), &start.state, "the-code", &oauth)
        .await
        .unwrap_err();
    assert!(matches!(late, CoreError::Invalid(_)), "{late}");

    let start = usecase
        .start_mcp_authorization(subject.clone(), id, REDIRECT)
        .await
        .unwrap();
    let completed = usecase
        .complete_mcp_authorization(subject.clone(), &start.state, "the-code", &oauth)
        .await
        .unwrap();
    assert_eq!(completed, id);
    let again = usecase
        .complete_mcp_authorization(subject, &start.state, "the-code", &oauth)
        .await
        .unwrap_err();
    assert!(matches!(again, CoreError::Invalid(_)));
    assert_eq!(oauth.exchanges.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn tokens_are_sealed_at_rest_and_bound_to_their_row() {
    let Some(pool) = pool().await else { return };
    let usecase = usecase(&pool);
    let organization_id = organization(&pool).await;
    let oauth = FakeOAuth {
        first_lifetime: 3600,
        ..FakeOAuth::default()
    };
    let id = connector(&usecase, organization_id, &oauth).await;
    connect(&usecase, organization_id, id, &oauth).await;

    let stored: (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT access_ciphertext, refresh_ciphertext FROM mcp_connections WHERE connector_id = $1",
    )
    .bind(id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    for ciphertext in [&stored.0, &stored.1] {
        let text = String::from_utf8_lossy(ciphertext);
        assert!(!text.contains("access-0") && !text.contains("refresh-0"));
    }
    let token = usecase
        .mcp_access_token(organization_id, id, "user-1", &oauth)
        .await
        .unwrap();
    assert_eq!(token.expose(), "access-0");
    assert_eq!(oauth.refreshes.load(Ordering::SeqCst), 0);

    sqlx::query("UPDATE mcp_connections SET subject = 'user-9' WHERE connector_id = $1")
        .bind(id.0)
        .execute(&pool)
        .await
        .unwrap();
    let moved = usecase
        .mcp_access_token(organization_id, id, "user-9", &oauth)
        .await
        .unwrap_err();
    assert!(matches!(moved, CoreError::Internal(_)), "{moved}");

    let other_key = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_secrets_cipher(SecretCipher::new(&[8u8; 32]).unwrap());
    sqlx::query("UPDATE mcp_connections SET subject = 'user-1' WHERE connector_id = $1")
        .bind(id.0)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        other_key
            .mcp_access_token(organization_id, id, "user-1", &oauth)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn an_expiring_token_is_refreshed_once_and_the_rotation_is_kept() {
    let Some(pool) = pool().await else { return };
    let usecase = usecase(&pool);
    let organization_id = organization(&pool).await;
    let oauth = FakeOAuth {
        first_lifetime: 30,
        ..FakeOAuth::default()
    };
    let id = connector(&usecase, organization_id, &oauth).await;
    connect(&usecase, organization_id, id, &oauth).await;

    let first = usecase
        .mcp_access_token(organization_id, id, "user-1", &oauth)
        .await
        .unwrap();
    assert_eq!(first.expose(), "refreshed-1");
    let second = usecase
        .mcp_access_token(organization_id, id, "user-1", &oauth)
        .await
        .unwrap();
    assert_eq!(second.expose(), "refreshed-1");
    assert_eq!(oauth.refreshes.load(Ordering::SeqCst), 1);

    sqlx::query("UPDATE mcp_connections SET expires_at = now() - interval '1 minute' WHERE connector_id = $1")
        .bind(id.0)
        .execute(&pool)
        .await
        .unwrap();
    let third = usecase
        .mcp_access_token(organization_id, id, "user-1", &oauth)
        .await
        .unwrap();
    assert_eq!(third.expose(), "refreshed-2");
    let scope: Option<String> =
        sqlx::query_scalar("SELECT scope FROM mcp_connections WHERE connector_id = $1")
            .bind(id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(scope.as_deref(), Some("mestier.read"));
}

#[tokio::test]
async fn a_refused_refresh_asks_to_reconnect_and_without_a_refresh_token_too() {
    let Some(pool) = pool().await else { return };
    let usecase = usecase(&pool);
    let organization_id = organization(&pool).await;
    let oauth = FakeOAuth {
        first_lifetime: 30,
        refresh_fails: true,
        ..FakeOAuth::default()
    };
    let id = connector(&usecase, organization_id, &oauth).await;
    connect(&usecase, organization_id, id, &oauth).await;

    let refused = usecase
        .mcp_access_token(organization_id, id, "user-1", &oauth)
        .await
        .unwrap_err();
    assert!(matches!(refused, CoreError::Conflict(_)), "{refused}");
    assert_eq!(oauth.refreshes.load(Ordering::SeqCst), 1);

    sqlx::query("UPDATE mcp_connections SET refresh_nonce = NULL, refresh_ciphertext = NULL WHERE connector_id = $1")
        .bind(id.0)
        .execute(&pool)
        .await
        .unwrap();
    let stuck = usecase
        .mcp_access_token(organization_id, id, "user-1", &oauth)
        .await
        .unwrap_err();
    assert!(matches!(stuck, CoreError::Conflict(_)));
    assert_eq!(oauth.refreshes.load(Ordering::SeqCst), 1);

    let missing = usecase
        .mcp_access_token(organization_id, id, "nobody", &oauth)
        .await
        .unwrap_err();
    assert!(matches!(missing, CoreError::Conflict(_)));
}

#[tokio::test]
async fn connections_are_per_user_listed_and_dropped_on_disconnect() {
    let Some(pool) = pool().await else { return };
    let usecase = usecase(&pool);
    let organization_id = organization(&pool).await;
    let oauth = FakeOAuth {
        first_lifetime: 3600,
        ..FakeOAuth::default()
    };
    let id = connector(&usecase, organization_id, &oauth).await;
    connect(&usecase, organization_id, id, &oauth).await;

    let mine = usecase
        .list_mcp_connectors(user(organization_id, "user-1"), None)
        .await
        .unwrap();
    assert_eq!(mine.len(), 1);
    assert!(mine[0].connection.as_ref().unwrap().expires_at.is_some());
    let theirs = usecase
        .list_mcp_connectors(user(organization_id, "user-2"), None)
        .await
        .unwrap();
    assert!(theirs[0].connection.is_none());

    usecase
        .disconnect_mcp_connector(user(organization_id, "user-1"), id)
        .await
        .unwrap();
    usecase
        .disconnect_mcp_connector(user(organization_id, "user-1"), id)
        .await
        .unwrap();
    let after = usecase
        .get_mcp_connector(user(organization_id, "user-1"), id)
        .await
        .unwrap();
    assert!(after.connection.is_none());
}

#[tokio::test]
async fn archived_connectors_cannot_start_an_authorization_and_other_organizations_see_nothing() {
    let Some(pool) = pool().await else { return };
    let usecase = usecase(&pool);
    let organization_id = organization(&pool).await;
    let oauth = FakeOAuth::default();
    let id = connector(&usecase, organization_id, &oauth).await;

    let duplicate = usecase
        .create_mcp_connector(
            user(organization_id, "user-1"),
            CreateConnector {
                url: "https://demo.mestier.fr/mcp".into(),
                name: Some("Mestier".into()),
                slug: None,
            },
            REDIRECT,
            &oauth,
        )
        .await
        .unwrap_err();
    assert!(matches!(duplicate, CoreError::Conflict(_)), "{duplicate}");

    let stranger = user(organization(&pool).await, "user-1");
    assert!(matches!(
        usecase.get_mcp_connector(stranger.clone(), id).await,
        Err(CoreError::NotFound)
    ));
    assert!(matches!(
        usecase
            .update_mcp_connector(
                stranger.clone(),
                id,
                UpdateConnector {
                    archived: Some(true),
                    ..UpdateConnector::default()
                }
            )
            .await,
        Err(CoreError::NotFound)
    ));
    assert!(matches!(
        usecase
            .start_mcp_authorization(stranger, id, REDIRECT)
            .await,
        Err(CoreError::NotFound)
    ));

    usecase
        .update_mcp_connector(
            user(organization_id, "user-1"),
            id,
            UpdateConnector {
                archived: Some(true),
                ..UpdateConnector::default()
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        usecase
            .start_mcp_authorization(user(organization_id, "user-1"), id, REDIRECT)
            .await,
        Err(CoreError::Conflict(_))
    ));
    assert!(
        usecase
            .list_mcp_connectors(user(organization_id, "user-1"), None)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn agents_cannot_touch_connectors_and_a_missing_key_refuses_to_store_tokens() {
    let Some(pool) = pool().await else { return };
    let usecase = usecase(&pool);
    let organization_id = organization(&pool).await;
    let oauth = FakeOAuth::default();
    let id = connector(&usecase, organization_id, &oauth).await;
    let agent = Subject {
        actor: Some("capitainerie-agent-writer".into()),
        ..user(organization_id, "user-1")
    };

    assert!(matches!(
        usecase.list_mcp_connectors(agent.clone(), None).await,
        Err(CoreError::Forbidden { .. })
    ));
    assert!(matches!(
        usecase
            .start_mcp_authorization(agent.clone(), id, REDIRECT)
            .await,
        Err(CoreError::Forbidden { .. })
    ));
    assert!(matches!(
        usecase
            .create_mcp_connector(
                agent,
                CreateConnector {
                    url: "https://x.example/mcp".into(),
                    name: None,
                    slug: None
                },
                REDIRECT,
                &oauth
            )
            .await,
        Err(CoreError::Forbidden { .. })
    ));

    let keyless = CapitainerieUseCase::with_local_policies(pool.clone());
    assert!(!keyless.secrets_configured());
    let refused = keyless
        .start_mcp_authorization(user(organization_id, "user-1"), id, REDIRECT)
        .await
        .unwrap_err();
    assert!(matches!(refused, CoreError::Internal(_)));
    let states: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mcp_oauth_states WHERE connector_id = $1")
            .bind(id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(states, 0);
}
