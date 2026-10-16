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
        authorization::LocalPolicyEngine,
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
    }
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::new(pool.clone(), LocalPolicyEngine)
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
