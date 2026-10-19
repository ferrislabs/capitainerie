use capitainerie_common::CoreError;

use super::{Member, ports::MemberRepository};
use crate::domain::authorization::{
    AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require,
};

pub struct MemberService<'a, M, A> {
    members: M,
    authorizer: &'a A,
}

impl<'a, M, A> MemberService<'a, M, A>
where
    M: MemberRepository,
    A: Authorizer,
{
    pub fn new(members: M, authorizer: &'a A) -> Self {
        Self {
            members,
            authorizer,
        }
    }

    pub async fn record(mut self, member: Member) -> Result<(), CoreError> {
        if member.subject.trim().is_empty() {
            return Err(CoreError::Invalid("a member needs a subject".into()));
        }
        self.members.upsert(&member).await
    }

    pub async fn list(mut self, subject: Subject) -> Result<Vec<Member>, CoreError> {
        let organization_id = subject.organization_id;
        require(
            self.authorizer,
            AccessRequest::new(subject, Action::MemberRead, ResourceKind::Member, None),
        )
        .await?;
        self.members.list(organization_id).await
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;
    use chrono::Utc;

    use super::*;
    use crate::domain::{
        authorization::{Decision, ports::MockAuthorizer},
        member::ports::MockMemberRepository,
    };

    fn member(subject: &str) -> Member {
        Member {
            organization_id: OrganizationId::new(),
            subject: subject.into(),
            name: "Claire".into(),
            email: None,
            last_seen_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn recording_upserts_the_member() {
        let mut members = MockMemberRepository::new();
        members
            .expect_upsert()
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));
        let authorizer = MockAuthorizer::new();
        MemberService::new(members, &authorizer)
            .record(member("user-1"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_member_without_a_subject_is_refused() {
        let authorizer = MockAuthorizer::new();
        let error = MemberService::new(MockMemberRepository::new(), &authorizer)
            .record(member("  "))
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Invalid(_)));
    }

    #[tokio::test]
    async fn listing_asks_the_pdp_for_member_read() {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .withf(|request| request.action == Action::MemberRead)
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(Decision::Deny {
                        reason: "no".into(),
                    })
                })
            });
        let error = MemberService::new(MockMemberRepository::new(), &authorizer)
            .list(Subject {
                id: "user-1".into(),
                organization_id: OrganizationId::new(),
                actor: None,
                is_admin: false,
            })
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
