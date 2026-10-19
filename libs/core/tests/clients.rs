#![allow(clippy::unwrap_used)]

use capitainerie_common::{ClientId, CoreError, OrganizationId, ProjectId};
use capitainerie_core::{
    CapitainerieUseCase, RunCardLinks, SubmittedTranscript,
    domain::{
        authorization::Subject,
        card::{
            Card, Column,
            link::LinkCard,
            service::{CreateCard, ProvenanceInput},
        },
        client::{
            Client, ListFilter,
            links::{Change, LinkChoice, Links},
            service::{CreateClient, UpdateClient},
        },
        project::{
            Project, ProjectFilter,
            service::{CreateProject, UpdateProject},
        },
        run::service::SubmitTranscript,
    },
    infrastructure::postgres,
};
use sqlx::PgPool;
use uuid::Uuid;

const BODY: &str = "Claire : il faut migrer la CI vers les runners ARM.";
const WRITER: &str = "capitainerie-agent-writer";

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

fn member(organization_id: OrganizationId) -> Subject {
    Subject {
        id: "user-1".into(),
        organization_id,
        actor: None,
        is_admin: false,
    }
}

fn agent(organization_id: OrganizationId) -> Subject {
    Subject {
        actor: Some(WRITER.into()),
        ..member(organization_id)
    }
}

async fn client(usecase: &CapitainerieUseCase, org: OrganizationId, name: &str) -> Client {
    usecase
        .create_client(
            member(org),
            CreateClient {
                name: name.into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await
        .unwrap()
}

async fn project(
    usecase: &CapitainerieUseCase,
    org: OrganizationId,
    name: &str,
    client: Option<&Client>,
    repository: Option<&str>,
) -> Project {
    usecase
        .create_project(
            member(org),
            CreateProject {
                client_id: client.map(|client| client.id),
                name: name.into(),
                aliases: vec![],
                repository: repository.map(str::to_owned),
                description: String::new(),
            },
        )
        .await
        .unwrap()
}

async fn card(usecase: &CapitainerieUseCase, org: OrganizationId) -> Card {
    usecase
        .create_card(
            member(org),
            CreateCard {
                title: "Migrer la CI".into(),
                description: String::new(),
                provenance: None,
                run_id: None,
                attributes: Default::default(),
            },
        )
        .await
        .unwrap()
}

async fn submit(
    usecase: &CapitainerieUseCase,
    org: OrganizationId,
    choice: LinkChoice,
    repository: Option<&str>,
) -> Result<SubmittedTranscript, CoreError> {
    usecase
        .submit_transcript(SubmitTranscript {
            subject: member(org),
            title: "Point".into(),
            body: BODY.into(),
            metadata: capitainerie_core::domain::transcript::MeetingMetadata::new(
                None,
                None,
                repository.map(str::to_owned),
            )
            .unwrap(),
            choice,
        })
        .await
}

fn is_foreign_key_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(db) if db.code().as_deref() == Some("23503"))
}

async fn linked_card(
    usecase: &CapitainerieUseCase,
    org: OrganizationId,
    id: capitainerie_common::CardId,
) -> Links {
    usecase
        .card_detail(member(org), id)
        .await
        .unwrap()
        .card
        .links
}

