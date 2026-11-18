#![allow(clippy::unwrap_used)]

use capitainerie_common::{CoreError, OrganizationId, ProjectId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{authorization::Subject, cost::Period},
    infrastructure::postgres,
};
use chrono::{DateTime, Duration, Utc};
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

fn admin(organization_id: OrganizationId) -> Subject {
    Subject {
        id: "admin-1".into(),
        organization_id,
        actor: None,
        is_admin: true,
    }
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
}

fn window() -> (DateTime<Utc>, DateTime<Utc>) {
    let to = Utc::now();
    (to - Duration::days(10), to)
}

fn period() -> Period {
    let (from, to) = window();
    Period::new(Some(from), Some(to), to).unwrap()
}

fn inside() -> DateTime<Utc> {
    Utc::now() - Duration::days(5)
}

async fn project(pool: &PgPool, org: OrganizationId, name: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO projects (id, organization_id, name, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, 'test', now(), now())",
    )
    .bind(id)
    .bind(org.0)
    .bind(name)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn agent(pool: &PgPool, org: OrganizationId, name: &str, archived: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO agents (id, organization_id, slug, name, archived, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, 'test', now(), now())",
    )
    .bind(id)
    .bind(org.0)
    .bind(format!("a-{}", id.simple()))
    .bind(name)
    .bind(archived)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn card(pool: &PgPool, org: OrganizationId, title: &str, project: Option<Uuid>) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO cards (id, organization_id, title, description, "column", created_by_subject,
                              version, created_at, updated_at, project_id)
           VALUES ($1, $2, $3, '', 'todo', 'test', 1, now(), now(), $4)"#,
    )
    .bind(id)
    .bind(org.0)
    .bind(title)
    .bind(project)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn transcript(pool: &PgPool, org: OrganizationId, project: Option<Uuid>) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO transcripts (id, organization_id, submitted_by, title, body, created_at, project_id)
         VALUES ($1, $2, 'test', 't', 'b', now(), $3)",
    )
    .bind(id)
    .bind(org.0)
    .bind(project)
    .execute(pool)
    .await
    .unwrap();
    id
}

struct Step<'a> {
    agent: Option<&'a str>,
    model: Option<&'a str>,
    cost: i64,
    at: DateTime<Utc>,
}

