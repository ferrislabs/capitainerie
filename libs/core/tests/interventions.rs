#![allow(clippy::unwrap_used)]

use capitainerie_common::{
    CardId, ClientId, CoreError, IntegrationInstanceId, OrganizationId, RunId,
};
use capitainerie_core::{
    CapitainerieUseCase,
    application::StartedIntervention,
    domain::{
        agent::SecretToken,
        authorization::Subject,
        card::link::LinkCard,
        card::{
            Card, Column,
            service::{CreateCard, MoveCard},
        },
        client::{Client, links::Change, service::CreateClient},
        conversation::CAPTAIN_AGENT,
        execution::EXECUTOR_AGENT,
        integration::{
            ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport,
            action::{Access, ActionRequest},
            intervention::{ConfideCard, Intervention, InterventionError},
            journal::{Grant, INTEGRATOR_AGENT},
            ports::IntegrationConnector,
            service::CreateInstance,
        },
        provider::ModelAssignments,
        run::{
            RunSubject, StepKind,
            step::{Step, StepStatus},
        },
    },
    infrastructure::{crypto::SecretCipher, postgres},
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

async fn setup() -> Option<(PgPool, CapitainerieUseCase, OrganizationId)> {
    let pool = postgres::connect(
        &database_url().or_else(|| {
            eprintln!("skipped: DATABASE_URL is unset");
            None
        })?,
        3,
    )
    .await
    .unwrap();
    let organization_id = organization(&pool).await;
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone())
        .with_cipher(SecretCipher::new(&[1u8; 32]).unwrap())
        .with_secrets_cipher(SecretCipher::new(&[7u8; 32]).unwrap());
    Some((pool, usecase, organization_id))
}

fn human(organization_id: OrganizationId, is_admin: bool) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin,
    }
}

fn acting_as(organization_id: OrganizationId, actor: &str) -> Subject {
    Subject {
        actor: Some(actor.into()),
        ..human(organization_id, false)
    }
}

struct Healthy;

impl IntegrationConnector for Healthy {
    async fn probe(
        &self,
        _connection: &InstanceConnection,
    ) -> Result<ProbeReport, ConnectorFailure> {
        Ok(ProbeReport {
            server_version: "19.0".into(),
            warning: None,
        })
    }

    async fn perform(
        &self,
        _connection: &InstanceConnection,
        _request: &ActionRequest,
    ) -> Result<serde_json::Value, ConnectorFailure> {
        Err(ConnectorFailure::NotSupported("not scripted".into()))
    }
}

