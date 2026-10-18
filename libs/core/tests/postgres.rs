#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        authorization::Subject,
        event::ports::EventBus,
        run::{RunStatus, service::SubmitTranscript},
    },
    infrastructure::{
        event::{OutboxRelay, OutboxRelayConfig},
        postgres,
    },
};
use capitainerie_events::{EventEnvelope, run::RunQueued};
use sqlx::PgPool;
use uuid::Uuid;

/// Skips when `DATABASE_URL` is unset; `REQUIRE_DATABASE_URL=1` turns the skip
/// into a failure so CI cannot pass without a database.
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

fn member(organization_id: OrganizationId) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin: false,
    }
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
}

async fn outbox_kinds_for_run(pool: &PgPool, run_id: Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT kind FROM outbox_events WHERE partition_key = $1 ORDER BY id")
        .bind(run_id)
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn readiness_succeeds_against_a_live_database() {
    let Some(pool) = pool().await else { return };
    usecase(&pool).readiness().await.unwrap();
}

#[tokio::test]
async fn organization_by_issuer_finds_only_known_realms() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let issuer = format!("http://idp.test/realms/{}", organization_id.0);

    let found = usecase(&pool)
        .organization_by_issuer(&issuer)
        .await
        .unwrap();
    let unknown = usecase(&pool)
        .organization_by_issuer("http://idp.test/realms/unknown")
        .await;

    assert_eq!(found.id, organization_id);
    assert!(matches!(unknown, Err(CoreError::NotFound)));
}

#[tokio::test]
async fn submitting_a_transcript_queues_a_run_and_its_event_atomically() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;

    let submitted = usecase(&pool)
        .submit_transcript(SubmitTranscript {
            subject: member(organization_id),
            title: "Point hebdo".into(),
            body: "Alice : je relance le fournisseur.".into(),
        })
        .await
        .unwrap();

    let run = usecase(&pool)
        .get_run(member(organization_id), submitted.run_id)
        .await
        .unwrap();
    assert_eq!(run.run.status, RunStatus::Queued);
    assert_eq!(run.transcript_title, "Point hebdo");
    assert_eq!(
        outbox_kinds_for_run(&pool, submitted.run_id.0).await,
        ["run.queued"]
    );
}

#[tokio::test]
async fn a_rejected_submission_leaves_no_trace() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;

    let error = usecase(&pool)
        .submit_transcript(SubmitTranscript {
            subject: member(organization_id),
            title: "Point hebdo".into(),
            body: "   ".into(),
        })
        .await
        .unwrap_err();

    assert!(matches!(error, CoreError::Invalid(_)));
    let (runs, total) = usecase(&pool)
        .list_runs(member(organization_id), 10, 0)
        .await
        .unwrap();
    assert!(runs.is_empty());
    assert_eq!(total, 0);
}

#[tokio::test]
async fn runs_are_listed_newest_first_and_never_across_organizations() {
    let Some(pool) = pool().await else { return };
    let mine = organization(&pool).await;
    let theirs = organization(&pool).await;
    for title in ["Premier", "Second"] {
        usecase(&pool)
            .submit_transcript(SubmitTranscript {
                subject: member(mine),
                title: title.into(),
                body: "contenu".into(),
            })
            .await
            .unwrap();
    }

    let (runs, total) = usecase(&pool).list_runs(member(mine), 10, 0).await.unwrap();
    let (other, other_total) = usecase(&pool)
        .list_runs(member(theirs), 10, 0)
        .await
        .unwrap();
    let foreign = usecase(&pool).get_run(member(theirs), runs[0].run.id).await;

    assert_eq!(total, 2);
    assert_eq!(
        runs.iter()
            .map(|r| r.transcript_title.as_str())
            .collect::<Vec<_>>(),
        ["Second", "Premier"]
    );
    assert!(other.is_empty() && other_total == 0);
    assert!(matches!(foreign, Err(CoreError::NotFound)));
}

