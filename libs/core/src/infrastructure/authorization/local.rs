use capitainerie_common::CoreError;

use crate::domain::authorization::{AccessRequest, Action, Decision, ports::Authorizer};

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
    match (request.action, &request.subject.actor) {
        (Action::TranscriptCreate | Action::RunRead, None) => Decision::Allow,
        (action, Some(actor)) => deny(&format!("agent {actor} may not {}", action.as_str())),
    }
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
    use crate::domain::authorization::{Resource, ResourceKind, Subject};

    fn request(action: Action, actor: Option<&str>, same_org: bool) -> AccessRequest {
        let organization_id = OrganizationId::new();
        AccessRequest {
            subject: Subject {
                id: "user-1".into(),
                organization_id,
                actor: actor.map(str::to_owned),
            },
            action,
            resource: Resource {
                kind: ResourceKind::Transcript,
                id: None,
                organization_id: if same_org {
                    organization_id
                } else {
                    OrganizationId::new()
                },
            },
        }
    }

    #[test]
    fn members_submit_and_read_in_their_organization() {
        assert_eq!(
            decide(&request(Action::TranscriptCreate, None, true)),
            Decision::Allow
        );
        assert_eq!(
            decide(&request(Action::RunRead, None, true)),
            Decision::Allow
        );
    }

    #[test]
    fn nobody_crosses_organizations() {
        assert!(matches!(
            decide(&request(Action::RunRead, None, false)),
            Decision::Deny { .. }
        ));
    }

    #[test]
    fn agents_do_not_submit_transcripts() {
        assert!(matches!(
            decide(&request(
                Action::TranscriptCreate,
                Some("agent-writer"),
                true
            )),
            Decision::Deny { .. }
        ));
    }
}
