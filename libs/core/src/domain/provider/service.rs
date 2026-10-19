use capitainerie_common::{CoreError, OrganizationId};
use chrono::Utc;

use super::{
    AgentRole, ModelAssignments, ModelTarget, ProviderAccess, ProviderCredential, ProviderKind,
    ProviderSummary,
    ports::{CatalogModel, ModelAssignmentRepository, ProviderRepository},
    preview,
};
use crate::domain::{
    agent::SecretToken,
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
};

#[derive(Debug, Clone)]
pub struct PutProvider {
    pub subject: Subject,
    pub kind: ProviderKind,
    pub secret: SecretToken,
    pub base_url: Option<String>,
}

pub struct ProviderService<'a, P, M, A> {
    providers: P,
    assignments: M,
    authorizer: &'a A,
}

impl<'a, P, M, A> ProviderService<'a, P, M, A>
where
    P: ProviderRepository,
    M: ModelAssignmentRepository,
    A: Authorizer,
{
    pub fn new(providers: P, assignments: M, authorizer: &'a A) -> Self {
        Self {
            providers,
            assignments,
            authorizer,
        }
    }

    pub async fn list(mut self, subject: Subject) -> Result<Vec<ProviderSummary>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::ProviderRead, ResourceKind::Provider, None)
            .await?;
        self.providers.list(organization_id).await
    }

    pub async fn put(mut self, command: PutProvider) -> Result<ProviderSummary, CoreError> {
        let subject = command.subject;
        self.require(
            subject.clone(),
            Action::ProviderManage,
            ResourceKind::Provider,
            None,
        )
        .await?;
        let base_url = validate_provider(command.kind, &command.secret, command.base_url)?;
        let preview = match command.secret.expose() {
            "" => String::new(),
            secret => preview(secret),
        };
        let credential = ProviderCredential {
            organization_id: subject.organization_id,
            kind: command.kind,
            secret: command.secret,
            base_url,
            preview,
            created_by: subject.id,
            updated_at: Utc::now(),
        };
        self.providers.upsert(&credential).await?;
        Ok(ProviderSummary::from(&credential))
    }

    pub async fn delete(mut self, subject: Subject, kind: ProviderKind) -> Result<(), CoreError> {
        let organization_id = subject.organization_id;
        self.require(
            subject,
            Action::ProviderManage,
            ResourceKind::Provider,
            None,
        )
        .await?;
        if self.providers.delete(organization_id, kind).await? {
            Ok(())
        } else {
            Err(CoreError::NotFound)
        }
    }

    pub async fn access_to_manage(
        mut self,
        subject: Subject,
        kind: ProviderKind,
    ) -> Result<ProviderAccess, CoreError> {
        let organization_id = subject.organization_id;
        self.require(
            subject,
            Action::ProviderManage,
            ResourceKind::Provider,
            None,
        )
        .await?;
        self.providers
            .find(organization_id, kind)
            .await?
            .map(ProviderAccess::from)
            .ok_or(CoreError::NotFound)
    }

    pub async fn accesses(
        mut self,
        subject: Subject,
        action: Action,
    ) -> Result<Vec<ProviderAccess>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, action, ResourceKind::Provider, None)
            .await?;
        self.all_accesses(organization_id).await
    }

    pub async fn assignments(
        mut self,
        subject: Subject,
    ) -> Result<Option<ModelAssignments>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(
            subject,
            Action::ProviderRead,
            ResourceKind::ModelAssignments,
            None,
        )
        .await?;
        self.assignments.get(organization_id).await
    }

    pub async fn assign(
        mut self,
        subject: Subject,
        assignments: ModelAssignments,
    ) -> Result<ModelAssignments, CoreError> {
        let organization_id = subject.organization_id;
        self.require(
            subject.clone(),
            Action::ModelsAssign,
            ResourceKind::ModelAssignments,
            None,
        )
        .await?;
        assignments.check_transcriber()?;
        let configured: Vec<ProviderKind> = self
            .providers
            .list(organization_id)
            .await?
            .into_iter()
            .map(|summary| summary.kind)
            .collect();
        for role in AgentRole::ALL {
            let Some(chosen) = assignments.for_role(role) else {
                continue;
            };
            let provider = chosen.provider;
            if !configured.contains(&provider) {
                return Err(CoreError::Invalid(format!(
                    "{} is not configured for this organization (role {})",
                    provider.as_str(),
                    role.as_str()
                )));
            }
        }
        self.assignments
            .put(organization_id, &assignments, &subject.id)
            .await?;
        Ok(assignments)
    }

    pub async fn transcriber_target(
        mut self,
        subject: Subject,
    ) -> Result<Option<ModelTarget>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(
            subject,
            Action::TranscriptionCreate,
            ResourceKind::Transcription,
            None,
        )
        .await?;
        let Some(model) = self
            .assignments
            .get(organization_id)
            .await?
            .and_then(|assignments| assignments.transcriber)
        else {
            return Ok(None);
        };
        Ok(self
            .providers
            .find(organization_id, model.provider)
            .await?
            .map(|credential| ModelTarget {
                model,
                access: Some(ProviderAccess::from(credential)),
            }))
    }

    async fn all_accesses(
        &mut self,
        organization_id: OrganizationId,
    ) -> Result<Vec<ProviderAccess>, CoreError> {
        let mut accesses = Vec::new();
        for summary in self.providers.list(organization_id).await? {
            if let Some(credential) = self.providers.find(organization_id, summary.kind).await? {
                accesses.push(ProviderAccess::from(credential));
            }
        }
        Ok(accesses)
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        kind: ResourceKind,
        id: Option<uuid::Uuid>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(subject, action, kind, id),
        )
        .await
    }
}

