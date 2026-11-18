use capitainerie_common::{AgentId, CardId, CoreError, OrganizationId};

use super::ports::CardRepository;
use crate::domain::crew::ports::CrewRepository;

pub const MAX_DEPTH: usize = 3;

/// Checks `parent` can be the parent of `card` (or of a card about to be
/// created, when `card` is `None`): it exists in the organization, it does
/// not make a loop, and the card stays within [`MAX_DEPTH`] levels.
pub async fn check_parent<C: CardRepository>(
    cards: &mut C,
    organization_id: OrganizationId,
    card: Option<CardId>,
    parent: CardId,
) -> Result<(), CoreError> {
    cards
        .get(organization_id, parent)
        .await?
        .ok_or_else(|| CoreError::Invalid("the parent card is not in the organization".into()))?;
    let ancestors = cards.ancestors(organization_id, parent).await?;
    if card.is_some_and(|card| card == parent || ancestors.contains(&card)) {
        return Err(CoreError::Invalid(
            "a card cannot be its own parent or the parent of one of its ancestors".into(),
        ));
    }
    if ancestors.len() + 2 > MAX_DEPTH {
        return Err(CoreError::Invalid(format!(
            "subtasks are limited to {MAX_DEPTH} levels"
        )));
    }
    Ok(())
}

pub async fn check_agent<R: CrewRepository>(
    crew: &mut R,
    organization_id: OrganizationId,
    id: AgentId,
) -> Result<(), CoreError> {
    match crew.find(organization_id, id).await? {
        None => Err(CoreError::Invalid(
            "the assignee agent is not in the organization".into(),
        )),
        Some(agent) if agent.archived => {
            Err(CoreError::Invalid("the assignee agent is archived".into()))
        }
        Some(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        card::{Author, Card, CardTitle, Column, ports::MockCardRepository},
        crew::{ports::MockCrewRepository, tests::agent},
    };

    fn card(organization_id: OrganizationId) -> Card {
        Card {
            id: CardId::new(),
            organization_id,
            title: CardTitle::try_from("Tâche".to_string()).unwrap(),
            description: String::new(),
            column: Column::Todo,
            provenance: None,
            created_by_run: None,
            created_by: Author {
                subject: "user-1".into(),
                actor: None,
            },
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            repository_hint: None,
            links: Default::default(),
            attributes: Default::default(),
        }
    }

    fn cards_with(parent: Option<Card>, ancestors: Vec<CardId>) -> MockCardRepository {
        let mut cards = MockCardRepository::new();
        cards.expect_get().returning(move |_, _| {
            let parent = parent.clone();
            Box::pin(async move { Ok(parent) })
        });
        cards.expect_ancestors().returning(move |_, _| {
            let ancestors = ancestors.clone();
            Box::pin(async move { Ok(ancestors) })
        });
        cards
    }

    #[tokio::test]
    async fn a_parent_of_the_organization_is_accepted() {
        let organization_id = OrganizationId::new();
        let parent = card(organization_id);
        let mut cards = cards_with(Some(parent.clone()), vec![]);
        assert!(
            check_parent(&mut cards, organization_id, None, parent.id)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn an_unknown_parent_is_refused() {
        let mut cards = cards_with(None, vec![]);
        let error = check_parent(&mut cards, OrganizationId::new(), None, CardId::new())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn subtasks_stop_at_three_levels() {
        let organization_id = OrganizationId::new();
        let parent = card(organization_id);
        let mut one_above = cards_with(Some(parent.clone()), vec![CardId::new()]);
        assert!(
            check_parent(&mut one_above, organization_id, None, parent.id)
                .await
                .is_ok()
        );
        let mut two_above = cards_with(Some(parent.clone()), vec![CardId::new(), CardId::new()]);
        let error = check_parent(&mut two_above, organization_id, None, parent.id)
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(message) if message.contains("3 levels")));
    }

    #[tokio::test]
    async fn a_card_cannot_descend_from_itself() {
        let organization_id = OrganizationId::new();
        let parent = card(organization_id);
        let own = CardId::new();
        let mut cards = cards_with(Some(parent.clone()), vec![own]);
        assert!(
            check_parent(&mut cards, organization_id, Some(own), parent.id)
                .await
                .is_err()
        );
        let mut itself = cards_with(Some(parent.clone()), vec![]);
        assert!(
            check_parent(&mut itself, organization_id, Some(parent.id), parent.id)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn only_an_active_agent_of_the_organization_can_be_assigned() {
        let organization_id = OrganizationId::new();
        let active = agent(organization_id, "dev");
        let mut archived = agent(organization_id, "old");
        archived.archived = true;
        let stored = [active.clone(), archived.clone()];
        let mut crew = MockCrewRepository::new();
        crew.expect_find().returning(move |_, id| {
            let found = stored.iter().find(|agent| agent.id == id).cloned();
            Box::pin(async move { Ok(found) })
        });
        assert!(
            check_agent(&mut crew, organization_id, active.id)
                .await
                .is_ok()
        );
        assert!(
            check_agent(&mut crew, organization_id, archived.id)
                .await
                .is_err()
        );
        assert!(
            check_agent(&mut crew, organization_id, AgentId::new())
                .await
                .is_err()
        );
    }
}