async fn client(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    name: &str,
) -> Client {
    usecase
        .create_client(
            human(organization_id, false),
            CreateClient {
                name: name.into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap()
}

async fn instance(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    client: &Client,
    name: &str,
    environment: &str,
) -> IntegrationInstance {
    usecase
        .create_integration_instance(
            human(organization_id, true),
            CreateInstance {
                client_id: client.id,
                kind: "odoo_json2".into(),
                name: name.into(),
                environment: environment.into(),
                base_url: "https://odoo.acme.test".into(),
                database: None,
                login: None,
                secret: SecretToken::new(KEY),
            },
            &Healthy,
        )
        .await
        .unwrap()
}

async fn todo_card(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    client: Option<&Client>,
) -> Card {
    let proposed = usecase
        .create_card_linked(
            human(organization_id, false),
            CreateCard {
                title: "Vérifier les factures d'avril".into(),
                description: "Dans l'Odoo du client".into(),
                provenance: None,
                run_id: None,
                attributes: Default::default(),
            },
            client.map(|client| client.id),
            None,
        )
        .await
        .unwrap();
    usecase
        .move_card(
            human(organization_id, false),
            proposed.id,
            MoveCard {
                to: Column::Todo,
                expected_version: proposed.version,
                run_id: None,
            },
        )
        .await
        .unwrap()
}

#[derive(sqlx::FromRow)]
struct RunRow {
    status: String,
    pipeline: Vec<String>,
    repository: Option<String>,
    base_branch: Option<String>,
    transcript_id: Option<Uuid>,
    card_id: Option<Uuid>,
}

fn confide(card: &Card, grants: Vec<(IntegrationInstanceId, Access)>) -> ConfideCard {
    ConfideCard {
        instructions: "  Ne touche pas aux factures payées  ".into(),
        grants,
        confirm_production_writes: false,
        expected_version: card.version,
    }
}

async fn hand_over(
    usecase: &CapitainerieUseCase,
    subject: Subject,
    card: &Card,
    command: ConfideCard,
) -> Result<StartedIntervention, InterventionError> {
    usecase
        .confide_card_to_integrator(subject, card.id, command)
        .await
}

fn refusal(error: InterventionError) -> &'static str {
    match error {
        InterventionError::Refused(refusal) => refusal.code(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn core(error: InterventionError) -> CoreError {
    match error {
        InterventionError::Core(error) => error,
        other => panic!("expected a core error, got {other:?}"),
    }
}

async fn runs_of(pool: &PgPool, card_id: CardId) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM runs WHERE card_id = $1")
        .bind(card_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn interventions_of(pool: &PgPool, card_id: CardId) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM interventions WHERE card_id = $1")
        .bind(card_id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn grants_in(pool: &PgPool, organization_id: OrganizationId) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM intervention_grants g JOIN interventions i ON i.run_id = g.run_id WHERE i.organization_id = $1",
    )
    .bind(organization_id.0)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn column_of(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    card_id: CardId,
) -> Column {
    usecase
        .card_detail(human(organization_id, false), card_id)
        .await
        .unwrap()
        .card
        .column
}

async fn nothing_was_written(
    pool: &PgPool,
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    card: &Card,
) {
    assert_eq!(runs_of(pool, card.id).await, 0);
    assert_eq!(interventions_of(pool, card.id).await, 0);
    assert_eq!(grants_in(pool, organization_id).await, 0);
    assert_eq!(
        column_of(usecase, organization_id, card.id).await,
        Column::Todo
    );
    let moved: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE partition_key = $1 AND kind = 'card.moved'",
    )
    .bind(card.id.0)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(moved, 1, "only the move that brought it to todo");
}

#[tokio::test]
async fn a_member_hands_a_validated_card_to_the_integrator() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let production = instance(&usecase, organization_id, &acme, "Odoo prod", "production").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;

    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(
            &card,
            vec![
                (staging.id, Access::Write),
                (production.id, Access::Read),
                (staging.id, Access::Read),
            ],
        ),
    )
    .await
    .unwrap();

    assert_eq!(started.card.column, Column::InProgress);
    assert_eq!(started.card.id, card.id);
    assert_eq!(started.card.version, card.version + 1);
    let row: RunRow = sqlx::query_as(
        "SELECT status, pipeline, repository, base_branch, transcript_id, card_id FROM runs WHERE id = $1",
    )
    .bind(started.run_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.status, "queued");
    assert_eq!(row.pipeline, ["intervene"]);
    assert_eq!(
        (row.repository, row.base_branch, row.transcript_id),
        (None, None, None)
    );
    assert_eq!(row.card_id, Some(card.id.0));

    let overview = usecase
        .get_run(human(organization_id, false), started.run_id)
        .await
        .unwrap();
    assert_eq!(
        overview.run.subject,
        RunSubject::Intervention { card_id: card.id }
    );
    assert_eq!(
        overview.title.as_deref(),
        Some("Vérifier les factures d'avril")
    );

    let stored = usecase
        .intervention_of(organization_id, started.run_id)
        .await
        .unwrap()
        .unwrap();
    let mut expected_grants = vec![
        Grant {
            instance_id: staging.id,
            access: Access::Write,
        },
        Grant {
            instance_id: production.id,
            access: Access::Read,
        },
    ];
    expected_grants.sort_by_key(|grant| grant.instance_id.0);
    assert_eq!(
        stored,
        Intervention {
            run_id: started.run_id,
            organization_id,
            card_id: card.id,
            client_id: acme.id,
            instructions: "Ne touche pas aux factures payées".into(),
            grants: expected_grants,
            created_by: "user-1".into(),
            created_at: stored.created_at,
        }
    );

    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM outbox_events WHERE partition_key = $1")
            .bind(started.run_id.0)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(kinds, ["run.queued"]);
    let moved: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE partition_key = $1 AND kind = 'card.moved'",
    )
    .bind(card.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(moved, 2);
    let delegated: i64 =
        sqlx::query_scalar("SELECT count(*) FROM run_delegations WHERE run_id = $1")
            .bind(started.run_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(delegated, 0, "the integrator never holds the user's token");
}

#[tokio::test]
async fn the_client_is_frozen_when_the_card_is_handed_over() {
    let Some((_pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let globex = client(&usecase, organization_id, "Globex").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Write)]),
    )
    .await
    .unwrap();

    usecase
        .link_card(
            human(organization_id, false),
            card.id,
            LinkCard {
                expected_version: started.card.version,
                client: Change::Set(globex.id),
                project: Change::Keep,
            },
        )
        .await
        .unwrap();

    let stored = usecase
        .intervention_of(organization_id, started.run_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.client_id, acme.id);
    assert_ne!(stored.client_id, globex.id);
}

#[tokio::test]
async fn the_run_freezes_the_organization_models() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let assignments = ModelAssignments::defaults("chat", "embed");
    sqlx::query(
        "INSERT INTO model_assignments (organization_id, assignments, updated_by, updated_at) VALUES ($1, $2, 'u', now())",
    )
    .bind(organization_id.0)
    .bind(serde_json::to_value(&assignments).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;

    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap();

    let overview = usecase
        .get_run(human(organization_id, false), started.run_id)
        .await
        .unwrap();
    assert_eq!(overview.run.models, Some(assignments));
}

#[tokio::test]
async fn an_instance_of_another_client_is_refused_and_nothing_is_written() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let globex = client(&usecase, organization_id, "Globex").await;
    let odoo_prod = instance(&usecase, organization_id, &acme, "Odoo prod", "production").await;
    let card = todo_card(&usecase, organization_id, Some(&globex)).await;

    let error = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(odoo_prod.id, Access::Read)]),
    )
    .await
    .unwrap_err();

    assert_eq!(refusal(error), "instance_of_another_client");
    nothing_was_written(&pool, &usecase, organization_id, &card).await;
}

