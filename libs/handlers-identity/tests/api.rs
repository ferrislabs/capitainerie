#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, FerrisKeyVerifier, Identity};
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::agent::SecretToken,
    infrastructure::{crypto::SecretCipher, postgres},
};
use capitainerie_handlers::{AppState, Caller};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

struct Fixture {
    pool: PgPool,
    organization: Organization,
    state: AppState,
}

async fn fixture() -> Option<Fixture> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => return None,
    };
    let pool = postgres::connect(&url, 3).await.unwrap();
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = format!("http://idp.test/realms/{slug}");
    sqlx::query(
        "INSERT INTO organizations (id, slug, name, issuer_url) VALUES ($1, $2, 'Test', $3)",
    )
    .bind(id)
    .bind(&slug)
    .bind(&issuer_url)
    .execute(&pool)
    .await
    .unwrap();
    let organization = Organization {
        id: OrganizationId(id),
        slug: OrganizationSlug::try_from(slug).unwrap(),
        name: "Test".into(),
        issuer_url,
        created_at: Utc::now(),
    };
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap())
        .with_secrets_cipher(SecretCipher::new(&[2u8; 32]).unwrap());
    let state = AppState {
        usecase,
        verifier: FerrisKeyVerifier::new(AcceptedClients::new(["capitainerie-console"])),
        delegation: None,
    };
    Some(Fixture {
        pool,
        organization,
        state,
    })
}

impl Fixture {
    async fn serve(
        &self,
        subject: &str,
        display_name: Option<&str>,
        email: Option<&str>,
        agent: Option<&str>,
    ) -> String {
        let caller = Caller {
            identity: Identity {
                subject: subject.into(),
                issuer: self.organization.issuer_url.clone(),
                client_id: "capitainerie-console".into(),
                username: Some(format!("{subject}-login")),
                email: email.map(str::to_owned),
                display_name: display_name.map(str::to_owned),
                actor: agent.map(|client_id| capitainerie_auth::Actor {
                    sub: "service-account".into(),
                    client_id: Some(client_id.to_owned()),
                    act: None,
                }),
                roles: vec![],
            },
            organization: self.organization.clone(),
            token: SecretToken::new("user-token"),
        };
        let app: Router = capitainerie_handlers_identity::router()
            .layer(Extension(caller))
            .with_state(self.state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("http://{address}")
    }

    async fn stored(&self, subject: &str) -> Option<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT name, email FROM members WHERE organization_id = $1 AND subject = $2",
        )
        .bind(self.organization.id.0)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await
        .unwrap()
    }
}

async fn get(base: &str, path: &str) -> reqwest::Response {
    reqwest::get(format!("{base}{path}")).await.unwrap()
}

#[tokio::test]
async fn me_records_the_caller_as_a_member_and_refreshes_it_each_time() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let first = fixture
        .serve(
            "claire",
            Some("Claire Martin"),
            Some("claire@acme.test"),
            None,
        )
        .await;
    assert_eq!(get(&first, "/api/v1/me").await.status(), 200);
    assert_eq!(
        fixture.stored("claire").await,
        Some(("Claire Martin".into(), Some("claire@acme.test".into())))
    );

    let renamed = fixture
        .serve("claire", Some("Claire Durand"), None, None)
        .await;
    get(&renamed, "/api/v1/me").await;
    assert_eq!(
        fixture.stored("claire").await,
        Some(("Claire Durand".into(), None))
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM members WHERE organization_id = $1")
        .bind(fixture.organization.id.0)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn a_member_without_a_display_name_is_named_by_login_then_subject() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let base = fixture.serve("malik", None, None, None).await;
    get(&base, "/api/v1/me").await;
    assert_eq!(
        fixture.stored("malik").await,
        Some(("malik-login".into(), None))
    );
}

#[tokio::test]
async fn an_agent_calling_me_is_not_recorded_as_a_member() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let base = fixture
        .serve("bot", Some("Bot"), None, Some("capitainerie-agent-writer"))
        .await;
    assert_eq!(get(&base, "/api/v1/me").await.status(), 200);
    assert_eq!(fixture.stored("bot").await, None);
}

#[tokio::test]
async fn members_lists_the_organization_by_name_and_refuses_agents() {
    let Some(fixture) = fixture().await else {
        return;
    };
    for (subject, name) in [("b", "Bastien"), ("a", "Alice")] {
        let base = fixture.serve(subject, Some(name), None, None).await;
        get(&base, "/api/v1/me").await;
    }
    let base = fixture.serve("a", Some("Alice"), None, None).await;

    let body: Value = get(&base, "/api/v1/members").await.json().await.unwrap();
    let names: Vec<_> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Alice", "Bastien"]);
    assert_eq!(body["items"][0]["subject"], "a");

    let agent = fixture
        .serve("a", Some("Alice"), None, Some("capitainerie-agent-writer"))
        .await;
    assert_eq!(get(&agent, "/api/v1/members").await.status(), 403);
}
