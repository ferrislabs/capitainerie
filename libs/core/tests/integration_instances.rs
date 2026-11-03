#![allow(clippy::unwrap_used)]

use std::sync::Mutex;

use capitainerie_common::{ClientId, CoreError, IntegrationInstanceId, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        client::{
            Client,
            service::{CreateClient, UpdateClient},
        },
        integration::{
            BaseUrl, ConnectorFailure, Environment, InstanceConnection, InstanceError,
            InstanceName, IntegrationInstance, IntegrationKind, ProbeReport,
            action::ActionRequest,
            ports::{IntegrationConnector, IntegrationInstanceRepository},
            service::{CreateInstance, UpdateInstance},
        },
    },
    infrastructure::{
        crypto::SecretCipher, integration::postgres::PgIntegrationInstanceRepository, postgres,
    },
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

const KEY: &str = "sk-0123456789abcdef-supersecret-wxyz";

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

fn subject(organization_id: OrganizationId, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin,
    }
}

fn cipher() -> SecretCipher {
    SecretCipher::new(&[7u8; 32]).unwrap()
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone()).with_secrets_cipher(cipher())
}

async fn client(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    name: &str,
) -> Client {
    usecase
        .create_client(
            subject(organization_id, false),
            CreateClient {
                name: name.into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap()
}

fn report(version: &str) -> ProbeReport {
    ProbeReport {
        server_version: version.into(),
        warning: None,
    }
}

struct ScriptedConnector {
    outcome: Mutex<Result<ProbeReport, ConnectorFailure>>,
    probed: Mutex<Vec<InstanceConnection>>,
}

impl ScriptedConnector {
    fn answering(outcome: Result<ProbeReport, ConnectorFailure>) -> Self {
        Self {
            outcome: Mutex::new(outcome),
            probed: Mutex::new(Vec::new()),
        }
    }

    fn healthy() -> Self {
        Self::answering(Ok(report("19.0")))
    }

    fn will_answer(&self, outcome: Result<ProbeReport, ConnectorFailure>) {
        *self.outcome.lock().unwrap() = outcome;
    }

    fn probes(&self) -> usize {
        self.probed.lock().unwrap().len()
    }

    fn last_probe(&self) -> InstanceConnection {
        self.probed.lock().unwrap().last().cloned().unwrap()
    }
}

impl IntegrationConnector for ScriptedConnector {
    async fn probe(
        &self,
        connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        self.probed.lock().unwrap().push(connection.clone());
        self.outcome.lock().unwrap().clone()
    }
    async fn perform(
        &self,
        _connection: &InstanceConnection,
        _request: &ActionRequest,
    ) -> Result<serde_json::Value, ConnectorFailure> {
        Err(ConnectorFailure::NotSupported("not scripted".into()))
    }
}

fn odoo(client: &Client, name: &str) -> CreateInstance {
    CreateInstance {
        client_id: client.id,
        kind: "odoo_json2".into(),
        name: name.into(),
        environment: "production".into(),
        base_url: "https://odoo.acme.test/".into(),
        database: None,
        login: None,
        secret: SecretToken::new(KEY),
    }
}

async fn add(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    client: &Client,
    name: &str,
    connector: &ScriptedConnector,
) -> IntegrationInstance {
    usecase
        .create_integration_instance(
            subject(organization_id, true),
            odoo(client, name),
            connector,
        )
        .await
        .unwrap()
}

async fn opened(
    pool: &PgPool,
    organization_id: OrganizationId,
    id: IntegrationInstanceId,
) -> Result<Option<InstanceConnection>, CoreError> {
    let cipher = cipher();
    postgres::with_tx(pool, async |tx| {
        PgIntegrationInstanceRepository::new(&tx, Some(&cipher))
            .connection(organization_id, id)
            .await
    })
    .await
}

async fn rows(pool: &PgPool, client: &Client) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM integration_instances WHERE client_id = $1")
        .bind(client.id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_probed_instance_is_stored_sealed_listed_without_its_secret_and_opens_for_the_system() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();

    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;
    let listed = usecase
        .list_integration_instances(subject(organization_id, false), acme.id, false)
        .await
        .unwrap();

    assert_eq!(created.server_version.as_deref(), Some("19.0"));
    assert!(created.checked_at.is_some());
    assert_eq!(created.secret_preview, "…wxyz");
    assert_eq!(created.base_url.as_str(), "https://odoo.acme.test");
    assert_eq!(listed, vec![created.clone()]);
    assert!(!format!("{listed:?}").contains("supersecret"));
    let (nonce, ciphertext): (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT secret_nonce, secret_ciphertext FROM integration_instances WHERE id = $1",
    )
    .bind(created.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(nonce.len(), 12);
    assert!(
        !ciphertext
            .windows(KEY.len())
            .any(|window| window == KEY.as_bytes())
    );
    let connection = opened(&pool, organization_id, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(connection.secret.expose(), KEY);
    assert_eq!(connection.kind, IntegrationKind::OdooJson2);
    assert_eq!(connection.base_url.as_str(), "https://odoo.acme.test");
    assert_eq!(connection.instance_id, created.id);
    assert_eq!(connector.last_probe().secret.expose(), KEY);
}

#[tokio::test]
async fn a_sealed_secret_is_bound_to_its_own_row() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let first = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;
    let second = add(&usecase, organization_id, &acme, "Odoo staging", &connector).await;

    sqlx::query(
        "UPDATE integration_instances
         SET secret_nonce = src.secret_nonce, secret_ciphertext = src.secret_ciphertext
         FROM integration_instances src
         WHERE integration_instances.id = $1 AND src.id = $2",
    )
    .bind(second.id.0)
    .bind(first.id.0)
    .execute(&pool)
    .await
    .unwrap();

    assert!(
        opened(&pool, organization_id, first.id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(matches!(
        opened(&pool, organization_id, second.id).await,
        Err(CoreError::Internal(_))
    ));
}

#[tokio::test]
async fn an_instance_is_unknown_to_another_organization() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let other_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;

    let listed = usecase
        .list_integration_instances(subject(other_id, false), acme.id, false)
        .await;
    let checked = usecase
        .check_integration_instance(subject(other_id, true), created.id, &connector)
        .await;
    let archived = usecase
        .archive_integration_instance(subject(other_id, true), created.id)
        .await;
    let created_elsewhere = usecase
        .create_integration_instance(subject(other_id, true), odoo(&acme, "Odoo"), &connector)
        .await;

    assert!(matches!(listed, Err(CoreError::NotFound)));
    assert!(matches!(
        checked,
        Err(InstanceError::Core(CoreError::NotFound))
    ));
    assert!(matches!(archived, Err(CoreError::NotFound)));
    assert!(matches!(
        created_elsewhere,
        Err(InstanceError::Core(CoreError::NotFound))
    ));
    assert!(opened(&pool, other_id, created.id).await.unwrap().is_none());
    assert_eq!(connector.probes(), 1);
}

#[tokio::test]
async fn a_name_is_unique_per_client_and_free_for_another_client() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let globex = client(&usecase, organization_id, "Globex").await;
    let connector = ScriptedConnector::healthy();
    let first = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;

    let twice = usecase
        .create_integration_instance(
            subject(organization_id, true),
            odoo(&acme, "Odoo prod"),
            &connector,
        )
        .await;
    let elsewhere = add(&usecase, organization_id, &globex, "Odoo prod", &connector).await;
    let other = add(&usecase, organization_id, &acme, "Odoo staging", &connector).await;
    let renamed = usecase
        .update_integration_instance(
            subject(organization_id, true),
            other.id,
            UpdateInstance {
                name: Some("Odoo prod".into()),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await;
    let other_case = usecase
        .create_integration_instance(
            subject(organization_id, true),
            odoo(&acme, "odoo PROD"),
            &connector,
        )
        .await;
    usecase
        .archive_integration_instance(subject(organization_id, true), first.id)
        .await
        .unwrap();
    let after_archive = usecase
        .create_integration_instance(
            subject(organization_id, true),
            odoo(&acme, "Odoo prod"),
            &connector,
        )
        .await;

    assert!(matches!(twice, Err(InstanceError::NameTaken(name)) if name == "Odoo prod"));
    assert!(matches!(renamed, Err(InstanceError::NameTaken(name)) if name == "Odoo prod"));
    assert!(matches!(other_case, Err(InstanceError::NameTaken(_))));
    assert!(after_archive.is_ok());
    assert_eq!(elsewhere.client_id, globex.id);
    assert_eq!(rows(&pool, &acme).await, 3);
    assert_eq!(rows(&pool, &globex).await, 1);
}

#[tokio::test]
async fn an_instance_cannot_point_to_a_client_of_another_organization() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let other_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let foreign = client(&usecase, other_id, "Globex").await;
    let now = Utc::now();
    let instance = IntegrationInstance {
        id: IntegrationInstanceId::new(),
        organization_id,
        client_id: foreign.id,
        kind: IntegrationKind::N8n,
        name: InstanceName::try_from("n8n prod".to_owned()).unwrap(),
        environment: Environment::Production,
        base_url: BaseUrl::try_from("https://n8n.acme.test".to_owned()).unwrap(),
        database: None,
        login: None,
        secret_preview: "…".into(),
        server_version: None,
        checked_at: None,
        created_by: "user-1".into(),
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    let cipher = cipher();

    let stored = postgres::with_tx(&pool, async |tx| {
        PgIntegrationInstanceRepository::new(&tx, Some(&cipher))
            .insert(&instance, &SecretToken::new("n8n_api_key_value"))
            .await
    })
    .await;
    let raw = sqlx::query(
        "INSERT INTO integration_instances
            (id, organization_id, client_id, kind, name, environment, base_url,
             secret_nonce, secret_ciphertext, secret_preview, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, 'n8n', 'n8n', 'production', 'https://n8n.acme.test',
                 '\\x00', '\\x00', '…', 'user-1', now(), now())",
    )
    .bind(Uuid::now_v7())
    .bind(organization_id.0)
    .bind(foreign.id.0)
    .execute(&pool)
    .await
    .unwrap_err();

    assert!(matches!(stored, Err(CoreError::Invalid(_))), "{stored:?}");
    assert!(
        raw.as_database_error()
            .is_some_and(|error| error.is_foreign_key_violation())
    );
    assert_eq!(rows(&pool, &foreign).await, 0);
}

#[tokio::test]
async fn the_database_refuses_incoherent_credentials() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let insert = |kind: &'static str,
                  database: Option<&'static str>,
                  login: Option<&'static str>| {
        let pool = pool.clone();
        async move {
            sqlx::query(
                "INSERT INTO integration_instances
                    (id, organization_id, client_id, kind, name, environment, base_url, database, login,
                     secret_nonce, secret_ciphertext, secret_preview, created_by, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, 'production', 'https://x.test', $6, $7,
                         '\\x00', '\\x00', '…', 'user-1', now(), now())",
            )
            .bind(Uuid::now_v7())
            .bind(organization_id.0)
            .bind(acme.id.0)
            .bind(kind)
            .bind(Uuid::now_v7().to_string())
            .bind(database)
            .bind(login)
            .execute(&pool)
            .await
        }
    };

    assert!(insert("odoo_jsonrpc", None, None).await.is_err());
    assert!(insert("odoo_jsonrpc", Some("acme"), None).await.is_err());
    assert!(insert("n8n", Some("acme"), None).await.is_err());
    assert!(insert("n8n", None, Some("api")).await.is_err());
    assert!(insert("odoo", None, None).await.is_err());
    assert!(
        insert("odoo_jsonrpc", Some("acme"), Some("api"))
            .await
            .is_ok()
    );
    assert!(insert("odoo_json2", None, None).await.is_ok());
    assert!(insert("n8n", None, None).await.is_ok());
}

#[tokio::test]
async fn a_failed_probe_stores_nothing_on_creation() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector =
        ScriptedConnector::answering(Err(ConnectorFailure::VersionMismatch("no JSON-2".into())));

    let created = usecase
        .create_integration_instance(
            subject(organization_id, true),
            odoo(&acme, "Odoo 17"),
            &connector,
        )
        .await;

    assert!(matches!(
        created,
        Err(InstanceError::Connector(ConnectorFailure::VersionMismatch(
            _
        )))
    ));
    assert_eq!(connector.probes(), 1);
    assert_eq!(rows(&pool, &acme).await, 0);
}

#[cfg(feature = "mock")]
#[tokio::test]
async fn a_connector_that_fails_leaves_no_row_behind() {
    use capitainerie_core::domain::integration::ports::MockIntegrationConnector;

    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let mut connector = MockIntegrationConnector::new();
    connector
        .expect_probe()
        .times(1)
        .returning(|_| Box::pin(async { Err(ConnectorFailure::Unauthorized) }));

    let created = usecase
        .create_integration_instance(
            subject(organization_id, true),
            odoo(&acme, "Odoo prod"),
            &connector,
        )
        .await;

    assert!(matches!(
        created,
        Err(InstanceError::Connector(ConnectorFailure::Unauthorized))
    ));
    assert_eq!(rows(&pool, &acme).await, 0);
}

#[tokio::test]
async fn only_a_name_or_an_environment_change_skips_the_probe() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let admin = subject(organization_id, true);
    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;
    assert_eq!(connector.probes(), 1);

    let renamed = usecase
        .update_integration_instance(
            admin.clone(),
            created.id,
            UpdateInstance {
                name: Some("Odoo recette".into()),
                environment: Some("staging".into()),
                base_url: Some("https://odoo.acme.test/".into()),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await
        .unwrap();
    assert_eq!(connector.probes(), 1);
    assert_eq!(renamed.name.as_str(), "Odoo recette");
    assert_eq!(renamed.environment, Environment::Staging);
    assert_eq!(renamed.checked_at, created.checked_at);

    connector.will_answer(Ok(report("19.0+e")));
    let moved = usecase
        .update_integration_instance(
            admin,
            created.id,
            UpdateInstance {
                base_url: Some("https://odoo2.acme.test".into()),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await
        .unwrap();
    assert_eq!(connector.probes(), 2);
    assert_eq!(
        connector.last_probe().base_url.as_str(),
        "https://odoo2.acme.test"
    );
    assert_eq!(connector.last_probe().secret.expose(), KEY);
    assert_eq!(moved.server_version.as_deref(), Some("19.0+e"));
    assert!(moved.checked_at > created.checked_at);
    let connection = opened(&pool, organization_id, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(connection.base_url.as_str(), "https://odoo2.acme.test");
    assert_eq!(connection.secret.expose(), KEY);
}

#[tokio::test]
async fn a_failed_probe_keeps_the_previous_url_and_secret() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let admin = subject(organization_id, true);
    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;
    connector.will_answer(Err(ConnectorFailure::Blocked("10.0.0.5".into())));

    let moved = usecase
        .update_integration_instance(
            admin.clone(),
            created.id,
            UpdateInstance {
                base_url: Some("http://10.0.0.5".into()),
                secret: Some(SecretToken::new("a-brand-new-key-9999")),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await;

    assert!(matches!(
        moved,
        Err(InstanceError::Connector(ConnectorFailure::Blocked(_)))
    ));
    let listed = usecase
        .list_integration_instances(admin, acme.id, false)
        .await
        .unwrap();
    assert_eq!(listed, vec![created.clone()]);
    let connection = opened(&pool, organization_id, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(connection.base_url.as_str(), "https://odoo.acme.test");
    assert_eq!(connection.secret.expose(), KEY);
}

#[tokio::test]
async fn a_new_secret_is_probed_then_sealed_in_place_of_the_old_one() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;

    let rotated = usecase
        .update_integration_instance(
            subject(organization_id, true),
            created.id,
            UpdateInstance {
                secret: Some(SecretToken::new("a-brand-new-key-9999")),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await
        .unwrap();

    assert_eq!(
        connector.last_probe().secret.expose(),
        "a-brand-new-key-9999"
    );
    assert_eq!(rotated.secret_preview, "…9999");
    let connection = opened(&pool, organization_id, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(connection.secret.expose(), "a-brand-new-key-9999");
}

#[tokio::test]
async fn a_check_records_a_failure_and_keeps_the_known_version() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let admin = subject(organization_id, true);
    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;
    connector.will_answer(Err(ConnectorFailure::Unreachable("refused".into())));

    let (failed, outcome) = usecase
        .check_integration_instance(admin.clone(), created.id, &connector)
        .await
        .unwrap();

    assert!(matches!(outcome, Err(ConnectorFailure::Unreachable(_))));
    assert_eq!(failed.server_version.as_deref(), Some("19.0"));
    assert!(failed.checked_at > created.checked_at);
    connector.will_answer(Ok(report("19.0+e")));
    let (healthy, outcome) = usecase
        .check_integration_instance(admin.clone(), created.id, &connector)
        .await
        .unwrap();
    assert_eq!(outcome.unwrap().server_version, "19.0+e");
    assert_eq!(healthy.server_version.as_deref(), Some("19.0+e"));
    let listed = usecase
        .list_integration_instances(admin, acme.id, false)
        .await
        .unwrap();
    assert_eq!(listed, vec![healthy]);
}

#[tokio::test]
async fn an_archived_instance_is_hidden_unless_asked_for_and_can_no_longer_change() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let admin = subject(organization_id, true);
    let kept = add(&usecase, organization_id, &acme, "Odoo staging", &connector).await;
    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;

    let archived = usecase
        .archive_integration_instance(admin.clone(), created.id)
        .await
        .unwrap();
    let again = usecase
        .archive_integration_instance(admin.clone(), created.id)
        .await
        .unwrap();
    let active = usecase
        .list_integration_instances(admin.clone(), acme.id, false)
        .await
        .unwrap();
    let everything = usecase
        .list_integration_instances(admin.clone(), acme.id, true)
        .await
        .unwrap();
    let update = usecase
        .update_integration_instance(
            admin.clone(),
            created.id,
            UpdateInstance {
                name: Some("x".into()),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await;
    let check = usecase
        .check_integration_instance(admin, created.id, &connector)
        .await;

    assert!(archived.is_archived());
    assert_eq!(again.archived_at, archived.archived_at);
    assert_eq!(active, vec![kept.clone()]);
    assert_eq!(everything.len(), 2);
    assert!(matches!(
        update,
        Err(InstanceError::Core(CoreError::Invalid(_)))
    ));
    assert!(matches!(
        check,
        Err(InstanceError::Core(CoreError::Invalid(_)))
    ));
    assert_eq!(connector.probes(), 2);
}

#[tokio::test]
async fn an_archived_or_unknown_client_takes_no_instance() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    usecase
        .update_client(
            subject(organization_id, false),
            acme.id,
            UpdateClient {
                archived: Some(true),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();

    let archived = usecase
        .create_integration_instance(
            subject(organization_id, true),
            odoo(&acme, "Odoo prod"),
            &connector,
        )
        .await;
    let unknown = usecase
        .create_integration_instance(
            subject(organization_id, true),
            CreateInstance {
                client_id: ClientId::new(),
                ..odoo(&acme, "Odoo prod")
            },
            &connector,
        )
        .await;

    assert!(matches!(
        archived,
        Err(InstanceError::Core(CoreError::Invalid(_)))
    ));
    assert!(matches!(
        unknown,
        Err(InstanceError::Core(CoreError::NotFound))
    ));
    assert_eq!(connector.probes(), 0);
}

#[tokio::test]
async fn a_member_reads_but_only_an_admin_manages_and_the_refusals_are_audited() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let connector = ScriptedConnector::healthy();
    let created = add(&usecase, organization_id, &acme, "Odoo prod", &connector).await;
    let member = subject(organization_id, false);
    let agent = Subject {
        actor: Some("capitainerie-agent-writer".into()),
        ..member.clone()
    };

    let create = usecase
        .create_integration_instance(member.clone(), odoo(&acme, "Other"), &connector)
        .await;
    let update = usecase
        .update_integration_instance(
            member.clone(),
            created.id,
            UpdateInstance {
                name: Some("x".into()),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await;
    let check = usecase
        .check_integration_instance(member.clone(), created.id, &connector)
        .await;
    let archive = usecase
        .archive_integration_instance(member.clone(), created.id)
        .await;
    let read = usecase
        .list_integration_instances(member, acme.id, false)
        .await
        .unwrap();
    let by_agent = usecase
        .list_integration_instances(agent, acme.id, false)
        .await;

    for refused in [
        create.map(|_| ()),
        update.map(|_| ()),
        check.map(|_| ()),
        archive.map(|_| ()).map_err(InstanceError::from),
    ] {
        assert!(matches!(
            refused,
            Err(InstanceError::Core(CoreError::Forbidden { .. }))
        ));
    }
    assert_eq!(read.len(), 1);
    assert!(matches!(by_agent, Err(CoreError::Forbidden { .. })));
    assert_eq!(connector.probes(), 1);
    let denials: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions
         WHERE organization_id = $1 AND action = 'integration.manage' AND NOT allowed",
    )
    .bind(organization_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(denials, 4);
}

#[tokio::test]
async fn without_a_secrets_key_nothing_is_probed_or_stored() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let acme = client(&usecase, organization_id, "Acme").await;
    let created = add(
        &usecase,
        organization_id,
        &acme,
        "Odoo prod",
        &ScriptedConnector::healthy(),
    )
    .await;
    let keyless = CapitainerieUseCase::with_local_policies(pool.clone());
    let connector = ScriptedConnector::healthy();
    let admin = subject(organization_id, true);

    let create = keyless
        .create_integration_instance(admin.clone(), odoo(&acme, "Odoo staging"), &connector)
        .await;
    let rotate = keyless
        .update_integration_instance(
            admin.clone(),
            created.id,
            UpdateInstance {
                secret: Some(SecretToken::new("a-brand-new-key-9999")),
                ..UpdateInstance::default()
            },
            &connector,
        )
        .await;
    let check = keyless
        .check_integration_instance(admin.clone(), created.id, &connector)
        .await;
    let listed = keyless
        .list_integration_instances(admin, acme.id, false)
        .await
        .unwrap();

    assert!(matches!(
        create,
        Err(InstanceError::Core(CoreError::Internal(_)))
    ));
    assert!(matches!(
        rotate,
        Err(InstanceError::Core(CoreError::Internal(_)))
    ));
    assert!(matches!(
        check,
        Err(InstanceError::Core(CoreError::Internal(_)))
    ));
    assert_eq!(connector.probes(), 0);
    assert_eq!(listed, vec![created]);
}