#[tokio::test]
async fn an_unknown_instance_or_one_of_another_organization_is_refused_alike() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let elsewhere = organization(&pool).await;
    let stranger = client(&usecase, elsewhere, "Acme").await;
    let foreign = instance(&usecase, elsewhere, &stranger, "Odoo prod", "production").await;

    for unknown in [IntegrationInstanceId::new(), foreign.id] {
        let error = hand_over(
            &usecase,
            human(organization_id, false),
            &card,
            confide(&card, vec![(unknown, Access::Read)]),
        )
        .await
        .unwrap_err();

        assert_eq!(refusal(error), "instance_of_another_client");
    }
    nothing_was_written(&pool, &usecase, organization_id, &card).await;
}

#[tokio::test]
async fn writing_to_production_needs_the_confirmation() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let n8n_prod = instance(&usecase, organization_id, &acme, "n8n prod", "production").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;

    let error = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(n8n_prod.id, Access::Write)]),
    )
    .await
    .unwrap_err();
    assert_eq!(refusal(error), "production_write_not_confirmed");
    nothing_was_written(&pool, &usecase, organization_id, &card).await;

    let confirmed = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        ConfideCard {
            confirm_production_writes: true,
            ..confide(&card, vec![(n8n_prod.id, Access::Write)])
        },
    )
    .await
    .unwrap();

    let stored = usecase
        .intervention_of(organization_id, confirmed.run_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.grants,
        [Grant {
            instance_id: n8n_prod.id,
            access: Access::Write
        }]
    );
}