fn step(model: &'static str, cost: i64) -> Step<'static> {
    Step {
        agent: None,
        model: Some(model),
        cost,
        at: inside(),
    }
}

async fn steps(pool: &PgPool, org: OrganizationId, run: Uuid, steps: &[Step<'_>]) {
    for (seq, step) in steps.iter().enumerate() {
        sqlx::query(
            "INSERT INTO run_steps (id, run_id, organization_id, seq, kind, agent, status, attempt,
                                    model, input_tokens, output_tokens, cost_micros, started_at)
             VALUES ($1, $2, $3, $4, 'extract', $5, 'succeeded', 1, $6, $7, $8, $9, $10)",
        )
        .bind(Uuid::now_v7())
        .bind(run)
        .bind(org.0)
        .bind(seq as i32)
        .bind(step.agent)
        .bind(step.model)
        .bind(step.cost * 2)
        .bind(step.cost)
        .bind(step.cost)
        .bind(step.at)
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn pipeline_run(
    pool: &PgPool,
    org: OrganizationId,
    project: Option<Uuid>,
    user: &str,
    list: &[Step<'_>],
) -> Uuid {
    let transcript = transcript(pool, org, project).await;
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO runs (id, organization_id, transcript_id, initiated_by, status, pipeline, created_at)
         VALUES ($1, $2, $3, $4, 'succeeded', '{}', now())",
    )
    .bind(id)
    .bind(org.0)
    .bind(transcript)
    .bind(user)
    .execute(pool)
    .await
    .unwrap();
    steps(pool, org, id, list).await;
    id
}

async fn agent_run(
    pool: &PgPool,
    org: OrganizationId,
    card: Uuid,
    agent: Uuid,
    user: &str,
    list: &[Step<'_>],
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO runs (id, organization_id, card_id, initiated_by, status, pipeline, created_at)
         VALUES ($1, $2, $3, $4, 'succeeded', '{}', now())",
    )
    .bind(id)
    .bind(org.0)
    .bind(card)
    .bind(user)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO agent_tasks (run_id, organization_id, card_id, agent_id, allow_writes, created_by, created_at)
         VALUES ($1, $2, $3, $4, false, $5, now())",
    )
    .bind(id)
    .bind(org.0)
    .bind(card)
    .bind(agent)
    .bind(user)
    .execute(pool)
    .await
    .unwrap();
    steps(pool, org, id, list).await;
    id
}

#[tokio::test]
async fn projects_list_their_agents_and_a_projectless_row() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    let (p1, p2) = (
        project(&pool, org, "Alpha").await,
        project(&pool, org, "Beta").await,
    );
    let (ann, bob) = (
        agent(&pool, org, "Ann", false).await,
        agent(&pool, org, "Bob", false).await,
    );
    let c1 = card(&pool, org, "one", Some(p1)).await;
    let c2 = card(&pool, org, "two", Some(p2)).await;
    agent_run(&pool, org, c1, ann, "u", &[step("m", 100)]).await;
    agent_run(&pool, org, c1, bob, "u", &[step("m", 50)]).await;
    agent_run(&pool, org, c2, ann, "u", &[step("m", 30)]).await;
    pipeline_run(&pool, org, None, "u", &[step("m", 7)]).await;
    pipeline_run(&pool, org, Some(p2), "u", &[step("m", 20)]).await;

    let report = usecase(&pool)
        .project_costs(admin(org), period())
        .await
        .unwrap();

    assert_eq!(report.rows.len(), 3);
    assert_eq!(report.total.cost_micros, 207);
    assert_eq!(report.rows[1].project_id, Some(ProjectId(p2)));
    assert_eq!(report.rows[1].spent.cost_micros, 50);
    assert_eq!(report.rows[0].project_id, Some(ProjectId(p1)));
    assert_eq!(report.rows[0].name.as_deref(), Some("Alpha"));
    assert_eq!(report.rows[0].agents.len(), 2);
    assert_eq!(report.rows[2].project_id, None);
    for row in &report.rows {
        let sum: i64 = row.agents.iter().map(|a| a.spent.cost_micros).sum();
        assert_eq!(sum, row.spent.cost_micros);
    }
}

#[tokio::test]
async fn agents_cover_agent_tasks_and_pipeline_roles() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    let ann = agent(&pool, org, "Ann", false).await;
    let c = card(&pool, org, "one", None).await;
    agent_run(&pool, org, c, ann, "u", &[step("m", 40)]).await;
    let extractor = Step {
        agent: Some("extractor"),
        ..step("m", 10)
    };
    pipeline_run(&pool, org, None, "u", &[extractor]).await;

    let report = usecase(&pool)
        .agent_costs(admin(org), period())
        .await
        .unwrap();

    let names: Vec<_> = report.rows.iter().map(|r| r.agent.name.as_str()).collect();
    assert_eq!(names, ["Ann", "extractor"]);
    assert!(report.rows[0].agent.id.is_some());
    assert!(report.rows[1].agent.id.is_none());
    assert_eq!(report.rows[0].runs, 1);
    assert_eq!(report.total.cost_micros, 50);
}

#[tokio::test]
async fn a_task_adds_every_run_of_its_card() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    let (ann, bob) = (
        agent(&pool, org, "Ann", false).await,
        agent(&pool, org, "Bob", false).await,
    );
    let c = card(&pool, org, "Fix it", None).await;
    agent_run(&pool, org, c, ann, "u", &[step("m", 60)]).await;
    agent_run(&pool, org, c, bob, "u", &[step("m", 25)]).await;

    let report = usecase(&pool)
        .task_costs(admin(org), period(), None)
        .await
        .unwrap();

    assert_eq!(report.rows.len(), 1);
    let task = &report.rows[0];
    assert_eq!(task.title, "Fix it");
    assert_eq!(task.spent.cost_micros, 85);
    assert_eq!(task.agents.len(), 2);
}

