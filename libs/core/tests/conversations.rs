#![allow(clippy::unwrap_used)]

use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::agent::Usage,
    domain::{
        authorization::Subject,
        conversation::{
            MessageContent, MessageRole, MessageStatus, ProposedCard, ToolRecord,
            service::{TurnOutcome, UpdateConversation},
            turn::{ChatRole, TurnError},
        },
        provider::{ModelRef, ProviderKind},
    },
    infrastructure::postgres,
};
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

async fn pool() -> Option<PgPool> {
    let Some(url) = database_url() else {
        eprintln!("skipped: DATABASE_URL is unset");
        return None;
    };
    Some(postgres::connect(&url, 4).await.unwrap())
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

fn member(organization_id: OrganizationId, id: &str) -> Subject {
    Subject {
        id: id.into(),
        organization_id,
        actor: None,
        is_admin: false,
    }
}

fn usecase(pool: &PgPool) -> CapitainerieUseCase {
    CapitainerieUseCase::with_local_policies(pool.clone())
}

fn outcome(text: &str, status: MessageStatus) -> TurnOutcome {
    TurnOutcome {
        content: MessageContent {
            text: text.into(),
            tool_calls: vec![ToolRecord {
                name: "search_cards".into(),
                arguments: "{\"query\":\"readme\"}".into(),
                succeeded: true,
                latency_ms: 12,
                summary: "3 cartes".into(),
            }],
            cards: vec![ProposedCard {
                card_id: Uuid::now_v7(),
                title: "Ajouter un README".into(),
            }],
        },
        status,
        usage: Usage {
            model: "claude-sonnet-5-5".into(),
            input_tokens: 1_000,
            output_tokens: 200,
        },
        cost_micros: 4_000,
        model: ModelRef {
            provider: ProviderKind::Anthropic,
            model: "claude-sonnet-5-5".into(),
        },
    }
}

#[tokio::test]
async fn a_conversation_round_trips_with_its_messages_and_usage() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let owner = member(organization_id, "user-1");
    let usecase = usecase(&pool);

    let created = usecase
        .create_conversation(owner.clone(), "Ajoute un README au dépôt test".into())
        .await
        .unwrap();
    let started = usecase
        .start_conversation_turn(owner.clone(), created.id, None)
        .await
        .unwrap();
    assert_eq!(started.history.len(), 1);
    assert_eq!(started.history[0].role, ChatRole::User);
    usecase
        .finish_conversation_turn(
            owner.clone(),
            created.id,
            outcome("Voici.", MessageStatus::Complete),
        )
        .await
        .unwrap();

    let (conversation, messages) = usecase
        .get_conversation(owner.clone(), created.id)
        .await
        .unwrap();
    assert_eq!(conversation.title, "Ajoute un README au dépôt test");
    assert_eq!(
        (
            conversation.input_tokens,
            conversation.output_tokens,
            conversation.cost_micros
        ),
        (1_000, 200, 4_000)
    );
    assert_eq!(conversation.model.unwrap().model, "claude-sonnet-5-5");
    let seqs: Vec<_> = messages.iter().map(|m| (m.seq, m.role, m.status)).collect();
    assert_eq!(
        seqs,
        [
            (1, MessageRole::User, MessageStatus::Complete),
            (2, MessageRole::Assistant, MessageStatus::Complete)
        ]
    );
    assert_eq!(messages[1].content.tool_calls[0].name, "search_cards");
    assert_eq!(messages[1].content.cards[0].title, "Ajouter un README");
}

