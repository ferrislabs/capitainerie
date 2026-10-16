use capitainerie_common::CoreError;

use crate::domain::{
    authorization::{AccessRequest, Action, Decision, ports::Authorizer},
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
        None => Decision::Allow,
        Some(actor) => decide_for_agent(actor, request.action),
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
