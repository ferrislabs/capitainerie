use capitainerie_common::CoreError;
use chrono::Utc;

use super::{
    UserProviderCredential, UserProviderSummary, ports::UserProviderRepository,
    validate_default_model, validate_user_provider,
};
use crate::domain::{
    agent::SecretToken,
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    provider::{ProviderAccess, ProviderKind, preview},
};

#[derive(Debug, Clone)]
pub struct PutUserProvider {
    pub subject: Subject,
    pub kind: ProviderKind,
    pub secret: Option<SecretToken>,
    pub base_url: Option<String>,
    pub default_model: String,
    pub is_default: Option<bool>,
}

/// Every operation is scoped to the caller's own subject: there is no way to
/// name another user.
pub struct UserProviderService<'a, R, A> {
    repository: R,
    authorizer: &'a A,
}

impl<'a, R, A> UserProviderService<'a, R, A>
where
    R: UserProviderRepository,
    A: Authorizer,
{
    pub fn new(repository: R, authorizer: &'a A) -> Self {
        Self {
            repository,
            authorizer,
        }
    }

    pub async fn list(mut self, subject: Subject) -> Result<Vec<UserProviderSummary>, CoreError> {
        self.require(subject.clone()).await?;
        self.repository
            .list(subject.organization_id, &subject.id)
            .await
    }

    pub async fn put(mut self, command: PutUserProvider) -> Result<UserProviderSummary, CoreError> {
        let subject = command.subject;
        self.require(subject.clone()).await?;
        let stored = self
            .repository
            .find(subject.organization_id, &subject.id, command.kind)
            .await?;
        let (secret, preview) = match (command.secret, stored) {
            (Some(secret), _) => {
                let preview = match secret.expose() {
                    "" => String::new(),
                    key => preview(key),
                };
                (secret, preview)
            }
            (None, Some(stored)) => (stored.secret, stored.preview),
            (None, None) => (SecretToken::new(""), String::new()),
        };
        let base_url = validate_user_provider(command.kind, &secret, command.base_url)?;
        let default_model = validate_default_model(&command.default_model)?;
        let existing = self
            .repository
            .list(subject.organization_id, &subject.id)
            .await?;
        let other_default = existing
            .iter()
            .any(|summary| summary.kind != command.kind && summary.is_default);
        let credential = UserProviderCredential {
            organization_id: subject.organization_id,
            subject: subject.id,
            kind: command.kind,
            secret,
            base_url,
            default_model,
            preview,
            is_default: command.is_default == Some(true) || !other_default,
            updated_at: Utc::now(),
        };
        self.repository.upsert(&credential).await?;
        Ok(UserProviderSummary::from(&credential))
    }

    pub async fn delete(mut self, subject: Subject, kind: ProviderKind) -> Result<(), CoreError> {
        self.require(subject.clone()).await?;
        if !self
            .repository
            .delete(subject.organization_id, &subject.id, kind)
            .await?
        {
            return Err(CoreError::NotFound);
        }
        let remaining = self
            .repository
            .list(subject.organization_id, &subject.id)
            .await?;
        if !remaining.iter().any(|summary| summary.is_default)
            && let Some(next) = remaining.first()
        {
            self.repository
                .set_default(subject.organization_id, &subject.id, next.kind)
                .await?;
        }
        Ok(())
    }

    pub async fn access(
        mut self,
        subject: Subject,
        kind: ProviderKind,
    ) -> Result<ProviderAccess, CoreError> {
        self.require(subject.clone()).await?;
        self.repository
            .find(subject.organization_id, &subject.id, kind)
            .await?
            .map(ProviderAccess::from)
            .ok_or(CoreError::NotFound)
    }

    async fn require(&self, subject: Subject) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(
                subject,
                Action::UserProviderManage,
                ResourceKind::UserProvider,
                None,
            ),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::authorization::Decision;

    struct Allow;

    impl Authorizer for Allow {
        async fn evaluate(&self, _: &AccessRequest) -> Result<Decision, CoreError> {
            Ok(Decision::Allow)
        }
    }

    #[derive(Clone, Default)]
    struct Memory {
        rows: Arc<Mutex<Vec<UserProviderCredential>>>,
    }

    fn poisoned<T>(_: std::sync::PoisonError<T>) -> CoreError {
        CoreError::Internal("poisoned".into())
    }

    impl UserProviderRepository for Memory {
        async fn upsert(&mut self, credential: &UserProviderCredential) -> Result<(), CoreError> {
            let mut rows = self.rows.lock().map_err(poisoned)?;
            if credential.is_default {
                rows.iter_mut().for_each(|row| row.is_default = false);
            }
            rows.retain(|row| row.kind != credential.kind);
            rows.push(credential.clone());
            Ok(())
        }

        async fn list(
            &mut self,
            _: OrganizationId,
            _: &str,
        ) -> Result<Vec<UserProviderSummary>, CoreError> {
            let rows = self.rows.lock().map_err(poisoned)?;
            Ok(rows.iter().map(UserProviderSummary::from).collect())
        }

        async fn find(
            &mut self,
            _: OrganizationId,
            _: &str,
            kind: ProviderKind,
        ) -> Result<Option<UserProviderCredential>, CoreError> {
            let rows = self.rows.lock().map_err(poisoned)?;
            Ok(rows.iter().find(|row| row.kind == kind).cloned())
        }

        async fn all(
            &mut self,
            _: OrganizationId,
            _: &str,
        ) -> Result<Vec<UserProviderCredential>, CoreError> {
            Ok(self.rows.lock().map_err(poisoned)?.clone())
        }

        async fn delete(
            &mut self,
            _: OrganizationId,
            _: &str,
            kind: ProviderKind,
        ) -> Result<bool, CoreError> {
            let mut rows = self.rows.lock().map_err(poisoned)?;
            let before = rows.len();
            rows.retain(|row| row.kind != kind);
            Ok(rows.len() < before)
        }

        async fn set_default(
            &mut self,
            _: OrganizationId,
            _: &str,
            kind: ProviderKind,
        ) -> Result<(), CoreError> {
            let mut rows = self.rows.lock().map_err(poisoned)?;
            rows.iter_mut()
                .for_each(|row| row.is_default = row.kind == kind);
            Ok(())
        }
    }

    fn subject() -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: OrganizationId::new(),
            actor: None,
            is_admin: false,
        }
    }

    fn put(subject: &Subject, kind: ProviderKind, is_default: Option<bool>) -> PutUserProvider {
        PutUserProvider {
            subject: subject.clone(),
            kind,
            secret: Some(SecretToken::new("sk-ant-api03-abcdefgh1234")),
            base_url: (kind == ProviderKind::OpenAiCompatible)
                .then(|| "https://generativelanguage.googleapis.com/v1beta/openai".into()),
            default_model: "some-model".into(),
            is_default,
        }
    }

    async fn store(memory: &Memory, command: PutUserProvider) -> UserProviderSummary {
        UserProviderService::new(memory.clone(), &Allow)
            .put(command)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn the_first_connection_becomes_the_default_and_shows_only_a_preview() {
        let memory = Memory::default();
        let subject = subject();
        let summary = store(&memory, put(&subject, ProviderKind::Anthropic, None)).await;
        assert!(summary.is_default);
        assert_eq!(summary.preview, "…1234");
        assert!(!format!("{summary:?}").contains("abcdefgh"));
    }

    #[tokio::test]
    async fn a_later_connection_is_not_the_default_unless_asked() {
        let memory = Memory::default();
        let subject = subject();
        store(&memory, put(&subject, ProviderKind::Anthropic, None)).await;
        let second = store(&memory, put(&subject, ProviderKind::Mistral, None)).await;
        assert!(!second.is_default);
        let third = store(
            &memory,
            put(&subject, ProviderKind::OpenAiCompatible, Some(true)),
        )
        .await;
        assert!(third.is_default);
        let listed = UserProviderService::new(memory, &Allow)
            .list(subject)
            .await
            .unwrap();
        assert_eq!(listed.iter().filter(|s| s.is_default).count(), 1);
    }

    #[tokio::test]
    async fn the_only_default_cannot_be_unset_by_a_put() {
        let memory = Memory::default();
        let subject = subject();
        store(&memory, put(&subject, ProviderKind::Anthropic, None)).await;
        let again = store(&memory, put(&subject, ProviderKind::Anthropic, Some(false))).await;
        assert!(again.is_default);
    }

    #[tokio::test]
    async fn deleting_the_default_promotes_another_connection() {
        let memory = Memory::default();
        let subject = subject();
        store(&memory, put(&subject, ProviderKind::Anthropic, None)).await;
        store(&memory, put(&subject, ProviderKind::Mistral, None)).await;
        UserProviderService::new(memory.clone(), &Allow)
            .delete(subject.clone(), ProviderKind::Anthropic)
            .await
            .unwrap();
        let listed = UserProviderService::new(memory, &Allow)
            .list(subject)
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].is_default);
    }

    #[tokio::test]
    async fn deleting_or_opening_a_missing_connection_is_not_found() {
        let memory = Memory::default();
        let subject = subject();
        let deleted = UserProviderService::new(memory.clone(), &Allow)
            .delete(subject.clone(), ProviderKind::Mistral)
            .await;
        assert!(matches!(deleted, Err(CoreError::NotFound)));
        let opened = UserProviderService::new(memory, &Allow)
            .access(subject, ProviderKind::Mistral)
            .await;
        assert!(matches!(opened, Err(CoreError::NotFound)));
    }

    #[tokio::test]
    async fn omitting_the_key_keeps_the_stored_one_and_its_preview() {
        let memory = Memory::default();
        let subject = subject();
        store(&memory, put(&subject, ProviderKind::Anthropic, None)).await;
        store(&memory, put(&subject, ProviderKind::Mistral, None)).await;
        let mut command = put(&subject, ProviderKind::Mistral, Some(true));
        command.secret = None;
        command.default_model = "other-model".into();
        let kept = store(&memory, command).await;
        assert_eq!(kept.preview, "…1234");
        assert_eq!(kept.default_model, "other-model");
        assert!(kept.is_default);
        let stored = UserProviderService::new(memory.clone(), &Allow)
            .access(subject.clone(), ProviderKind::Mistral)
            .await
            .unwrap();
        assert_eq!(stored.secret.expose(), "sk-ant-api03-abcdefgh1234");
        let listed = UserProviderService::new(memory, &Allow)
            .list(subject)
            .await
            .unwrap();
        assert_eq!(listed.iter().filter(|s| s.is_default).count(), 1);
    }

    #[tokio::test]
    async fn omitting_the_key_of_a_new_hosted_connection_is_refused() {
        let memory = Memory::default();
        let subject = subject();
        let mut command = put(&subject, ProviderKind::Anthropic, None);
        command.secret = None;
        let refused = UserProviderService::new(memory.clone(), &Allow)
            .put(command)
            .await;
        assert!(matches!(refused, Err(CoreError::Invalid(_))));
        let mut command = put(&subject, ProviderKind::OpenAiCompatible, None);
        command.secret = None;
        assert!(store(&memory, command).await.preview.is_empty());
    }

    #[tokio::test]
    async fn an_invalid_default_model_or_url_is_refused() {
        let memory = Memory::default();
        let subject = subject();
        let mut command = put(&subject, ProviderKind::Anthropic, None);
        command.default_model = "Claude Sonnet 5.5".into();
        let refused = UserProviderService::new(memory.clone(), &Allow)
            .put(command)
            .await;
        assert!(matches!(refused, Err(CoreError::Invalid(_))));
        let mut command = put(&subject, ProviderKind::OpenAiCompatible, None);
        command.base_url = Some("http://127.0.0.1:11434/v1".into());
        let refused = UserProviderService::new(memory, &Allow).put(command).await;
        assert!(matches!(refused, Err(CoreError::Invalid(_))));
    }
}
