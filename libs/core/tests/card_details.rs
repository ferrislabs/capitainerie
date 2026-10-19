#![allow(clippy::unwrap_used)]

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        authorization::Subject,
        card::{
            Card,
            attributes::{
                AcceptanceCriteria, Attributes, CardFilter, Criterion, Estimate, Labels, Priority,
            },
            edit::EditCard,
            service::CreateCard,
        },
        client::links::Change,
        member::Member,
    },
    infrastructure::postgres,
};
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
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

async fn setup() -> Option<(PgPool, CapitainerieUseCase)> {
    let pool = postgres::connect(&database_url()?, 3).await.unwrap();
    let usecase = CapitainerieUseCase::with_local_policies(pool.clone());
    Some((pool, usecase))
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

fn human(organization_id: OrganizationId) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin: false,
    }
}

fn agent(organization_id: OrganizationId) -> Subject {
    Subject {
        actor: Some("capitainerie-agent-writer".into()),
        ..human(organization_id)
    }
}

fn criterion(text: &str, done: bool) -> Criterion {
    Criterion {
        text: text.into(),
        done,
    }
}

async fn card(
    usecase: &CapitainerieUseCase,
    organization_id: OrganizationId,
    title: &str,
    attributes: Attributes,
) -> Card {
    usecase
        .create_card(
            human(organization_id),
            CreateCard {
                title: title.into(),
                attributes,
                ..Default::default()
            },
        )
        .await
        .unwrap()
}

async fn record(usecase: &CapitainerieUseCase, organization_id: OrganizationId, subject: &str) {
    usecase
        .record_member(Member {
            organization_id,
            subject: subject.into(),
            name: subject.to_uppercase(),
            email: Some(format!("{subject}@example.test")),
            last_seen_at: Utc::now(),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn card_details_round_trip_through_postgres() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    record(&usecase, org, "claire").await;
    let created = card(
        &usecase,
        org,
        "Migrer la CI",
        Attributes {
            acceptance_criteria: AcceptanceCriteria::try_from(vec![
                criterion("Les builds ARM passent", false),
                criterion("Le déploiement est documenté", true),
            ])
            .unwrap(),
            priority: Priority::High,
            due_date: NaiveDate::from_ymd_opt(2026, 10, 31),
            assignee_subject: None,
            labels: Labels::try_from(vec!["CI".into(), "infra".into()]).unwrap(),
            estimate: Some(Estimate::L),
        },
    )
    .await;

    let mut command = EditCard::new(created.version);
    command.assignee_subject = Change::Set("claire".into());
    usecase
        .edit_card(human(org), created.id, command)
        .await
        .unwrap();

    let stored = usecase
        .card_detail(human(org), created.id)
        .await
        .unwrap()
        .card;
    assert_eq!(stored.version, created.version + 1);
    assert_eq!(stored.attributes.priority, Priority::High);
    assert_eq!(
        stored.attributes.due_date,
        NaiveDate::from_ymd_opt(2026, 10, 31)
    );
    assert_eq!(stored.attributes.estimate, Some(Estimate::L));
    assert_eq!(stored.attributes.labels.as_slice(), ["CI", "infra"]);
    assert_eq!(
        stored.attributes.assignee_subject.as_deref(),
        Some("claire")
    );
    let criteria = stored.attributes.acceptance_criteria.as_slice();
    assert_eq!(criteria.len(), 2);
    assert_eq!(criteria[0], criterion("Les builds ARM passent", false));
    assert_eq!(criteria[1], criterion("Le déploiement est documenté", true));
}

#[tokio::test]
async fn a_new_card_defaults_to_normal_priority_and_no_details() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let created = card(&usecase, org, "Simple", Default::default()).await;
    let stored = usecase
        .card_detail(human(org), created.id)
        .await
        .unwrap()
        .card;
    assert_eq!(stored.attributes, Attributes::default());
    assert_eq!(stored.attributes.priority, Priority::Normal);
}

#[tokio::test]
async fn clearing_removes_the_stored_values() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let created = card(
        &usecase,
        org,
        "À vider",
        Attributes {
            priority: Priority::Urgent,
            due_date: NaiveDate::from_ymd_opt(2026, 11, 2),
            labels: Labels::try_from(vec!["a".into()]).unwrap(),
            estimate: Some(Estimate::Xs),
            ..Default::default()
        },
    )
    .await;
    let mut command = EditCard::new(created.version);
    command.priority = Change::Clear;
    command.due_date = Change::Clear;
    command.labels = Change::Clear;
    command.estimate = Change::Clear;
    let edited = usecase
        .edit_card(human(org), created.id, command)
        .await
        .unwrap();
    assert_eq!(edited.attributes, Attributes::default());
    let stored = usecase
        .card_detail(human(org), created.id)
        .await
        .unwrap()
        .card;
    assert_eq!(stored.attributes, Attributes::default());
}

