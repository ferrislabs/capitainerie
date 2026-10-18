#![allow(clippy::unwrap_used)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{Json, Router, routing::post};
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        execution::{GithubAppCredentials, RepositoryRef},
    },
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_github::{GithubApps, GithubBases};
use chrono::Utc;
use serde_json::json;
use sqlx::types::Uuid;

const PEM: &str = include_str!("fixtures/test-only-app-key.pem");

async fn usecase() -> Option<CapitainerieUseCase> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => {
            eprintln!("skipped: DATABASE_URL is unset");
            return None;
        }
    };
    let pool = postgres::connect(&url, 2).await.unwrap();
    Some(
        CapitainerieUseCase::with_local_policies(pool)
            .with_secrets_cipher(SecretCipher::new(&[8u8; 32]).unwrap()),
    )
}

async fn organization() -> Subject {
    let id = Uuid::now_v7();
    let pool = postgres::connect(&std::env::var("DATABASE_URL").unwrap(), 1)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(format!("test-{}", id.simple()))
    .bind(format!("http://idp.test/realms/{id}"))
    .execute(&pool)
    .await
    .unwrap();
    Subject {
        id: "admin".into(),
        organization_id: OrganizationId(id),
        actor: None,
        is_admin: true,
    }
}

fn credentials(subject: &Subject) -> GithubAppCredentials {
    GithubAppCredentials {
        organization_id: subject.organization_id,
        app_id: (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64,
        slug: "capitainerie-test".into(),
        client_id: "Iv1.test".into(),
        client_secret: SecretToken::new("oauth-secret"),
        private_key_pem: SecretToken::new(PEM),
        webhook_secret: SecretToken::new("hook"),
        html_url: "https://github.com/apps/capitainerie-test".into(),
        owner_login: "acme".into(),
        owner_type: "Organization".into(),
        created_by: "admin".into(),
        created_at: Utc::now(),
    }
}

async fn fake_github(issued: Arc<AtomicUsize>) -> String {
    let app = Router::new().route(
        "/app/installations/{id}/access_tokens",
        post(move || {
            let issued = Arc::clone(&issued);
            async move {
                issued.fetch_add(1, Ordering::SeqCst);
                Json(json!({
                    "token": "ghs_fake",
                    "expires_at": (Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
                }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    base
}

fn apps(usecase: &CapitainerieUseCase, base: &str, ttl: Duration) -> GithubApps {
    GithubApps::new(
        usecase.clone(),
        GithubBases {
            api_base: base.to_owned(),
            web_base: base.to_owned(),
        },
    )
    .unwrap()
    .with_ttl(ttl)
}

#[tokio::test]
async fn an_organization_without_an_app_has_no_client() {
    let Some(usecase) = usecase().await else {
        return;
    };
    let subject = organization().await;
    let apps = apps(&usecase, "http://127.0.0.1:1", Duration::from_secs(300));

    assert!(
        apps.client(subject.organization_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn clients_of_one_app_share_their_installation_tokens() {
    let Some(usecase) = usecase().await else {
        return;
    };
    let subject = organization().await;
    usecase
        .save_github_app(subject.clone(), credentials(&subject))
        .await
        .unwrap();
    let issued = Arc::new(AtomicUsize::new(0));
    let base = fake_github(Arc::clone(&issued)).await;
    let apps = apps(&usecase, &base, Duration::ZERO);
    let repository: RepositoryRef = "acme/web".parse().unwrap();

    for _ in 0..3 {
        let client = apps.client(subject.organization_id).await.unwrap().unwrap();
        client.installation_token(5, &repository).await.unwrap();
    }

    assert_eq!(issued.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_deleted_app_is_noticed_once_the_cache_entry_expires() {
    let Some(usecase) = usecase().await else {
        return;
    };
    let subject = organization().await;
    usecase
        .save_github_app(subject.clone(), credentials(&subject))
        .await
        .unwrap();
    let cached = apps(&usecase, "http://127.0.0.1:1", Duration::from_secs(300));
    let expiring = apps(&usecase, "http://127.0.0.1:1", Duration::from_millis(50));
    assert!(
        cached
            .client(subject.organization_id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        expiring
            .client(subject.organization_id)
            .await
            .unwrap()
            .is_some()
    );

    usecase.delete_github_app(subject.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(80)).await;

    assert!(
        cached
            .client(subject.organization_id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        expiring
            .client(subject.organization_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn a_recreated_app_gets_its_own_client_and_tokens() {
    let Some(usecase) = usecase().await else {
        return;
    };
    let subject = organization().await;
    usecase
        .save_github_app(subject.clone(), credentials(&subject))
        .await
        .unwrap();
    let issued = Arc::new(AtomicUsize::new(0));
    let base = fake_github(Arc::clone(&issued)).await;
    let apps = apps(&usecase, &base, Duration::ZERO);
    let repository: RepositoryRef = "acme/web".parse().unwrap();
    let first = apps.client(subject.organization_id).await.unwrap().unwrap();
    first.installation_token(5, &repository).await.unwrap();

    usecase.delete_github_app(subject.clone()).await.unwrap();
    usecase
        .save_github_app(subject.clone(), credentials(&subject))
        .await
        .unwrap();
    let second = apps.client(subject.organization_id).await.unwrap().unwrap();
    second.installation_token(5, &repository).await.unwrap();

    assert_eq!(issued.load(Ordering::SeqCst), 2);
}
