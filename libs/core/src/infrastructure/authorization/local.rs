use capitainerie_common::CoreError;

use crate::domain::{
    authorization::{AccessRequest, Action, Decision, Subject, ports::Authorizer},
    card::Column,
};

/// In-process stand-in for FerrisKey's AuthZEN PDP until its HTTP endpoint
/// exists (docs/ferriskey-needs.md, need 1). Same port, so swapping it is a
/// one-line change in the application's type alias.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalPolicyEngine;

impl Authorizer for LocalPolicyEngine {
    async fn evaluate(&self, request: &AccessRequest) -> Result<Decision, CoreError> {
        Ok(decide(request))
    }
}

fn decide(request: &AccessRequest) -> Decision {
    if request.subject.organization_id != request.resource.organization_id {
        return deny("the resource belongs to another organization");
    }
    match &request.subject.actor {
        None => decide_for_human(&request.subject, request.action),
        Some(actor) => decide_for_agent(actor, request.action),
    }
}

fn decide_for_human(subject: &Subject, action: Action) -> Decision {
    match action {
        Action::ProviderManage | Action::ModelsAssign if !subject.is_admin => {
            deny("only an organization admin may do this")
        }
        _ => Decision::Allow,
    }
}

/// An agent acts for a human, never as one: it proposes and refines, and the
/// human decides what enters the board.
fn decide_for_agent(actor: &str, action: Action) -> Decision {
    match action {
        Action::CardCreate | Action::CardRead | Action::CardComment => Decision::Allow,
        Action::CardUpdate { column } if is_settled(column) => deny(&format!(
            "agent {actor} may not edit a {} card",
            column.as_str()
        )),
        Action::CardUpdate { .. } => Decision::Allow,
        Action::CardMove {
            from: Column::Proposed,
            ..
        } => deny("only a human can take a card out of proposed"),
        Action::CardMove { to, .. } if matches!(to, Column::Proposed | Column::Rejected) => deny(
            &format!("agent {actor} may not move a card to {}", to.as_str()),
        ),
        Action::CardMove { .. } => Decision::Allow,
        other => deny(&format!("agent {actor} may not {}", other.as_str())),
    }
}

fn is_settled(column: Column) -> bool {
    matches!(column, Column::Done | Column::Rejected)
}

fn deny(reason: &str) -> Decision {
    Decision::Deny {
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::authorization::{ResourceKind, Subject};

    fn request(action: Action, actor: Option<&str>, same_org: bool) -> AccessRequest {
        let organization_id = OrganizationId::new();
        let mut request = AccessRequest::new(
            Subject {
                id: "user-1".into(),
                organization_id,
                actor: actor.map(str::to_owned),
                is_admin: false,
            },
            action,
            ResourceKind::Card,
            None,
        );
        if !same_org {
            request.resource.organization_id = OrganizationId::new();
        }
        request
    }

    fn allowed(action: Action, actor: Option<&str>) -> bool {
        decide(&request(action, actor, true)) == Decision::Allow
    }

    const AGENT: Option<&str> = Some("capitainerie-agent-writer");

    #[test]
    fn humans_do_anything_in_their_organization() {
        assert!(allowed(Action::TranscriptCreate, None));
        assert!(allowed(
            Action::CardMove {
                from: Column::Proposed,
                to: Column::Todo
            },
            None
        ));
    }

    fn admin_request(action: Action, actor: Option<&str>, is_admin: bool) -> AccessRequest {
        let mut request = request(action, actor, true);
        request.subject.is_admin = is_admin;
        request
    }

    #[test]
    fn only_admins_manage_providers_and_assign_models() {
        for action in [Action::ProviderManage, Action::ModelsAssign] {
            assert_eq!(decide(&admin_request(action, None, true)), Decision::Allow);
            assert!(matches!(
                decide(&admin_request(action, None, false)),
                Decision::Deny { .. }
            ));
        }
    }

    #[test]
    fn every_member_reads_providers() {
        assert_eq!(
            decide(&admin_request(Action::ProviderRead, None, false)),
            Decision::Allow
        );
    }

    #[test]
    fn agents_get_no_provider_action_even_for_an_admin() {
        for action in [
            Action::ProviderRead,
            Action::ProviderManage,
            Action::ModelsAssign,
        ] {
            assert!(matches!(
                decide(&admin_request(action, AGENT, true)),
                Decision::Deny { .. }
            ));
        }
    }

    #[test]
    fn nobody_crosses_organizations() {
        assert!(matches!(
            decide(&request(Action::RunRead, None, false)),
            Decision::Deny { .. }
        ));
    }

    #[test]
    fn agents_propose_and_comment_but_do_not_submit_transcripts() {
        assert!(allowed(Action::CardCreate, AGENT));
        assert!(allowed(Action::CardComment, AGENT));
        assert!(!allowed(Action::TranscriptCreate, AGENT));
    }

    #[test]
    fn only_humans_take_a_card_out_of_proposed() {
        assert!(!allowed(
            Action::CardMove {
                from: Column::Proposed,
                to: Column::Todo
            },
            AGENT
        ));
        assert!(!allowed(
            Action::CardMove {
                from: Column::Todo,
                to: Column::Rejected
            },
            AGENT
        ));
        assert!(allowed(
            Action::CardMove {
                from: Column::Todo,
                to: Column::InProgress
            },
            AGENT
        ));
    }

    #[test]
    fn agents_leave_settled_cards_alone() {
        assert!(allowed(
            Action::CardUpdate {
                column: Column::Proposed
            },
            AGENT
        ));
        assert!(!allowed(
            Action::CardUpdate {
                column: Column::Done
            },
            AGENT
        ));
    }
}