#[tokio::test]
async fn the_other_refusals_name_their_code_and_write_nothing() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let archived = instance(&usecase, organization_id, &acme, "Odoo old", "staging").await;
    usecase
        .archive_integration_instance(human(organization_id, true), archived.id)
        .await
        .unwrap();
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let too_long = ConfideCard {
        instructions: "x".repeat(4001),
        ..confide(&card, vec![(staging.id, Access::Read)])
    };
    let cases = [
        (confide(&card, vec![]), "no_instance_granted"),
        (
            confide(&card, vec![(archived.id, Access::Read)]),
            "instance_archived",
        ),
        (too_long, "instructions_too_long"),
    ];

    for (command, code) in cases {
        let error = hand_over(&usecase, human(organization_id, false), &card, command)
            .await
            .unwrap_err();

        assert_eq!(refusal(error), code);
        nothing_was_written(&pool, &usecase, organization_id, &card).await;
    }
}

#[tokio::test]
async fn a_card_without_a_client_is_refused() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, None).await;

    let error = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap_err();

    assert_eq!(refusal(error), "card_without_client");
    nothing_was_written(&pool, &usecase, organization_id, &card).await;
}

#[tokio::test]
async fn a_card_that_is_not_todo_is_a_conflict() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let proposed = usecase
        .create_card_linked(
            human(organization_id, false),
            CreateCard {
                title: "Idée".into(),
                description: String::new(),
                provenance: None,
                run_id: None,
                attributes: Default::default(),
            },
            Some(acme.id),
            None,
        )
        .await
        .unwrap();

    let error = hand_over(
        &usecase,
        human(organization_id, false),
        &proposed,
        confide(&proposed, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap_err();

    assert!(matches!(core(error), CoreError::Conflict(code) if code == "card_not_executable"));
    assert_eq!(runs_of(&pool, proposed.id).await, 0);
    assert_eq!(interventions_of(&pool, proposed.id).await, 0);
}

#[tokio::test]
async fn a_card_has_one_active_intervention() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let first = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap();
    let back_to_todo = usecase
        .move_card(
            human(organization_id, false),
            card.id,
            MoveCard {
                to: Column::Todo,
                expected_version: first.card.version,
                run_id: None,
            },
        )
        .await
        .unwrap();

    let error = hand_over(
        &usecase,
        human(organization_id, false),
        &back_to_todo,
        confide(&back_to_todo, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap_err();

    assert!(matches!(core(error), CoreError::Conflict(code) if code == "execution_already_active"));
    assert_eq!(runs_of(&pool, card.id).await, 1);
    assert_eq!(interventions_of(&pool, card.id).await, 1);
    let duplicate = sqlx::query(
        "INSERT INTO runs (id, organization_id, card_id, initiated_by, status, pipeline, created_at)
         VALUES ($1, $2, $3, 'u', 'queued', '{intervene}', now())",
    )
    .bind(Uuid::now_v7())
    .bind(organization_id.0)
    .bind(card.id.0)
    .execute(&pool)
    .await;
    assert!(
        duplicate.is_err(),
        "the database refuses a second active run"
    );
}

#[tokio::test]
async fn a_stale_version_hands_nothing_over() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let command = ConfideCard {
        expected_version: card.version - 1,
        ..confide(&card, vec![(staging.id, Access::Read)])
    };

    let error = hand_over(&usecase, human(organization_id, false), &card, command)
        .await
        .unwrap_err();

    assert!(matches!(core(error), CoreError::Conflict(_)));
    nothing_was_written(&pool, &usecase, organization_id, &card).await;
}

#[tokio::test]
async fn a_card_of_another_organization_is_unknown() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let elsewhere = organization(&pool).await;

    let error = hand_over(
        &usecase,
        human(elsewhere, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap_err();

    assert!(matches!(core(error), CoreError::NotFound));
    nothing_was_written(&pool, &usecase, organization_id, &card).await;
}

#[tokio::test]
async fn an_agent_cannot_hand_a_card_over_and_the_refusal_is_audited() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;

    for actor in [EXECUTOR_AGENT, INTEGRATOR_AGENT, CAPTAIN_AGENT] {
        let error = hand_over(
            &usecase,
            acting_as(organization_id, actor),
            &card,
            confide(&card, vec![(staging.id, Access::Read)]),
        )
        .await
        .unwrap_err();

        assert!(
            matches!(core(error), CoreError::Forbidden { .. }),
            "{actor}"
        );
        let denied: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'card.intervene' AND actor = $2 AND NOT allowed",
        )
        .bind(organization_id.0)
        .bind(actor)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(denied, 1, "{actor}");
    }
    nothing_was_written(&pool, &usecase, organization_id, &card).await;
}