#[tokio::test]
async fn tasks_can_be_narrowed_to_a_project() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    let p = project(&pool, org, "Alpha").await;
    let ann = agent(&pool, org, "Ann", false).await;
    let inside_project = card(&pool, org, "in", Some(p)).await;
    let outside = card(&pool, org, "out", None).await;
    agent_run(&pool, org, inside_project, ann, "u", &[step("m", 5)]).await;
    agent_run(&pool, org, outside, ann, "u", &[step("m", 9)]).await;

    let report = usecase(&pool)
        .task_costs(admin(org), period(), Some(ProjectId(p)))
        .await
        .unwrap();

    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].title, "in");
}

#[tokio::test]
async fn users_list_their_models() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    pipeline_run(&pool, org, None, "ann", &[step("m1", 20), step("m2", 30)]).await;
    pipeline_run(&pool, org, None, "bob", &[step("m1", 40)]).await;

    let report = usecase(&pool)
        .user_costs(admin(org), period())
        .await
        .unwrap();

    assert_eq!(report.rows.len(), 2);
    assert_eq!(report.rows[0].user, "ann");
    assert_eq!(report.rows[0].spent.cost_micros, 50);
    let models: Vec<_> = report.rows[0]
        .models
        .iter()
        .map(|m| (m.model.as_str(), m.spent.cost_micros))
        .collect();
    assert_eq!(models, [("m2", 30), ("m1", 20)]);
    assert_eq!(report.rows[1].user, "bob");
    assert_eq!(report.rows[1].models.len(), 1);
}

#[tokio::test]
async fn the_period_is_half_open() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    let from = Utc::now() - Duration::days(4);
    let to = from + Duration::days(2);
    let at = |at| Step {
        agent: None,
        model: Some("m"),
        cost: 10,
        at,
    };
    pipeline_run(
        &pool,
        org,
        None,
        "u",
        &[
            at(from - Duration::seconds(1)),
            at(from),
            at(to - Duration::seconds(1)),
            at(to),
        ],
    )
    .await;

    let period = Period::new(Some(from), Some(to), to).unwrap();
    let report = usecase(&pool).user_costs(admin(org), period).await.unwrap();

    assert_eq!(report.total.cost_micros, 20);
}

#[tokio::test]
async fn another_organization_never_shows() {
    let Some(pool) = pool().await else { return };
    let mine = organization(&pool).await;
    let other = organization(&pool).await;
    let other_project = project(&pool, other, "Theirs").await;
    let other_agent = agent(&pool, other, "Eve", false).await;
    let other_card = card(&pool, other, "secret", Some(other_project)).await;
    agent_run(
        &pool,
        other,
        other_card,
        other_agent,
        "eve",
        &[step("m", 99)],
    )
    .await;
    let case = usecase(&pool);

    assert!(
        case.project_costs(admin(mine), period())
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    assert!(
        case.agent_costs(admin(mine), period())
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    assert!(
        case.task_costs(admin(mine), period(), None)
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    assert!(
        case.user_costs(admin(mine), period())
            .await
            .unwrap()
            .rows
            .is_empty()
    );
    let error = case
        .task_costs(admin(mine), period(), Some(ProjectId(other_project)))
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::NotFound));
}

#[tokio::test]
async fn missing_model_and_agent_fall_under_unknown() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    let bare = Step {
        agent: None,
        model: None,
        cost: 3,
        at: inside(),
    };
    pipeline_run(&pool, org, None, "u", &[bare]).await;
    let case = usecase(&pool);

    let agents = case.agent_costs(admin(org), period()).await.unwrap();
    assert_eq!(agents.rows[0].agent.name, "unknown");
    assert!(agents.rows[0].agent.id.is_none());
    let users = case.user_costs(admin(org), period()).await.unwrap();
    assert_eq!(users.rows[0].models[0].model, "unknown");
}

#[tokio::test]
async fn an_archived_agent_keeps_its_past_cost() {
    let Some(pool) = pool().await else { return };
    let org = organization(&pool).await;
    let old = agent(&pool, org, "Retired", true).await;
    let c = card(&pool, org, "one", None).await;
    agent_run(&pool, org, c, old, "u", &[step("m", 12)]).await;

    let report = usecase(&pool)
        .agent_costs(admin(org), period())
        .await
        .unwrap();

    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].agent.name, "Retired");
    assert_eq!(report.rows[0].spent.cost_micros, 12);
}
