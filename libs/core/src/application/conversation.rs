use capitainerie_common::{ConversationId, CoreError};
use capitainerie_macros::transactional;
use chrono::{DateTime, Utc};

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::{AccessRequest, Action, ResourceKind, Subject, require},
        conversation::{
            Conversation, Message,
            service::{ConversationService, StartedTurn, TurnOutcome, UpdateConversation},
            turn::TurnError,
        },
        execution::{GithubInstallation, ports::GithubInstallationRepository},
        provider::{ModelTarget, service::ProviderService},
    },
    infrastructure::provider::postgres::PgProviderRepository,
};

impl CapitainerieUseCase {
    #[transactional(conversation, conversation_message, authz)]
    pub async fn create_conversation(
        &self,
        subject: Subject,
        message: String,
    ) -> Result<Conversation, CoreError> {
        ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .create(subject, message)
        .await
    }

    #[transactional(conversation, conversation_message, authz)]
    pub async fn list_conversations(
        &self,
        subject: Subject,
    ) -> Result<Vec<Conversation>, CoreError> {
        ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .list(subject)
        .await
    }

    #[transactional(conversation, conversation_message, authz)]
    pub async fn get_conversation(
        &self,
        subject: Subject,
        id: ConversationId,
    ) -> Result<(Conversation, Vec<Message>), CoreError> {
        ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .get(subject, id)
        .await
    }

    #[transactional(conversation, conversation_message, authz)]
    pub async fn check_conversation_writable(
        &self,
        subject: Subject,
        id: ConversationId,
    ) -> Result<Conversation, CoreError> {
        ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .check_writable(subject, id)
        .await
    }

    #[transactional(conversation, conversation_message, authz)]
    pub async fn update_conversation(
        &self,
        subject: Subject,
        id: ConversationId,
        command: UpdateConversation,
    ) -> Result<Conversation, CoreError> {
        ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .update(subject, id, command)
        .await
    }

    /// Takes the conversation's turn slot and stores the member's message,
    /// committed before any model call.
    pub async fn start_conversation_turn(
        &self,
        subject: Subject,
        id: ConversationId,
        message: Option<String>,
    ) -> Result<StartedTurn, TurnError> {
        self.start_turn_stored(subject, id, message)
            .await
            .map_err(TurnError::from)?
    }

    #[transactional(conversation, conversation_message, authz)]
    async fn start_turn_stored(
        &self,
        subject: Subject,
        id: ConversationId,
        message: Option<String>,
    ) -> Result<Result<StartedTurn, TurnError>, CoreError> {
        match ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .start_turn(subject, id, message)
        .await
        {
            Ok(started) => Ok(Ok(started)),
            Err(TurnError::Core(error)) => Err(error),
            Err(refusal) => Ok(Err(refusal)),
        }
    }

    #[transactional(conversation, conversation_message, authz)]
    pub async fn finish_conversation_turn(
        &self,
        subject: Subject,
        id: ConversationId,
        leased_at: DateTime<Utc>,
        outcome: TurnOutcome,
    ) -> Result<(), CoreError> {
        ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .finish_turn(subject, id, leased_at, outcome)
        .await
    }

    #[transactional(conversation, conversation_message, authz)]
    pub async fn abandon_conversation_turn(
        &self,
        subject: Subject,
        id: ConversationId,
        leased_at: DateTime<Utc>,
    ) -> Result<(), CoreError> {
        ConversationService::new(
            conversation_repository,
            conversation_message_repository,
            authz,
        )
        .abandon_turn(subject, id, leased_at)
        .await
    }

    #[transactional(model_assignment, authz)]
    pub async fn captain_target(&self, subject: Subject) -> Result<Option<ModelTarget>, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .captain_target(subject)
            .await
    }

    /// The GitHub installation of the subject's organization, read through the
    /// policy decision point so the Captain's reads are decided and audited.
    #[transactional(github_installation, authz)]
    pub async fn read_github_installation(
        &self,
        subject: Subject,
    ) -> Result<Option<GithubInstallation>, CoreError> {
        let organization_id = subject.organization_id;
        require(
            authz,
            AccessRequest::new(
                subject,
                Action::GithubRead,
                ResourceKind::GithubInstallation,
                None,
            ),
        )
        .await?;
        let mut github_installation_repository = github_installation_repository;
        github_installation_repository.get(organization_id).await
    }
}
