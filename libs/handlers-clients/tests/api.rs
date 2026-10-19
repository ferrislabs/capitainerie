#![allow(clippy::unwrap_used)]

use axum::{Extension, Router};
use capitainerie_auth::{AcceptedClients, Actor, FerrisKeyVerifier, Identity};
use capitainerie_common::OrganizationId;
use capitainerie_core::{
    CapitainerieUseCase, Organization, OrganizationSlug,
    domain::{agent::SecretToken, authorization::Subject},
    infrastructure::postgres,
};
use capitainerie_handlers::{AppState, Caller};
use chrono::Utc;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

fn database_url() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(url) => Some(url),
        Err(_) if std::env::var("REQUIRE_DATABASE_URL").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required when REQUIRE_DATABASE_URL=1")
        }
        Err(_) => None,
    }
}

struct Fixture {
    pool: PgPool,
    usecase: CapitainerieUseCase,
    organization: Organization,
    base: String,
}

async fn organization(pool: &PgPool) -> Organization {
    let id = Uuid::now_v7();
    let slug = format!("test-{}", id.simple());
    let issuer_url = format!("http://idp.test/realms/{slug}");
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
        created_at: Utc::now(),
    }
}

async fn fixture(actor: Option<&str>) -> Option<Fixture> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    let pool = postgres::connect(&url, 3).await.unwrap();
    let organization = organization(&pool).await;
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone());
    let state = AppState {
        usecase: usecase.clone(),
        verifier: FerrisKeyVerifier::new(AcceptedClients::new(["capitainerie-console"])),
        delegation: None,
    };
    let caller = Caller {
        identity: Identity {
            subject: "user-1".into(),
            issuer: organization.issuer_url.clone(),
            client_id: "capitainerie-console".into(),
            username: None,
            email: None,
            display_name: None,
            actor: actor.map(|client_id| Actor {
                sub: "service-account".into(),
                client_id: Some(client_id.to_owned()),
                act: None,
            }),
            roles: vec![],
        },
        organization: organization.clone(),
        token: SecretToken::new("user-token"),
    };
    let app: Router = capitainerie_handlers_clients::router()
        .layer(Extension(caller))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Some(Fixture {
        pool,
        usecase,
        organization,
        base: format!("http://{address}"),
    })
}

impl Fixture {
    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut request = reqwest::Client::new().request(method, format!("{}{path}", self.base));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        let body = response.json().await.unwrap_or(Value::Null);
        (status, body)
    }

    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, path, Some(body)).await
    }

    async fn patch(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::PATCH, path, Some(body)).await
    }

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::GET, path, None).await
    }

    fn subject(&self) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: self.organization.id,
            actor: None,
            is_admin: false,
        }
    }
}

fn names(list: &Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            item.get("client").or_else(|| item.get("project")).unwrap()["name"]
                .as_str()
                .unwrap()
        })
        .collect()
}