/// Returns the base URL to keep: only the OpenAI-compatible provider has one.
pub fn validate_provider(
    kind: ProviderKind,
    secret: &SecretToken,
    base_url: Option<String>,
) -> Result<Option<String>, CoreError> {
    match kind {
        ProviderKind::OpenAiCompatible => {
            let url = base_url
                .map(|url| url.trim().trim_end_matches('/').to_owned())
                .filter(|url| !url.is_empty())
                .ok_or_else(|| {
                    CoreError::Invalid("base_url is required for openai_compatible".into())
                })?;
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(CoreError::Invalid(
                    "base_url must start with http:// or https://".into(),
                ));
            }
            Ok(Some(url))
        }
        ProviderKind::Anthropic | ProviderKind::Mistral => {
            if secret.expose().trim().is_empty() {
                return Err(CoreError::Invalid(format!(
                    "an API key is required for {}",
                    kind.as_str()
                )));
            }
            Ok(None)
        }
    }
}

/// Every role must name a model its provider really offers, with the
/// capability the role needs. `catalog` holds what the configured providers
/// announced.
pub fn validate_assignments(
    assignments: &ModelAssignments,
    catalog: &[CatalogModel],
) -> Result<(), CoreError> {
    assignments.check_transcriber()?;
    for role in AgentRole::ALL {
        let Some(chosen) = assignments.for_role(role) else {
            continue;
        };
        if role == AgentRole::Embeddings && chosen.provider == ProviderKind::Anthropic {
            return Err(CoreError::Invalid(
                "anthropic offers no embeddings model".into(),
            ));
        }
        let model = catalog
            .iter()
            .find(|model| model.provider == chosen.provider && model.id == chosen.model)
            .ok_or_else(|| {
                CoreError::Invalid(format!(
                    "{} does not offer {} (role {})",
                    chosen.provider.as_str(),
                    chosen.model,
                    role.as_str()
                ))
            })?;
        let capable = match role {
            AgentRole::Embeddings => model.embeddings,
            AgentRole::Transcriber => model.transcription,
            _ => model.chat,
        };
        if !capable {
            return Err(CoreError::Invalid(format!(
                "{} cannot serve the {} role",
                chosen.model,
                role.as_str()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::domain::{authorization::Decision, provider::ModelRef};

    fn secret(value: &str) -> SecretToken {
        SecretToken::new(value)
    }

    #[test]
    fn an_openai_compatible_endpoint_needs_an_http_base_url() {
        let none = validate_provider(ProviderKind::OpenAiCompatible, &secret(""), None);
        assert!(matches!(none, Err(CoreError::Invalid(_))));
        let blank = validate_provider(
            ProviderKind::OpenAiCompatible,
            &secret(""),
            Some("  ".into()),
        );
        assert!(matches!(blank, Err(CoreError::Invalid(_))));
        let ftp = validate_provider(
            ProviderKind::OpenAiCompatible,
            &secret(""),
            Some("ftp://host".into()),
        );
        assert!(matches!(ftp, Err(CoreError::Invalid(_))));
        let ok = validate_provider(
            ProviderKind::OpenAiCompatible,
            &secret(""),
            Some("http://localhost:11434/v1/".into()),
        )
        .unwrap();
        assert_eq!(ok.as_deref(), Some("http://localhost:11434/v1"));
    }

    #[test]
    fn a_hosted_provider_needs_a_key_and_ignores_the_base_url() {
        for kind in [ProviderKind::Anthropic, ProviderKind::Mistral] {
            assert!(validate_provider(kind, &secret(" "), None).is_err());
            let kept = validate_provider(kind, &secret("sk-key"), Some("http://x".into())).unwrap();
            assert_eq!(kept, None);
        }
    }

    fn model(provider: ProviderKind, id: &str, chat: bool, embeddings: bool) -> CatalogModel {
        CatalogModel {
            provider,
            id: id.into(),
            display_name: id.into(),
            chat,
            embeddings,
            transcription: id.contains("voxtral") || id.contains("whisper"),
            context_window: None,
            input_price_per_mtok_micros: None,
            output_price_per_mtok_micros: None,
        }
    }

    fn reference(provider: ProviderKind, id: &str) -> ModelRef {
        ModelRef {
            provider,
            model: id.into(),
        }
    }

    fn assignments() -> ModelAssignments {
        let chat = reference(ProviderKind::Anthropic, "claude-haiku-5-5");
        ModelAssignments {
            extractor: chat.clone(),
            judge: chat.clone(),
            writer: chat,
            embeddings: reference(ProviderKind::OpenAiCompatible, "bge-m3"),
            transcriber: None,
            executor: Default::default(),
        }
    }

    fn catalog() -> Vec<CatalogModel> {
        vec![
            model(ProviderKind::Anthropic, "claude-haiku-5-5", true, false),
            model(ProviderKind::OpenAiCompatible, "bge-m3", false, true),
        ]
    }

    #[test]
    fn a_consistent_choice_is_accepted() {
        validate_assignments(&assignments(), &catalog()).unwrap();
    }

    #[test]
    fn an_unknown_model_is_refused() {
        let mut chosen = assignments();
        chosen.judge.model = "claude-nope".into();
        assert!(matches!(
            validate_assignments(&chosen, &catalog()),
            Err(CoreError::Invalid(_))
        ));
    }

    #[test]
    fn embeddings_need_an_embeddings_model_and_chat_roles_a_chat_model() {
        let mut chosen = assignments();
        chosen.embeddings = reference(ProviderKind::Anthropic, "claude-haiku-5-5");
        assert!(validate_assignments(&chosen, &catalog()).is_err());

        let mut chosen = assignments();
        chosen.writer = reference(ProviderKind::OpenAiCompatible, "bge-m3");
        assert!(validate_assignments(&chosen, &catalog()).is_err());

        let mut chosen = assignments();
        chosen.embeddings = reference(ProviderKind::OpenAiCompatible, "claude-haiku-5-5");
        assert!(validate_assignments(&chosen, &catalog()).is_err());
    }

    #[test]
    fn a_transcriber_needs_a_transcription_model_of_a_non_anthropic_provider() {
        let catalog = vec![
            model(ProviderKind::Anthropic, "claude-haiku-5-5", true, false),
            model(ProviderKind::OpenAiCompatible, "bge-m3", false, true),
            model(ProviderKind::Mistral, "voxtral-mini-latest", false, false),
            model(ProviderKind::Mistral, "mistral-small", true, false),
        ];
        let mut chosen = assignments();
        chosen.transcriber = Some(reference(ProviderKind::Mistral, "voxtral-mini-latest"));
        validate_assignments(&chosen, &catalog).unwrap();

        chosen.transcriber = Some(reference(ProviderKind::Mistral, "mistral-small"));
        assert!(validate_assignments(&chosen, &catalog).is_err());

        chosen.transcriber = Some(reference(ProviderKind::Anthropic, "claude-haiku-5-5"));
        assert!(matches!(
            validate_assignments(&chosen, &catalog),
            Err(CoreError::Invalid(message)) if message.contains("no transcription")
        ));
    }

    struct Allow;

    impl Authorizer for Allow {
        async fn evaluate(&self, _: &AccessRequest) -> Result<Decision, CoreError> {
            Ok(Decision::Allow)
        }
    }

    #[derive(Clone, Default)]
    struct Memory {
        credentials: Arc<Mutex<HashMap<ProviderKind, ProviderCredential>>>,
        assigned: Arc<Mutex<Option<ModelAssignments>>>,
    }

    impl ProviderRepository for Memory {
        async fn upsert(&mut self, credential: &ProviderCredential) -> Result<(), CoreError> {
            if let Ok(mut credentials) = self.credentials.lock() {
                credentials.insert(credential.kind, credential.clone());
            }
            Ok(())
        }

        async fn list(&mut self, _: OrganizationId) -> Result<Vec<ProviderSummary>, CoreError> {
            let credentials = self.credentials.lock().map_err(poisoned)?;
            Ok(credentials.values().map(ProviderSummary::from).collect())
        }

        async fn find(
            &mut self,
            _: OrganizationId,
            kind: ProviderKind,
        ) -> Result<Option<ProviderCredential>, CoreError> {
            let credentials = self.credentials.lock().map_err(poisoned)?;
            Ok(credentials.get(&kind).cloned())
        }

        async fn delete(
            &mut self,
            _: OrganizationId,
            kind: ProviderKind,
        ) -> Result<bool, CoreError> {
            let mut credentials = self.credentials.lock().map_err(poisoned)?;
            Ok(credentials.remove(&kind).is_some())
        }
    }

    impl ModelAssignmentRepository for Memory {
        async fn get(&mut self, _: OrganizationId) -> Result<Option<ModelAssignments>, CoreError> {
            Ok(self.assigned.lock().map_err(poisoned)?.clone())
        }

        async fn put(
            &mut self,
            _: OrganizationId,
            assignments: &ModelAssignments,
            _: &str,
        ) -> Result<(), CoreError> {
            *self.assigned.lock().map_err(poisoned)? = Some(assignments.clone());
            Ok(())
        }
    }

    fn poisoned<T>(_: std::sync::PoisonError<T>) -> CoreError {
        CoreError::Internal("poisoned".into())
    }

    fn subject() -> Subject {
        Subject {
            id: "admin-1".into(),
            organization_id: OrganizationId::new(),
            actor: None,
            is_admin: true,
        }
    }

    #[tokio::test]
    async fn a_stored_provider_lists_with_its_preview_only() {
        let memory = Memory::default();
        let subject = subject();
        let summary = ProviderService::new(memory.clone(), memory.clone(), &Allow)
            .put(PutProvider {
                subject: subject.clone(),
                kind: ProviderKind::Anthropic,
                secret: secret("sk-ant-api03-abcdefgh1234"),
                base_url: None,
            })
            .await
            .unwrap();
        assert_eq!(summary.preview, "…1234");
        assert!(!format!("{summary:?}").contains("abcdefgh"));
        let listed = ProviderService::new(memory.clone(), memory, &Allow)
            .list(subject)
            .await
            .unwrap();
        assert_eq!(listed, vec![summary]);
    }

    #[tokio::test]
    async fn assigning_a_provider_that_is_not_configured_is_refused() {
        let memory = Memory::default();
        let result = ProviderService::new(memory.clone(), memory, &Allow)
            .assign(subject(), assignments())
            .await;
        assert!(matches!(result, Err(CoreError::Invalid(_))));
    }

    #[tokio::test]
    async fn an_anthropic_transcriber_is_refused_even_when_anthropic_is_configured() {
        let memory = Memory::default();
        let subject = subject();
        for (kind, secret) in [
            (ProviderKind::Anthropic, "sk-ant"),
            (ProviderKind::OpenAiCompatible, ""),
        ] {
            ProviderService::new(memory.clone(), memory.clone(), &Allow)
                .put(PutProvider {
                    subject: subject.clone(),
                    kind,
                    secret: self::secret(secret),
                    base_url: Some("http://localhost:11434/v1".into()),
                })
                .await
                .unwrap();
        }
        let mut chosen = assignments();
        chosen.transcriber = Some(reference(ProviderKind::Anthropic, "claude-haiku-5-5"));
        let result = ProviderService::new(memory.clone(), memory, &Allow)
            .assign(subject, chosen)
            .await;
        assert!(matches!(result, Err(CoreError::Invalid(_))));
    }

    #[tokio::test]
    async fn the_transcriber_target_needs_an_assignment_and_a_key() {
        let memory = Memory::default();
        let subject = subject();
        let none = ProviderService::new(memory.clone(), memory.clone(), &Allow)
            .transcriber_target(subject.clone())
            .await
            .unwrap();
        assert_eq!(none, None);

        ProviderService::new(memory.clone(), memory.clone(), &Allow)
            .put(PutProvider {
                subject: subject.clone(),
                kind: ProviderKind::Mistral,
                secret: secret("sk-mistral"),
                base_url: None,
            })
            .await
            .unwrap();
        let mut chosen = assignments();
        chosen.transcriber = Some(reference(ProviderKind::Mistral, "voxtral-mini-latest"));
        *memory.assigned.lock().unwrap() = Some(chosen.clone());
        let target = ProviderService::new(memory.clone(), memory.clone(), &Allow)
            .transcriber_target(subject.clone())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.model.model, "voxtral-mini-latest");
        assert_eq!(
            target.access.map(|access| access.kind),
            Some(ProviderKind::Mistral)
        );

        memory.credentials.lock().unwrap().clear();
        let keyless = ProviderService::new(memory.clone(), memory, &Allow)
            .transcriber_target(subject)
            .await
            .unwrap();
        assert_eq!(keyless, None);
    }

    #[tokio::test]
    async fn deleting_a_missing_provider_is_not_found() {
        let memory = Memory::default();
        let result = ProviderService::new(memory.clone(), memory, &Allow)
            .delete(subject(), ProviderKind::Mistral)
            .await;
        assert!(matches!(result, Err(CoreError::NotFound)));
    }
}