#[tokio::test]
async fn the_assignee_must_be_a_member_of_the_same_organization() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let other = organization(&pool).await;
    record(&usecase, other, "outsider").await;
    let created = card(&usecase, org, "Qui ?", Default::default()).await;

    for subject in ["outsider", "nobody"] {
        let mut command = EditCard::new(created.version);
        command.assignee_subject = Change::Set(subject.into());
        let error = usecase
            .edit_card(human(org), created.id, command)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)), "{subject}");
    }
    let stored = usecase
        .card_detail(human(org), created.id)
        .await
        .unwrap()
        .card;
    assert_eq!(stored.version, created.version);
    assert_eq!(stored.attributes.assignee_subject, None);
}

#[tokio::test]
async fn a_stale_version_is_a_conflict_and_an_agent_is_refused() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let created = card(&usecase, org, "Verrou", Default::default()).await;

    let mut first = EditCard::new(created.version);
    first.priority = Change::Set(Priority::High);
    usecase
        .edit_card(human(org), created.id, first)
        .await
        .unwrap();

    let mut stale = EditCard::new(created.version);
    stale.priority = Change::Set(Priority::Low);
    let error = usecase
        .edit_card(human(org), created.id, stale)
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Conflict(_)));

    let mut by_agent = EditCard::new(created.version + 1);
    by_agent.priority = Change::Set(Priority::Low);
    let error = usecase
        .edit_card(agent(org), created.id, by_agent)
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Forbidden { .. }));

    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM outbox_events WHERE partition_key = $1 ORDER BY id")
            .bind(created.id.0)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        kinds.iter().filter(|kind| kind.contains("updated")).count(),
        1
    );
}

#[tokio::test]
async fn the_list_filters_by_priority_assignee_label_and_organization() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let other = organization(&pool).await;
    record(&usecase, org, "claire").await;
    let urgent = card(
        &usecase,
        org,
        "Urgent",
        Attributes {
            priority: Priority::Urgent,
            labels: Labels::try_from(vec!["Infra".into()]).unwrap(),
            ..Default::default()
        },
    )
    .await;
    let mut assign = EditCard::new(urgent.version);
    assign.assignee_subject = Change::Set("claire".into());
    usecase
        .edit_card(human(org), urgent.id, assign)
        .await
        .unwrap();
    card(&usecase, org, "Calme", Default::default()).await;
    card(
        &usecase,
        other,
        "Ailleurs",
        Attributes {
            priority: Priority::Urgent,
            ..Default::default()
        },
    )
    .await;

    let titles = |filter: CardFilter| {
        let usecase = usecase.clone();
        async move {
            usecase
                .list_cards(human(org), filter)
                .await
                .unwrap()
                .into_iter()
                .map(|card| card.title.as_str().to_owned())
                .collect::<Vec<_>>()
        }
    };

    assert_eq!(titles(CardFilter::default()).await.len(), 2);
    assert_eq!(
        titles(CardFilter {
            priority: Some(Priority::Urgent),
            ..Default::default()
        })
        .await,
        ["Urgent"]
    );
    assert_eq!(
        titles(CardFilter {
            assignee: Some("claire".into()),
            ..Default::default()
        })
        .await,
        ["Urgent"]
    );
    assert_eq!(
        titles(CardFilter {
            label: Some("infra".into()),
            ..Default::default()
        })
        .await,
        ["Urgent"]
    );
    assert!(
        titles(CardFilter {
            label: Some("autre".into()),
            ..Default::default()
        })
        .await
        .is_empty()
    );
    assert!(
        titles(CardFilter {
            priority: Some(Priority::Urgent),
            assignee: Some("personne".into()),
            ..Default::default()
        })
        .await
        .is_empty()
    );
}

#[tokio::test]
async fn recording_a_member_upserts_and_listing_is_per_organization() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let other = organization(&pool).await;
    record(&usecase, org, "claire").await;
    record(&usecase, org, "bastien").await;
    record(&usecase, other, "ailleurs").await;
    usecase
        .record_member(Member {
            organization_id: org,
            subject: "claire".into(),
            name: "Claire Martin".into(),
            email: None,
            last_seen_at: Utc::now(),
        })
        .await
        .unwrap();

    let members = usecase.list_members(human(org)).await.unwrap();
    let names: Vec<_> = members.iter().map(|member| member.name.as_str()).collect();
    assert_eq!(names, ["BASTIEN", "Claire Martin"]);
    assert_eq!(members[1].email, None);

    let error = usecase.list_members(agent(org)).await.unwrap_err();
    assert!(matches!(error, CoreError::Forbidden { .. }));
}