#[tokio::test]
async fn a_client_is_created_read_updated_and_archived() {
    let Some(fixture) = fixture(None).await else {
        return;
    };

    let (status, created) = fixture
        .post(
            "/api/v1/clients",
            json!({"name": " Acme ", "aliases": ["ACME SA"], "notes": "Historique"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["name"], "Acme");
    assert_eq!(created["aliases"], json!(["ACME SA"]));
    assert_eq!(created["archived"], false);
    let path = format!("/api/v1/clients/{}", created["id"].as_str().unwrap());

    let (_, list) = fixture.get("/api/v1/clients").await;
    assert_eq!(names(&list), ["Acme"]);
    assert_eq!(list["items"][0]["project_count"], 0);
    let (_, found) = fixture.get("/api/v1/clients?q=acme%20sa").await;
    assert_eq!(names(&found), ["Acme"]);
    let (_, missing) = fixture.get("/api/v1/clients?q=globex").await;
    assert!(names(&missing).is_empty());

    let (status, detail) = fixture.get(&path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["client"]["notes"], "Historique");
    assert_eq!(detail["projects"], json!([]));
    assert_eq!(detail["cards"].as_array().unwrap().len(), 5);
    assert_eq!(
        detail["cards"][0],
        json!({"column": "proposed", "cards": 0})
    );

    let (status, updated) = fixture
        .patch(&path, json!({"name": "Acme Industries", "notes": ""}))
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["name"], "Acme Industries");
    assert_eq!(updated["aliases"], json!(["ACME SA"]));
    assert_eq!(updated["notes"], "");

    let (status, archived) = fixture.patch(&path, json!({"archived": true})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(archived["archived"], true);
    let (_, active) = fixture.get("/api/v1/clients").await;
    assert!(names(&active).is_empty());
    let (_, only_archived) = fixture.get("/api/v1/clients?archived=true").await;
    assert_eq!(names(&only_archived), ["Acme Industries"]);
    let (_, restored) = fixture.patch(&path, json!({"archived": false})).await;
    assert_eq!(restored["archived"], false);
}

#[tokio::test]
async fn invalid_clients_are_refused_before_anything_is_stored() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    for body in [
        json!({"name": "Dupont, Jean"}),
        json!({"name": "   "}),
        json!({"name": "Acme", "aliases": ["a,b"]}),
        json!({"name": "Acme", "aliases": (0..11).map(|n| format!("A{n}")).collect::<Vec<_>>()}),
        json!({"name": "x".repeat(121)}),
        json!({"name": "Acme", "notes": "x".repeat(5001)}),
    ] {
        let (status, _) = fixture.post("/api/v1/clients", body.clone()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
    let (_, list) = fixture.get("/api/v1/clients").await;
    assert!(names(&list).is_empty());

    let (_, created) = fixture
        .post("/api/v1/clients", json!({"name": "Acme"}))
        .await;
    let path = format!("/api/v1/clients/{}", created["id"].as_str().unwrap());
    let (status, _) = fixture.patch(&path, json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .post("/api/v1/clients", json!({"name": "ACME"}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn projects_hang_under_a_client_and_clear_their_fields_with_null() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let (_, acme) = fixture
        .post("/api/v1/clients", json!({"name": "Acme"}))
        .await;
    let (_, globex) = fixture
        .post("/api/v1/clients", json!({"name": "Globex"}))
        .await;

    let (status, site) = fixture
        .post(
            "/api/v1/projects",
            json!({
                "client_id": acme["id"],
                "name": "Site",
                "repository": " acme/web ",
                "description": "Refonte"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{site}");
    assert_eq!(site["client"], json!({"id": acme["id"], "name": "Acme"}));
    assert_eq!(site["repository"], "acme/web");
    let path = format!("/api/v1/projects/{}", site["id"].as_str().unwrap());
    fixture
        .post(
            "/api/v1/projects",
            json!({"client_id": globex["id"], "name": "App"}),
        )
        .await;
    fixture
        .post("/api/v1/projects", json!({"name": "Interne"}))
        .await;

    let (_, all) = fixture.get("/api/v1/projects").await;
    assert_eq!(names(&all), ["App", "Interne", "Site"]);
    let (_, of_acme) = fixture
        .get(&format!(
            "/api/v1/projects?client_id={}",
            acme["id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(names(&of_acme), ["Site"]);
    let (_, detail) = fixture
        .get(&format!("/api/v1/clients/{}", acme["id"].as_str().unwrap()))
        .await;
    assert_eq!(detail["projects"][0]["name"], "Site");
    let (status, project_detail) = fixture.get(&path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(project_detail["project"]["name"], "Site");
    assert_eq!(project_detail["cards"].as_array().unwrap().len(), 5);

    let (status, moved) = fixture
        .patch(
            &path,
            json!({"client_id": globex["id"], "repository": null}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    assert_eq!(moved["client"]["name"], "Globex");
    assert_eq!(moved["repository"], Value::Null);
    assert_eq!(moved["description"], "Refonte");

    let (_, internal) = fixture
        .patch(&path, json!({"client_id": null, "name": "Site interne"}))
        .await;
    assert_eq!(internal["client"], Value::Null);
    assert_eq!(internal["name"], "Site interne");

    let (status, _) = fixture.patch(&path, json!({"archived": true})).await;
    assert_eq!(status, StatusCode::OK);
    let (_, active) = fixture.get("/api/v1/projects").await;
    assert_eq!(names(&active), ["App", "Interne"]);
    let (_, archived) = fixture.get("/api/v1/projects?archived=true").await;
    assert_eq!(names(&archived), ["Site interne"]);
}

#[tokio::test]
async fn invalid_projects_and_duplicates_are_refused() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let (_, acme) = fixture
        .post("/api/v1/clients", json!({"name": "Acme"}))
        .await;
    for body in [
        json!({"name": "Site", "repository": "../etc"}),
        json!({"name": "Site", "repository": "acme"}),
        json!({"name": "a,b"}),
        json!({"name": "Site", "client_id": Uuid::now_v7()}),
    ] {
        let (status, _) = fixture.post("/api/v1/projects", body.clone()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
    let body = json!({"name": "Site", "client_id": acme["id"]});
    assert_eq!(
        fixture.post("/api/v1/projects", body.clone()).await.0,
        StatusCode::CREATED
    );
    assert_eq!(
        fixture.post("/api/v1/projects", body).await.0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn another_organizations_records_are_unknown() {
    let Some(fixture) = fixture(None).await else {
        return;
    };
    let other = organization(&fixture.pool).await;
    let other_subject = Subject {
        organization_id: other.id,
        ..fixture.subject()
    };
    let foreign_client = fixture
        .usecase
        .create_client(
            other_subject.clone(),
            capitainerie_core::domain::client::service::CreateClient {
                name: "Globex".into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap();
    let foreign_project = fixture
        .usecase
        .create_project(
            other_subject,
            capitainerie_core::domain::project::service::CreateProject {
                client_id: Some(foreign_client.id),
                name: "App".into(),
                aliases: vec![],
                repository: None,
                description: String::new(),
            },
        )
        .await
        .unwrap();
    let client_path = format!("/api/v1/clients/{}", foreign_client.id.0);
    let project_path = format!("/api/v1/projects/{}", foreign_project.id.0);

    assert_eq!(fixture.get(&client_path).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        fixture.patch(&client_path, json!({"name": "x"})).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(fixture.get(&project_path).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        fixture.patch(&project_path, json!({"name": "x"})).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, clients) = fixture.get("/api/v1/clients").await;
    let (_, projects) = fixture.get("/api/v1/projects").await;
    assert!(names(&clients).is_empty());
    assert!(names(&projects).is_empty());
    let (_, filtered) = fixture
        .get(&format!(
            "/api/v1/projects?client_id={}",
            foreign_client.id.0
        ))
        .await;
    assert!(names(&filtered).is_empty());
    let (status, _) = fixture
        .post(
            "/api/v1/projects",
            json!({"name": "Site", "client_id": foreign_client.id.0}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, own) = fixture
        .post("/api/v1/projects", json!({"name": "Site"}))
        .await;
    let (status, _) = fixture
        .patch(
            &format!("/api/v1/projects/{}", own["id"].as_str().unwrap()),
            json!({"client_id": foreign_client.id.0}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn an_agent_is_refused_on_every_route() {
    let Some(fixture) = fixture(Some("capitainerie-agent-writer")).await else {
        return;
    };
    let client_id = Uuid::now_v7();
    let project_id = Uuid::now_v7();
    let routes = [
        (Method::GET, "/api/v1/clients".to_owned(), None),
        (
            Method::POST,
            "/api/v1/clients".to_owned(),
            Some(json!({"name": "Pirate"})),
        ),
        (Method::GET, format!("/api/v1/clients/{client_id}"), None),
        (
            Method::PATCH,
            format!("/api/v1/clients/{client_id}"),
            Some(json!({"name": "x"})),
        ),
        (Method::GET, "/api/v1/projects".to_owned(), None),
        (
            Method::POST,
            "/api/v1/projects".to_owned(),
            Some(json!({"name": "Pirate"})),
        ),
        (Method::GET, format!("/api/v1/projects/{project_id}"), None),
        (
            Method::PATCH,
            format!("/api/v1/projects/{project_id}"),
            Some(json!({"name": "x"})),
        ),
    ];
    for (method, path, body) in routes {
        let (status, _) = fixture.call(method.clone(), &path, body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM clients WHERE organization_id = $1")
        .bind(fixture.organization.id.0)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}