#[derive(Clone, Default)]
struct RecordingBus(Arc<Mutex<Vec<EventEnvelope>>>);

impl EventBus for RecordingBus {
    async fn publish(&self, events: &[EventEnvelope]) -> Result<(), CoreError> {
        self.0.lock().unwrap().extend_from_slice(events);
        Ok(())
    }
}

#[derive(Clone)]
struct FailingBus;

impl EventBus for FailingBus {
    async fn publish(&self, _events: &[EventEnvelope]) -> Result<(), CoreError> {
        Err(CoreError::Internal("bus down".into()))
    }
}

async fn published_at(pool: &PgPool, run_id: Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT published_at FROM outbox_events WHERE partition_key = $1")
        .bind(run_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn the_relay_publishes_committed_events_once_the_bus_accepts_them() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let submitted = usecase(&pool)
        .submit_transcript(SubmitTranscript {
            subject: member(organization_id),
            title: "Relais".into(),
            body: "contenu".into(),
        })
        .await
        .unwrap();
    let run_id = submitted.run_id.0;

    let failing = OutboxRelay::new(pool.clone(), FailingBus, OutboxRelayConfig::default());
    assert!(failing.publish_pending().await.is_err());
    assert!(published_at(&pool, run_id).await.is_none());

    let bus = RecordingBus::default();
    let relay = OutboxRelay::new(pool.clone(), bus.clone(), OutboxRelayConfig::default());
    while relay.publish_pending().await.unwrap() > 0 {}

    assert!(published_at(&pool, run_id).await.is_some());
    let published = bus.0.lock().unwrap().clone();
    let ours = published
        .iter()
        .find(|envelope| envelope.partition_key == run_id)
        .unwrap();
    let event: RunQueued = ours.open().unwrap().unwrap();
    assert_eq!(event.transcript_id, submitted.transcript_id.0);
    assert_eq!(event.organization_id, organization_id.0);
}

#[tokio::test]
async fn the_timeline_lists_what_the_run_did_not_its_reads() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let usecase = usecase(&pool);
    let submitted = usecase
        .submit_transcript(SubmitTranscript {
            subject: member(organization_id),
            title: "Point".into(),
            body: "Claire : Malik migre la CI.".into(),
        })
        .await
        .unwrap();
    for (action, kind) in [
        ("card.create", "card"),
        ("run.read", "run"),
        ("run.read", "run"),
        ("card.read", "card"),
        ("provider.read", "provider"),
        ("run.read", "run"),
    ] {
        sqlx::query(
            "INSERT INTO authorization_decisions (id, organization_id, subject, action, resource_kind, allowed, run_id) VALUES ($1, $2, 'user-1', $3, $4, true, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(organization_id.0)
        .bind(action)
        .bind(kind)
        .bind(submitted.run_id.0)
        .execute(&pool)
        .await
        .unwrap();
    }

    let timeline = usecase
        .run_timeline(member(organization_id), submitted.run_id)
        .await
        .unwrap();

    let actions: Vec<&str> = timeline
        .decisions
        .iter()
        .map(|decision| decision.action.as_str())
        .collect();
    assert!(actions.contains(&"card.create"));
    assert!(actions.iter().all(|action| !action.ends_with(".read")));
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE run_id = $1 AND action LIKE '%.read'",
    )
    .bind(submitted.run_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(audited >= 5);
}

mod cards {
    use capitainerie_common::CoreError;
    use capitainerie_core::domain::card::{
        Column,
        service::{CreateCard, MoveCard, ProvenanceInput, UpdateCard},
    };

    use super::*;

    const BODY: &str = "Claire : Malik migre la CI vers ARM avant fin octobre.";

    fn agent(organization_id: OrganizationId) -> Subject {
        Subject {
            actor: Some("capitainerie-agent-writer".into()),
            is_admin: false,
            ..member(organization_id)
        }
    }

    async fn decisions(pool: &PgPool, organization_id: OrganizationId) -> Vec<(String, bool)> {
        sqlx::query_as(
            "SELECT action, allowed FROM authorization_decisions WHERE organization_id = $1 AND actor IS NOT NULL ORDER BY decided_at, id",
        )
        .bind(organization_id.0)
        .fetch_all(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn an_agent_proposes_and_only_a_human_validates() {
        let Some(pool) = pool().await else { return };
        let organization_id = organization(&pool).await;
        let usecase = usecase(&pool);
        let submitted = usecase
            .submit_transcript(SubmitTranscript {
                subject: member(organization_id),
                title: "Point".into(),
                body: BODY.into(),
            })
            .await
            .unwrap();

        let card = usecase
            .create_card(
                agent(organization_id),
                CreateCard {
                    title: "Migrer la CI vers ARM".into(),
                    description: "Avant fin octobre".into(),
                    provenance: Some(ProvenanceInput {
                        transcript_id: submitted.transcript_id,
                        start: 9,
                        end: 35,
                        quote: "Malik migre la CI vers ARM".into(),
                    }),
                    run_id: Some(submitted.run_id),
                },
            )
            .await
            .unwrap();
        assert_eq!(card.column, Column::Proposed);

        let refused = usecase
            .move_card(
                agent(organization_id),
                card.id,
                MoveCard {
                    to: Column::Todo,
                    expected_version: card.version,
                    run_id: Some(submitted.run_id),
                },
            )
            .await;
        assert!(matches!(refused, Err(CoreError::Forbidden { .. })));

        let refined = usecase
            .update_card(
                agent(organization_id),
                card.id,
                UpdateCard {
                    title: None,
                    description: Some("Avant le 31 octobre, point mardi".into()),
                    expected_version: card.version,
                    run_id: Some(submitted.run_id),
                },
            )
            .await
            .unwrap();
        let validated = usecase
            .move_card(
                member(organization_id),
                card.id,
                MoveCard {
                    to: Column::Todo,
                    expected_version: refined.version,
                    run_id: None,
                },
            )
            .await
            .unwrap();
        let stale = usecase
            .move_card(
                member(organization_id),
                card.id,
                MoveCard {
                    to: Column::Done,
                    expected_version: refined.version,
                    run_id: None,
                },
            )
            .await;

        assert_eq!(validated.column, Column::Todo);
        assert_eq!(validated.version, 3);
        assert!(matches!(stale, Err(CoreError::Conflict(_))));
        assert_eq!(
            decisions(&pool, organization_id).await,
            [
                ("card.create".to_owned(), true),
                ("card.move".to_owned(), false),
                ("card.update".to_owned(), true),
            ]
        );
        let kinds: Vec<String> = sqlx::query_scalar(
            "SELECT kind FROM outbox_events WHERE partition_key = $1 ORDER BY id",
        )
        .bind(card.id.0)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(kinds, ["card.proposed", "card.updated", "card.moved"]);
    }

    #[tokio::test]
    async fn an_invented_quote_is_refused() {
        let Some(pool) = pool().await else { return };
        let organization_id = organization(&pool).await;
        let usecase = usecase(&pool);
        let submitted = usecase
            .submit_transcript(SubmitTranscript {
                subject: member(organization_id),
                title: "Point".into(),
                body: BODY.into(),
            })
            .await
            .unwrap();

        let error = usecase
            .create_card(
                agent(organization_id),
                CreateCard {
                    title: "Migrer la CI vers x86".into(),
                    description: String::new(),
                    provenance: Some(ProvenanceInput {
                        transcript_id: submitted.transcript_id,
                        start: 9,
                        end: 35,
                        quote: "Malik migre la CI vers x86".into(),
                    }),
                    run_id: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(error, CoreError::Invalid(_)));
        let (cards,) = (usecase.list_cards(member(organization_id)).await.unwrap(),);
        assert!(cards.is_empty());
    }
}