#[tokio::test]
async fn only_the_owner_finds_a_conversation() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let owner = member(organization_id, "user-1");
    let other_member = member(organization_id, "user-2");
    let other_organization = member(organization(&pool).await, "user-1");
    let usecase = usecase(&pool);
    let created = usecase
        .create_conversation(owner.clone(), "Secret".into())
        .await
        .unwrap();

    for stranger in [other_member, other_organization] {
        assert!(matches!(
            usecase.get_conversation(stranger.clone(), created.id).await,
            Err(CoreError::NotFound)
        ));
        assert!(matches!(
            usecase
                .update_conversation(stranger.clone(), created.id, UpdateConversation::default())
                .await,
            Err(CoreError::NotFound)
        ));
        assert!(matches!(
            usecase
                .start_conversation_turn(stranger.clone(), created.id, Some("salut".into()))
                .await,
            Err(TurnError::Core(CoreError::NotFound))
        ));
        assert!(
            usecase
                .list_conversations(stranger)
                .await
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(usecase.list_conversations(owner).await.unwrap().len(), 1);
}

#[tokio::test]
async fn one_turn_at_a_time_and_the_slot_frees_when_it_finishes() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let owner = member(organization_id, "user-1");
    let usecase = usecase(&pool);
    let created = usecase
        .create_conversation(owner.clone(), "Bonjour".into())
        .await
        .unwrap();

    usecase
        .start_conversation_turn(owner.clone(), created.id, None)
        .await
        .unwrap();
    let second = usecase
        .start_conversation_turn(owner.clone(), created.id, Some("Encore".into()))
        .await;
    assert!(matches!(second, Err(TurnError::InProgress)));
    let (_, messages) = usecase
        .get_conversation(owner.clone(), created.id)
        .await
        .unwrap();
    assert_eq!(messages.len(), 1, "a refused turn stores nothing");

    usecase
        .finish_conversation_turn(
            owner.clone(),
            created.id,
            outcome("Ok", MessageStatus::Interrupted),
        )
        .await
        .unwrap();
    let third = usecase
        .start_conversation_turn(owner.clone(), created.id, Some("Encore".into()))
        .await
        .unwrap();
    let texts: Vec<_> = third.history.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts[0], "Bonjour");
    assert!(texts[1].starts_with("Ok") && texts[1].ends_with("[réponse interrompue]"));
    assert_eq!(texts[2], "Encore");
}

#[tokio::test]
async fn an_expired_lease_does_not_lock_the_conversation_for_good() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let owner = member(organization_id, "user-1");
    let usecase = usecase(&pool);
    let created = usecase
        .create_conversation(owner.clone(), "Bonjour".into())
        .await
        .unwrap();
    usecase
        .start_conversation_turn(owner.clone(), created.id, None)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE conversations SET turn_started_at = now() - interval '1 hour' WHERE id = $1",
    )
    .bind(created.id.0)
    .execute(&pool)
    .await
    .unwrap();

    usecase
        .start_conversation_turn(owner, created.id, Some("Reprise".into()))
        .await
        .unwrap();
}

#[tokio::test]
async fn a_full_conversation_refuses_a_new_turn() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let owner = member(organization_id, "user-1");
    let usecase = usecase(&pool);
    let created = usecase
        .create_conversation(owner.clone(), "Bonjour".into())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE conversations SET input_tokens = 390000, output_tokens = 10000 WHERE id = $1",
    )
    .bind(created.id.0)
    .execute(&pool)
    .await
    .unwrap();

    let refused = usecase
        .start_conversation_turn(owner, created.id, Some("Encore".into()))
        .await;

    assert!(matches!(refused, Err(TurnError::Full)));
}

#[tokio::test]
async fn titles_can_be_changed_and_archived_conversations_leave_the_list() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let owner = member(organization_id, "user-1");
    let usecase = usecase(&pool);
    let created = usecase
        .create_conversation(owner.clone(), "Bonjour".into())
        .await
        .unwrap();

    let renamed = usecase
        .update_conversation(
            owner.clone(),
            created.id,
            UpdateConversation {
                title: Some("README Acme".into()),
                archived: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(renamed.title, "README Acme");
    usecase
        .update_conversation(
            owner.clone(),
            created.id,
            UpdateConversation {
                title: None,
                archived: Some(true),
            },
        )
        .await
        .unwrap();

    assert!(
        usecase
            .list_conversations(owner.clone())
            .await
            .unwrap()
            .is_empty()
    );
    let refused = usecase
        .start_conversation_turn(owner, created.id, Some("Encore".into()))
        .await;
    assert!(matches!(
        refused,
        Err(TurnError::Core(CoreError::Invalid(_)))
    ));
}

#[tokio::test]
async fn the_captain_is_denied_conversations_and_the_denial_is_audited() {
    let Some(pool) = pool().await else { return };
    let organization_id = organization(&pool).await;
    let mut captain = member(organization_id, "user-1");
    captain.actor = Some("capitainerie-agent-captain".into());

    let refused = usecase(&pool)
        .create_conversation(captain, "Bonjour".into())
        .await;

    assert!(matches!(refused, Err(CoreError::Forbidden { .. })));
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM authorization_decisions WHERE organization_id = $1 AND action = 'conversation.write' AND NOT allowed",
    )
    .bind(organization_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, 1);
}
