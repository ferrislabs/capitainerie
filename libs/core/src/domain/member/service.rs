use capitainerie_common::CoreError;

use super::{Member, ports::MemberRepository};
use crate::domain::{
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    label::{Name, clean_label},
};

const EMAIL_MAX_CHARS: usize = 254;

fn sound_name(member: &Member) -> String {
    clean_label(&member.name, Name::MAX_CHARS, "the name").unwrap_or_else(|_| {
        member
            .subject
            .trim()
            .chars()
            .take(Name::MAX_CHARS)
            .collect()
    })
}

fn sound_email(email: Option<&str>) -> Option<String> {
    let email = email?.trim();
    let valid = !email.is_empty()
        && email.chars().count() <= EMAIL_MAX_CHARS
        && !email.chars().any(|c| c.is_control() || c.is_whitespace())
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty());
    valid.then(|| email.to_owned())
}

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
        let member = Member {
            name: sound_name(&member),
            email: sound_email(member.email.as_deref()),
            ..member
        };
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

    async fn recorded(name: &str, email: Option<&str>) -> Member {
        let stored = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = std::sync::Arc::clone(&stored);
        let mut members = MockMemberRepository::new();
        members.expect_upsert().returning(move |member| {
            *sink.lock().unwrap() = Some(member.clone());
            Box::pin(async { Ok(()) })
        });
        let authorizer = MockAuthorizer::new();
        let mut input = member("user-1");
        input.name = name.into();
        input.email = email.map(str::to_owned);
        MemberService::new(members, &authorizer)
            .record(input)
            .await
            .unwrap();
        stored.lock().unwrap().clone().unwrap()
    }

    #[tokio::test]
    async fn a_sound_name_and_email_are_kept_trimmed() {
        let stored = recorded("  Claire Martin ", Some(" claire@acme.test ")).await;
        assert_eq!(stored.name, "Claire Martin");
        assert_eq!(stored.email.as_deref(), Some("claire@acme.test"));
    }

    #[tokio::test]
    async fn an_invalid_name_falls_back_to_the_subject() {
        for bad in ["", "  ", "Claire, Martin", "A\nB", &"x".repeat(121)] {
            assert_eq!(recorded(bad, None).await.name, "user-1", "{bad:?}");
        }
    }

    #[tokio::test]
    async fn an_invalid_email_is_dropped() {
        for bad in [
            "",
            "pas-un-mail",
            "@acme.test",
            "a@",
            "a b@acme.test",
            "a@b\n.test",
        ] {
            assert_eq!(recorded("Claire", Some(bad)).await.email, None, "{bad:?}");
        }
        let long = format!("{}@acme.test", "a".repeat(250));
        assert_eq!(recorded("Claire", Some(&long)).await.email, None);
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