#[tokio::test]
async fn the_database_keeps_a_grant_on_the_client_of_its_intervention() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let globex = client(&usecase, organization_id, "Globex").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let other = instance(&usecase, organization_id, &acme, "Odoo dev", "development").await;
    let foreign = instance(
        &usecase,
        organization_id,
        &globex,
        "Odoo prod",
        "production",
    )
    .await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap();
    let insert = |client_id: ClientId, instance_id: IntegrationInstanceId| {
        let pool = pool.clone();
        let run_id: RunId = started.run_id;
        async move {
            sqlx::query(
                "INSERT INTO intervention_grants (run_id, client_id, instance_id, access) VALUES ($1, $2, $3, 'read')",
            )
            .bind(run_id.0)
            .bind(client_id.0)
            .bind(instance_id.0)
            .execute(&pool)
            .await
        }
    };

    let instance_of_another_client = insert(acme.id, foreign.id).await;
    let client_of_another_card = insert(globex.id, foreign.id).await;
    let same_client = insert(acme.id, other.id).await;

    assert!(
        instance_of_another_client.is_err(),
        "(client_id, instance_id) must exist in integration_instances"
    );
    assert!(
        client_of_another_card.is_err(),
        "(run_id, client_id) must exist in interventions"
    );
    assert!(same_client.is_ok());
    let stored = usecase
        .intervention_of(organization_id, started.run_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.grants.len(), 2);
}

#[tokio::test]
async fn the_database_refuses_an_intervention_for_a_client_of_another_organization() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let elsewhere = organization(&pool).await;
    let stranger = client(&usecase, elsewhere, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap();

    let moved = sqlx::query("UPDATE interventions SET client_id = $2 WHERE run_id = $1")
        .bind(started.run_id.0)
        .bind(stranger.id.0)
        .execute(&pool)
        .await;

    assert!(moved.is_err());
}

#[tokio::test]
async fn an_intervention_is_unknown_to_another_organization() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap();
    let elsewhere = organization(&pool).await;

    let foreign = usecase
        .intervention_of(elsewhere, started.run_id)
        .await
        .unwrap();
    let unknown = usecase
        .intervention_of(organization_id, RunId::new())
        .await
        .unwrap();

    assert_eq!(foreign, None);
    assert_eq!(unknown, None);
}

async fn running_intervene_step(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    started: &StartedIntervention,
) {
    let overview = usecase
        .get_run(human(organization_id, false), started.run_id)
        .await
        .unwrap();
    let step = Step {
        id: capitainerie_common::StepId::new(),
        run_id: started.run_id,
        organization_id,
        seq: 0,
        kind: StepKind::Intervene,
        agent: Some(INTEGRATOR_AGENT.into()),
        status: StepStatus::Running,
        attempt: 1,
        output: None,
        usage: Default::default(),
        cost_micros: 0,
        latency_ms: None,
        error: None,
        started_at: Utc::now(),
        finished_at: None,
    };
    usecase.record_step(overview.run, step).await.unwrap();
}

