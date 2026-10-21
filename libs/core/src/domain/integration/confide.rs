use super::{
    IntegrationInstance,
    action::Access,
    intervention::{ConfideCard, ConfideRefusal, INSTRUCTIONS_MAX_CHARS, IntervenableCard},
    journal::Grant,
};

pub fn grants_for(
    card: &IntervenableCard,
    instances: &[IntegrationInstance],
    command: &ConfideCard,
) -> Result<Vec<Grant>, ConfideRefusal> {
    if command.instructions.trim().chars().count() > INSTRUCTIONS_MAX_CHARS {
        return Err(ConfideRefusal::InstructionsTooLong);
    }
    if command.grants.is_empty() {
        return Err(ConfideRefusal::NoGrant);
    }
    let mut grants: Vec<Grant> = Vec::with_capacity(command.grants.len());
    for &(instance_id, access) in &command.grants {
        match grants
            .iter_mut()
            .find(|grant| grant.instance_id == instance_id)
        {
            Some(grant) if access == Access::Write => grant.access = Access::Write,
            Some(_) => {}
            None => grants.push(Grant {
                instance_id,
                access,
            }),
        }
    }
    for grant in &grants {
        let instance = instances
            .iter()
            .find(|instance| instance.id == grant.instance_id)
            .filter(|instance| instance.client_id == card.client_id())
            .ok_or(ConfideRefusal::InstanceOfAnotherClient)?;
        if instance.is_archived() {
            return Err(ConfideRefusal::InstanceArchived);
        }
        if grant.access == Access::Write
            && instance.environment.is_production()
            && !command.confirm_production_writes
        {
            return Err(ConfideRefusal::ProductionWriteNotConfirmed);
        }
    }
    Ok(grants)
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{CardId, ClientId, IntegrationInstanceId, OrganizationId};
    use chrono::Utc;

    use super::*;
    use crate::domain::{
        card::{Card, CardTitle, Column},
        client::{ClientRef, links::Links},
        integration::{
            BaseUrl, Environment, InstanceName, IntegrationInstance, IntegrationKind,
            action::Access,
            intervention::{ConfideCard, ConfideRefusal, IntervenableCard},
            journal::Grant,
        },
    };

    fn card_of(client_id: ClientId) -> IntervenableCard {
        let links = Links::of(
            Some(ClientRef {
                id: client_id,
                name: "Acme".into(),
            }),
            None,
        )
        .unwrap();
        IntervenableCard::try_from(Card {
            id: CardId::new(),
            organization_id: OrganizationId::new(),
            title: CardTitle::try_from("Vérifier les hypothèses".to_string()).unwrap(),
            description: String::new(),
            column: Column::Todo,
            provenance: None,
            created_by_run: None,
            created_by: crate::domain::card::Author {
                subject: "user-1".into(),
                actor: None,
            },
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            repository_hint: None,
            links,
            attributes: Default::default(),
        })
        .unwrap()
    }

    fn instance_of(
        client_id: ClientId,
        environment: Environment,
        archived: bool,
    ) -> IntegrationInstance {
        IntegrationInstance {
            id: IntegrationInstanceId::new(),
            organization_id: OrganizationId::new(),
            client_id,
            kind: IntegrationKind::OdooJson2,
            name: InstanceName::try_from("Odoo".to_string()).unwrap(),
            environment,
            base_url: BaseUrl::try_from("https://odoo.acme.test".to_string()).unwrap(),
            database: None,
            login: None,
            secret_preview: "…".into(),
            server_version: None,
            checked_at: None,
            created_by: "admin".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: archived.then(Utc::now),
        }
    }

    fn command(grants: Vec<(IntegrationInstanceId, Access)>) -> ConfideCard {
        ConfideCard {
            instructions: "Vérifier les factures".into(),
            grants,
            confirm_production_writes: false,
            expected_version: 1,
        }
    }

    #[test]
    fn each_requested_instance_becomes_a_grant_of_its_level() {
        let acme = ClientId::new();
        let staging = instance_of(acme, Environment::Staging, false);
        let production = instance_of(acme, Environment::Production, false);
        let card = card_of(acme);

        let grants = grants_for(
            &card,
            &[staging.clone(), production.clone()],
            &command(vec![
                (staging.id, Access::Write),
                (production.id, Access::Read),
            ]),
        )
        .unwrap();

        assert_eq!(
            grants,
            [
                Grant {
                    instance_id: staging.id,
                    access: Access::Write
                },
                Grant {
                    instance_id: production.id,
                    access: Access::Read
                },
            ]
        );
    }

    #[test]
    fn a_duplicated_instance_keeps_the_highest_access() {
        let acme = ClientId::new();
        let staging = instance_of(acme, Environment::Staging, false);
        let other = instance_of(acme, Environment::Development, false);

        for requested in [
            vec![
                (staging.id, Access::Read),
                (other.id, Access::Read),
                (staging.id, Access::Write),
            ],
            vec![
                (staging.id, Access::Write),
                (other.id, Access::Read),
                (staging.id, Access::Read),
            ],
        ] {
            let grants = grants_for(
                &card_of(acme),
                &[staging.clone(), other.clone()],
                &command(requested),
            )
            .unwrap();

            assert_eq!(grants.len(), 2);
            assert!(grants.contains(&Grant {
                instance_id: staging.id,
                access: Access::Write
            }));
            assert!(grants.contains(&Grant {
                instance_id: other.id,
                access: Access::Read
            }));
        }
    }

    #[test]
    fn no_instance_is_no_grant() {
        let refusal = grants_for(&card_of(ClientId::new()), &[], &command(vec![]));

        assert_eq!(refusal, Err(ConfideRefusal::NoGrant));
    }

    #[test]
    fn an_instance_of_another_client_is_refused() {
        let acme = ClientId::new();
        let globex = instance_of(ClientId::new(), Environment::Staging, false);

        let refusal = grants_for(
            &card_of(acme),
            std::slice::from_ref(&globex),
            &command(vec![(globex.id, Access::Read)]),
        );

        assert_eq!(refusal, Err(ConfideRefusal::InstanceOfAnotherClient));
    }

    #[test]
    fn an_instance_the_caller_could_not_load_is_refused_like_one_of_another_client() {
        let acme = ClientId::new();
        let staging = instance_of(acme, Environment::Staging, false);
        let unknown = IntegrationInstanceId::new();

        let refusal = grants_for(
            &card_of(acme),
            std::slice::from_ref(&staging),
            &command(vec![(staging.id, Access::Read), (unknown, Access::Read)]),
        );

        assert_eq!(refusal, Err(ConfideRefusal::InstanceOfAnotherClient));
    }

    #[test]
    fn an_archived_instance_is_refused() {
        let acme = ClientId::new();
        let archived = instance_of(acme, Environment::Staging, true);

        let refusal = grants_for(
            &card_of(acme),
            std::slice::from_ref(&archived),
            &command(vec![(archived.id, Access::Read)]),
        );

        assert_eq!(refusal, Err(ConfideRefusal::InstanceArchived));
    }

    #[test]
    fn an_archived_instance_of_another_client_is_not_told_apart() {
        let archived = instance_of(ClientId::new(), Environment::Staging, true);

        let refusal = grants_for(
            &card_of(ClientId::new()),
            std::slice::from_ref(&archived),
            &command(vec![(archived.id, Access::Read)]),
        );

        assert_eq!(refusal, Err(ConfideRefusal::InstanceOfAnotherClient));
    }

    #[test]
    fn writing_to_production_needs_the_confirmation() {
        let acme = ClientId::new();
        let production = instance_of(acme, Environment::Production, false);
        let card = card_of(acme);
        let mut confide = command(vec![(production.id, Access::Write)]);

        let refused = grants_for(&card, std::slice::from_ref(&production), &confide);
        confide.confirm_production_writes = true;
        let confirmed = grants_for(&card, std::slice::from_ref(&production), &confide);

        assert_eq!(refused, Err(ConfideRefusal::ProductionWriteNotConfirmed));
        assert_eq!(
            confirmed,
            Ok(vec![Grant {
                instance_id: production.id,
                access: Access::Write
            }])
        );
    }

    #[test]
    fn reading_production_and_writing_staging_need_no_confirmation() {
        let acme = ClientId::new();
        let production = instance_of(acme, Environment::Production, false);
        let staging = instance_of(acme, Environment::Staging, false);

        let granted = grants_for(
            &card_of(acme),
            &[production.clone(), staging.clone()],
            &command(vec![
                (production.id, Access::Read),
                (staging.id, Access::Write),
            ]),
        );

        assert_eq!(granted.unwrap().len(), 2);
    }

    #[test]
    fn a_write_that_a_duplicate_read_would_hide_still_needs_the_confirmation() {
        let acme = ClientId::new();
        let production = instance_of(acme, Environment::Production, false);

        let refusal = grants_for(
            &card_of(acme),
            std::slice::from_ref(&production),
            &command(vec![
                (production.id, Access::Write),
                (production.id, Access::Read),
            ]),
        );

        assert_eq!(refusal, Err(ConfideRefusal::ProductionWriteNotConfirmed));
    }

    #[test]
    fn instructions_are_trimmed_then_bounded() {
        let acme = ClientId::new();
        let staging = instance_of(acme, Environment::Staging, false);
        let card = card_of(acme);
        let mut confide = command(vec![(staging.id, Access::Read)]);

        confide.instructions = format!("  {}  ", "é".repeat(INSTRUCTIONS_MAX_CHARS));
        let at_the_limit = grants_for(&card, std::slice::from_ref(&staging), &confide);
        confide.instructions = "é".repeat(INSTRUCTIONS_MAX_CHARS + 1);
        let over_the_limit = grants_for(&card, std::slice::from_ref(&staging), &confide);

        assert!(at_the_limit.is_ok());
        assert_eq!(over_the_limit, Err(ConfideRefusal::InstructionsTooLong));
    }

    #[test]
    fn too_long_instructions_are_refused_before_anything_else() {
        let mut confide = command(vec![]);
        confide.instructions = "x".repeat(INSTRUCTIONS_MAX_CHARS + 1);

        let refusal = grants_for(&card_of(ClientId::new()), &[], &confide);

        assert_eq!(refusal, Err(ConfideRefusal::InstructionsTooLong));
    }
}