async fn card_version(pool: &PgPool, id: capitainerie_common::CardId) -> i32 {
    sqlx::query_scalar("SELECT version FROM cards WHERE id = $1")
        .bind(id.0)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn keep_client() -> Change<ClientId> {
    Change::Keep
}

fn keep_project() -> Change<ProjectId> {
    Change::Keep
}

#[tokio::test]
async fn a_client_round_trips_with_its_aliases_notes_projects_and_card_counts() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = usecase
        .create_client(
            member(org),
            CreateClient {
                name: "  Acme ".into(),
                aliases: vec!["ACME SA".into(), "Acme Corp".into()],
                notes: "Client historique".into(),
            },
        )
        .await
        .unwrap();
    let site = project(&usecase, org, "Site", Some(&acme), Some("acme/web")).await;
    let proposed = card(&usecase, org).await;
    usecase
        .link_card(
            member(org),
            proposed.id,
            LinkCard {
                expected_version: proposed.version,
                client: keep_client(),
                project: Change::Set(site.id),
            },
        )
        .await
        .unwrap();

    let detail = usecase.get_client(member(org), acme.id).await.unwrap();

    assert_eq!(detail.client.name.as_str(), "Acme");
    assert_eq!(detail.client.aliases.as_slice(), ["ACME SA", "Acme Corp"]);
    assert_eq!(detail.client.notes, "Client historique");
    assert_eq!(detail.projects.len(), 1);
    assert_eq!(detail.projects[0].name.as_str(), "Site");
    assert_eq!(detail.cards.len(), Column::ALL.len());
    for count in &detail.cards {
        let expected = u64::from(count.column == Column::Proposed);
        assert_eq!(count.cards, expected, "{:?}", count.column);
    }

    let by_alias = usecase
        .list_clients(
            member(org),
            ListFilter {
                query: Some("corp".into()),
                archived: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(by_alias.len(), 1);
    assert_eq!((by_alias[0].projects, by_alias[0].open_cards), (1, 1));
    let none = usecase
        .list_clients(
            member(org),
            ListFilter {
                query: Some("zzz".into()),
                archived: None,
            },
        )
        .await
        .unwrap();
    assert!(none.is_empty());
}

#[tokio::test]
async fn archiving_hides_a_client_from_the_default_list_and_it_can_come_back() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let archived_only = ListFilter {
        query: None,
        archived: Some(true),
    };

    let archived = usecase
        .update_client(
            member(org),
            acme.id,
            UpdateClient {
                archived: Some(true),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();

    assert!(archived.is_archived());
    assert!(
        usecase
            .list_clients(member(org), ListFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        usecase
            .list_clients(member(org), archived_only.clone())
            .await
            .unwrap()
            .len(),
        1
    );

    usecase
        .update_client(
            member(org),
            acme.id,
            UpdateClient {
                archived: Some(false),
                name: Some("Acme Industries".into()),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();
    let back = usecase
        .list_clients(member(org), ListFilter::default())
        .await
        .unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].client.name.as_str(), "Acme Industries");
    assert!(
        usecase
            .list_clients(member(org), archived_only)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn names_are_unique_without_case_per_organization_and_per_client() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let other_org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let globex = client(&usecase, org, "Globex").await;

    let duplicate = usecase
        .create_client(
            member(org),
            CreateClient {
                name: "ACME".into(),
                aliases: vec![],
                notes: String::new(),
            },
        )
        .await;
    assert!(matches!(duplicate, Err(CoreError::Conflict(_))));
    client(&usecase, other_org, "Acme").await;

    project(&usecase, org, "Site", Some(&acme), None).await;
    project(&usecase, org, "Site", Some(&globex), None).await;
    project(&usecase, org, "Interne", None, None).await;
    for (name, owner) in [("site", Some(&acme)), ("INTERNE", None)] {
        let again = usecase
            .create_project(
                member(org),
                CreateProject {
                    client_id: owner.map(|client| client.id),
                    name: name.into(),
                    aliases: vec![],
                    repository: None,
                    description: String::new(),
                },
            )
            .await;
        assert!(matches!(again, Err(CoreError::Conflict(_))), "{name}");
    }
}

#[tokio::test]
async fn another_organizations_client_or_project_is_refused_by_the_service_and_the_database() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let other_org = organization(&pool).await;
    let foreign_client = client(&usecase, other_org, "Globex").await;
    let foreign_project = project(&usecase, other_org, "App", Some(&foreign_client), None).await;
    let mine = card(&usecase, org).await;

    let created = usecase
        .create_project(
            member(org),
            CreateProject {
                client_id: Some(foreign_client.id),
                name: "Site".into(),
                aliases: vec![],
                repository: None,
                description: String::new(),
            },
        )
        .await;
    assert!(matches!(created, Err(CoreError::Invalid(_))));
    for choice in [
        LinkChoice {
            client_id: Some(foreign_client.id),
            project_id: None,
        },
        LinkChoice {
            client_id: None,
            project_id: Some(foreign_project.id),
        },
    ] {
        let submitted = submit(&usecase, org, choice, None).await;
        assert!(matches!(submitted, Err(CoreError::Invalid(_))));
    }
    for (client_change, project_change) in [
        (Change::Set(foreign_client.id), keep_project()),
        (keep_client(), Change::Set(foreign_project.id)),
    ] {
        let linked = usecase
            .link_card(
                member(org),
                mine.id,
                LinkCard {
                    expected_version: mine.version,
                    client: client_change,
                    project: project_change,
                },
            )
            .await;
        assert!(matches!(linked, Err(CoreError::Invalid(_))));
    }
    assert!(matches!(
        usecase.get_client(member(org), foreign_client.id).await,
        Err(CoreError::NotFound)
    ));
    assert!(matches!(
        usecase.get_project(member(org), foreign_project.id).await,
        Err(CoreError::NotFound)
    ));
    assert!(matches!(
        usecase
            .update_client(
                member(org),
                foreign_client.id,
                UpdateClient {
                    notes: Some("x".into()),
                    ..UpdateClient::default()
                }
            )
            .await,
        Err(CoreError::NotFound)
    ));

    let transcript = submit(&usecase, org, LinkChoice::default(), None)
        .await
        .unwrap();
    let own_project = project(&usecase, org, "Site", None, None).await;
    let attempts = [
        sqlx::query("UPDATE cards SET client_id = $1 WHERE id = $2")
            .bind(foreign_client.id.0)
            .bind(mine.id.0),
        sqlx::query("UPDATE cards SET project_id = $1 WHERE id = $2")
            .bind(foreign_project.id.0)
            .bind(mine.id.0),
        sqlx::query("UPDATE transcripts SET client_id = $1 WHERE id = $2")
            .bind(foreign_client.id.0)
            .bind(transcript.transcript_id.0),
        sqlx::query("UPDATE transcripts SET project_id = $1 WHERE id = $2")
            .bind(foreign_project.id.0)
            .bind(transcript.transcript_id.0),
        sqlx::query("UPDATE projects SET client_id = $1 WHERE id = $2")
            .bind(foreign_client.id.0)
            .bind(own_project.id.0),
    ];
    for attempt in attempts {
        let error = attempt.execute(&pool).await.unwrap_err();
        assert!(is_foreign_key_violation(&error), "{error:?}");
    }
}

#[tokio::test]
async fn the_database_keeps_a_linked_project_with_its_client() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let globex = client(&usecase, org, "Globex").await;
    let site = project(&usecase, org, "Site", Some(&acme), None).await;
    let mine = card(&usecase, org).await;
    usecase
        .link_card(
            member(org),
            mine.id,
            LinkCard {
                expected_version: mine.version,
                client: keep_client(),
                project: Change::Set(site.id),
            },
        )
        .await
        .unwrap();

    let error = sqlx::query("UPDATE cards SET client_id = $1 WHERE id = $2")
        .bind(globex.id.0)
        .bind(mine.id.0)
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(is_foreign_key_violation(&error), "{error:?}");
}

#[tokio::test]
async fn moving_a_project_to_another_client_carries_its_cards_and_transcripts() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let globex = client(&usecase, org, "Globex").await;
    let site = project(&usecase, org, "Site", Some(&acme), None).await;
    let mine = card(&usecase, org).await;
    usecase
        .link_card(
            member(org),
            mine.id,
            LinkCard {
                expected_version: mine.version,
                client: keep_client(),
                project: Change::Set(site.id),
            },
        )
        .await
        .unwrap();
    let transcript = submit(
        &usecase,
        org,
        LinkChoice {
            client_id: None,
            project_id: Some(site.id),
        },
        None,
    )
    .await
    .unwrap();
    let before = card_version(&pool, mine.id).await;
    let transcript_client = || async {
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT client_id FROM transcripts WHERE id = $1")
            .bind(transcript.transcript_id.0)
            .fetch_one(&pool)
            .await
            .unwrap()
    };

    for (change, expected) in [
        (Change::Set(globex.id), Some(globex.id)),
        (Change::Clear, None),
        (Change::Set(acme.id), Some(acme.id)),
    ] {
        usecase
            .update_project(
                member(org),
                site.id,
                UpdateProject {
                    client_id: change,
                    ..UpdateProject::default()
                },
            )
            .await
            .unwrap();
        let links = linked_card(&usecase, org, mine.id).await;
        assert_eq!(links.client_id(), expected);
        assert_eq!(links.project_id(), Some(site.id));
        assert_eq!(transcript_client().await, expected.map(|id| id.0));
    }
    assert_eq!(card_version(&pool, mine.id).await, before + 3);
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE partition_key = $1 AND kind = 'card.updated'",
    )
    .bind(mine.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 4);
}

#[tokio::test]
async fn a_choice_at_intake_links_the_transcript_and_takes_the_projects_repository() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let site = project(&usecase, org, "Site", Some(&acme), Some("acme/web")).await;
    let choice = LinkChoice {
        client_id: None,
        project_id: Some(site.id),
    };

    let inherited = submit(&usecase, org, choice, None).await.unwrap();
    let explicit = submit(&usecase, org, choice, Some("other/repo"))
        .await
        .unwrap();
    let bare = submit(&usecase, org, LinkChoice::default(), None)
        .await
        .unwrap();
    let only_client = submit(
        &usecase,
        org,
        LinkChoice {
            client_id: Some(acme.id),
            project_id: None,
        },
        None,
    )
    .await
    .unwrap();

    let run = usecase
        .get_run(member(org), inherited.run_id)
        .await
        .unwrap();
    assert_eq!(run.links.client().map(|c| c.name.as_str()), Some("Acme"));
    assert_eq!(run.links.project().map(|p| p.name.as_str()), Some("Site"));
    assert_eq!(
        run.repository_hint.map(|hint| hint.to_string()).as_deref(),
        Some("acme/web")
    );
    let run = usecase.get_run(member(org), explicit.run_id).await.unwrap();
    assert_eq!(
        run.repository_hint.map(|hint| hint.to_string()).as_deref(),
        Some("other/repo")
    );
    let run = usecase.get_run(member(org), bare.run_id).await.unwrap();
    assert!(run.links.is_empty());
    assert_eq!(run.repository_hint, None);
    let run = usecase
        .get_run(member(org), only_client.run_id)
        .await
        .unwrap();
    assert_eq!(run.links.client_id(), Some(acme.id));
    assert_eq!(run.links.project(), None);
}