#[tokio::test]
async fn the_integrator_comments_once_as_the_human_acting_through_the_agent() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap();
    running_intervene_step(&usecase, organization_id, &started).await;

    let first = usecase
        .comment_card_as_integrator(started.run_id, card.id, "Lu : 12 factures.".into())
        .await
        .unwrap();
    let again = usecase
        .comment_card_as_integrator(started.run_id, card.id, "Lu deux fois.".into())
        .await
        .unwrap();

    assert_eq!(first, again);
    let comments: Vec<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT author_subject, author_actor, body FROM card_comments WHERE card_id = $1",
    )
    .bind(card.id.0)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        comments,
        [(
            "user-1".to_owned(),
            Some(INTEGRATOR_AGENT.to_owned()),
            "Lu : 12 factures.".to_owned()
        )]
    );
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE run_id = $1 AND action = 'card.comment' AND actor = $2 AND subject = 'user-1' AND allowed",
    )
    .bind(started.run_id.0)
    .bind(INTEGRATOR_AGENT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, 1, "the second call decides nothing");
    let stored: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT output FROM run_steps WHERE run_id = $1")
            .bind(started.run_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.unwrap()["comment_id"], first.0.to_string());
    assert_eq!(
        column_of(&usecase, organization_id, card.id).await,
        Column::InProgress,
        "the card stays in progress until a human moves it"
    );
}

#[tokio::test]
async fn the_integrator_comment_is_refused_for_another_card_or_an_unknown_run() {
    let Some((_pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let card = todo_card(&usecase, organization_id, Some(&acme)).await;
    let other = todo_card(&usecase, organization_id, Some(&acme)).await;
    let started = hand_over(
        &usecase,
        human(organization_id, false),
        &card,
        confide(&card, vec![(staging.id, Access::Read)]),
    )
    .await
    .unwrap();
    running_intervene_step(&usecase, organization_id, &started).await;

    let wrong_card = usecase
        .comment_card_as_integrator(started.run_id, other.id, "x".into())
        .await
        .unwrap_err();
    let unknown_run = usecase
        .comment_card_as_integrator(RunId::new(), card.id, "x".into())
        .await
        .unwrap_err();

    assert!(
        matches!(wrong_card, CoreError::Invalid(_)),
        "{wrong_card:?}"
    );
    assert!(
        matches!(unknown_run, CoreError::NotFound),
        "{unknown_run:?}"
    );
}

#[tokio::test]
async fn the_integrator_does_not_comment_for_the_run_of_an_execution() {
    let Some((_pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let card = todo_card(&usecase, organization_id, None).await;
    usecase
        .connect_github(
            human(organization_id, true),
            (Uuid::now_v7().as_u128() & 0x3FFF_FFFF_FFFF_FFFF) as i64,
            "acme".into(),
        )
        .await
        .unwrap();
    let execution = usecase
        .execute_card(
            human(organization_id, false),
            card.id,
            "acme/web".parse().unwrap(),
            "main".into(),
            card.version,
        )
        .await
        .unwrap();

    let error = usecase
        .comment_card_as_integrator(execution.run_id, card.id, "x".into())
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Invalid(_)), "{error:?}");
}

#[tokio::test]
async fn instances_are_read_by_id_within_the_organization_archived_ones_included() {
    let Some((pool, usecase, organization_id)) = setup().await else {
        return;
    };
    let acme = client(&usecase, organization_id, "Acme").await;
    let staging = instance(&usecase, organization_id, &acme, "Odoo staging", "staging").await;
    let old = instance(&usecase, organization_id, &acme, "Odoo old", "staging").await;
    usecase
        .archive_integration_instance(human(organization_id, true), old.id)
        .await
        .unwrap();
    let elsewhere = organization(&pool).await;
    let stranger = client(&usecase, elsewhere, "Acme").await;
    let foreign = instance(&usecase, elsewhere, &stranger, "Odoo prod", "production").await;

    let found = usecase
        .integration_instances_by_id(
            human(organization_id, false),
            &[
                staging.id,
                old.id,
                staging.id,
                foreign.id,
                IntegrationInstanceId::new(),
            ],
        )
        .await
        .unwrap();
    let refused = usecase
        .integration_instances_by_id(acting_as(organization_id, EXECUTOR_AGENT), &[staging.id])
        .await
        .unwrap_err();

    let ids: Vec<_> = found.iter().map(|found| found.id).collect();
    assert_eq!(ids, [staging.id, old.id]);
    assert!(found[1].is_archived());
    assert!(
        matches!(refused, CoreError::Forbidden { .. }),
        "{refused:?}"
    );
}