#[tokio::test]
async fn an_archived_client_or_project_cannot_be_chosen_at_intake() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let site = project(&usecase, org, "Site", Some(&acme), None).await;
    usecase
        .update_project(
            member(org),
            site.id,
            UpdateProject {
                archived: Some(true),
                ..UpdateProject::default()
            },
        )
        .await
        .unwrap();
    usecase
        .update_client(
            member(org),
            acme.id,
            UpdateClient {
                archived: Some(true),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();

    for choice in [
        LinkChoice {
            client_id: Some(acme.id),
            project_id: None,
        },
        LinkChoice {
            client_id: None,
            project_id: Some(site.id),
        },
    ] {
        let submitted = submit(&usecase, org, choice, None).await;
        assert!(matches!(submitted, Err(CoreError::Invalid(_))));
    }
}

#[tokio::test]
async fn the_detection_catalog_lists_active_records_most_recently_used_first() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let first = client(&usecase, org, "Premier").await;
    let second = client(&usecase, org, "Deuxieme").await;
    let third = client(&usecase, org, "Troisieme").await;
    let under_third = project(&usecase, org, "Sous le troisieme", Some(&third), None).await;
    let internal = project(&usecase, org, "Interne", None, None).await;
    let retired = client(&usecase, org, "Retire").await;
    usecase
        .update_client(
            member(org),
            retired.id,
            UpdateClient {
                archived: Some(true),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();
    let orphan = project(&usecase, org, "Orphelin", Some(&second), None).await;
    usecase
        .update_client(
            member(org),
            second.id,
            UpdateClient {
                archived: Some(true),
                ..UpdateClient::default()
            },
        )
        .await
        .unwrap();
    let mine = card(&usecase, org).await;
    usecase
        .link_card(
            member(org),
            mine.id,
            LinkCard {
                expected_version: mine.version,
                client: Change::Set(first.id),
                project: keep_project(),
            },
        )
        .await
        .unwrap();

    let catalog = usecase.detection_catalog(org).await.unwrap();
    let block = catalog.block().unwrap();
    let short = |id: Uuid| id.simple().to_string()[..8].to_owned();

    assert_eq!(catalog.len(), 4);
    let position = |id: Uuid| block.find(&short(id)).unwrap();
    assert!(position(first.id.0) < position(third.id.0));
    assert!(position(internal.id.0) < position(under_third.id.0));
    for hidden in [retired.id.0, second.id.0, orphan.id.0] {
        assert!(!block.contains(&short(hidden)), "{hidden}");
    }
    assert!(block.find("Clients connus").unwrap() < block.find("Projets connus").unwrap());
}

#[tokio::test]
async fn cards_a_run_created_are_linked_unless_a_human_already_chose() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let globex = client(&usecase, org, "Globex").await;
    let submitted = submit(&usecase, org, LinkChoice::default(), None)
        .await
        .unwrap();
    let propose = |quote: &'static str| {
        let usecase = usecase.clone();
        async move {
            let start = BODY.find(quote).unwrap() as u32;
            usecase
                .create_card(
                    agent(org),
                    CreateCard {
                        title: quote.into(),
                        description: String::new(),
                        provenance: Some(ProvenanceInput {
                            transcript_id: submitted.transcript_id,
                            start,
                            end: start + quote.len() as u32,
                            quote: quote.into(),
                        }),
                        run_id: Some(submitted.run_id),
                        attributes: Default::default(),
                    },
                )
                .await
                .unwrap()
        }
    };
    let ci = propose("migrer la CI").await;
    let runners = propose("runners ARM").await;
    let ci_span = (
        ci.provenance.as_ref().unwrap().start,
        ci.provenance.as_ref().unwrap().end,
    );
    usecase
        .link_card(
            member(org),
            runners.id,
            LinkCard {
                expected_version: runners.version,
                client: Change::Set(globex.id),
                project: keep_project(),
            },
        )
        .await
        .unwrap();

    let linked = usecase
        .link_run_cards(
            org,
            submitted.run_id,
            "user-1",
            WRITER,
            vec![
                RunCardLinks {
                    span: Some(ci_span),
                    links: Links::of(Some(acme.reference()), None).unwrap(),
                },
                RunCardLinks {
                    span: None,
                    links: Links::default(),
                },
                RunCardLinks {
                    span: Some((0, 1)),
                    links: Links::of(Some(acme.reference()), None).unwrap(),
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(linked, 1);
    assert_eq!(
        linked_card(&usecase, org, ci.id).await.client_id(),
        Some(acme.id)
    );

    let everything = usecase
        .link_run_cards(
            org,
            submitted.run_id,
            "user-1",
            WRITER,
            vec![RunCardLinks {
                span: None,
                links: Links::of(Some(acme.reference()), None).unwrap(),
            }],
        )
        .await
        .unwrap();
    assert_eq!(everything, 0);
    assert_eq!(
        linked_card(&usecase, org, runners.id).await.client_id(),
        Some(globex.id)
    );
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM outbox_events WHERE partition_key = $1 AND kind = 'card.updated'",
    )
    .bind(ci.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 1);
}

#[tokio::test]
async fn agents_are_refused_everywhere_and_the_refusals_are_audited() {
    let Some((pool, usecase)) = setup().await else {
        return;
    };
    let org = organization(&pool).await;
    let acme = client(&usecase, org, "Acme").await;
    let site = project(&usecase, org, "Site", Some(&acme), None).await;
    let mine = card(&usecase, org).await;
    let forbidden = |result: Result<(), CoreError>| {
        assert!(
            matches!(result, Err(CoreError::Forbidden { .. })),
            "{result:?}"
        );
    };

    forbidden(
        usecase
            .create_client(
                agent(org),
                CreateClient {
                    name: "Pirate".into(),
                    aliases: vec![],
                    notes: String::new(),
                },
            )
            .await
            .map(|_| ()),
    );
    forbidden(
        usecase
            .list_clients(agent(org), ListFilter::default())
            .await
            .map(|_| ()),
    );
    forbidden(usecase.get_client(agent(org), acme.id).await.map(|_| ()));
    forbidden(
        usecase
            .update_client(
                agent(org),
                acme.id,
                UpdateClient {
                    notes: Some("x".into()),
                    ..UpdateClient::default()
                },
            )
            .await
            .map(|_| ()),
    );
    forbidden(
        usecase
            .list_projects(agent(org), ProjectFilter::default())
            .await
            .map(|_| ()),
    );
    forbidden(usecase.get_project(agent(org), site.id).await.map(|_| ()));
    forbidden(
        usecase
            .update_project(
                agent(org),
                site.id,
                UpdateProject {
                    name: Some("x".into()),
                    ..UpdateProject::default()
                },
            )
            .await
            .map(|_| ()),
    );
    forbidden(
        usecase
            .link_card(
                agent(org),
                mine.id,
                LinkCard {
                    expected_version: mine.version,
                    client: Change::Set(acme.id),
                    project: keep_project(),
                },
            )
            .await
            .map(|_| ()),
    );

    let denied: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT action FROM authorization_decisions
         WHERE organization_id = $1 AND actor = $2 AND NOT allowed ORDER BY action",
    )
    .bind(org.0)
    .bind(WRITER)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(denied, ["card.update", "client.manage", "client.read"]);
    assert_eq!(linked_card(&usecase, org, mine.id).await, Links::default());
}
