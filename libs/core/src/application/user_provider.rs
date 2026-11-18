use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::Subject,
        provider::{ProviderKind, ports::ModelCatalog},
        user_provider::{
            UserProviderCredential, UserProviderSummary,
            ports::UserProviderRepository,
            service::{PutUserProvider, UserProviderService},
        },
    },
    infrastructure::user_provider::postgres::PgUserProviderRepository,
};

const FAILURE_MAX_CHARS: usize = 300;

fn scrubbed(message: &str, secret: &str) -> String {
    let cleaned = match secret {
        "" => message.to_owned(),
        secret => message.replace(secret, "***"),
    };
    cleaned.chars().take(FAILURE_MAX_CHARS).collect()
}

impl CapitainerieUseCase {
    #[transactional(authz)]
    pub async fn list_user_providers(
        &self,
        subject: Subject,
    ) -> Result<Vec<UserProviderSummary>, CoreError> {
        let repository = PgUserProviderRepository::new(&tx, self.secrets.as_deref());
        UserProviderService::new(repository, authz)
            .list(subject)
            .await
    }

    #[transactional(authz)]
    pub async fn put_user_provider(
        &self,
        command: PutUserProvider,
    ) -> Result<UserProviderSummary, CoreError> {
        let repository = PgUserProviderRepository::new(&tx, self.secrets.as_deref());
        UserProviderService::new(repository, authz)
            .put(command)
            .await
    }

    #[transactional(authz)]
    pub async fn delete_user_provider(
        &self,
        subject: Subject,
        kind: ProviderKind,
    ) -> Result<(), CoreError> {
        let repository = PgUserProviderRepository::new(&tx, self.secrets.as_deref());
        UserProviderService::new(repository, authz)
            .delete(subject, kind)
            .await
    }

    /// Calls the provider with the caller's own stored key; the transaction is
    /// closed before the network call. A provider failure is the inner `Err`,
    /// with the key scrubbed from its message.
    pub async fn test_user_provider<C: ModelCatalog>(
        &self,
        subject: Subject,
        kind: ProviderKind,
        catalog: &C,
    ) -> Result<Result<(), String>, CoreError> {
        let access = self.own_provider_access(subject, kind).await?;
        Ok(catalog
            .list_models(&access)
            .await
            .map(|_| ())
            .map_err(|error| scrubbed(&error.to_string(), access.secret.expose())))
    }

    #[transactional(authz)]
    async fn own_provider_access(
        &self,
        subject: Subject,
        kind: ProviderKind,
    ) -> Result<crate::domain::provider::ProviderAccess, CoreError> {
        let repository = PgUserProviderRepository::new(&tx, self.secrets.as_deref());
        UserProviderService::new(repository, authz)
            .access(subject, kind)
            .await
    }

    /// For the worker, which acts for no user: every key the initiator of a
    /// run connected. Never reachable from an API handler.
    #[transactional()]
    pub async fn user_credentials_of(
        &self,
        organization_id: OrganizationId,
        subject: &str,
    ) -> Result<Vec<UserProviderCredential>, CoreError> {
        let mut repository = PgUserProviderRepository::new(&tx, self.secrets.as_deref());
        repository.all(organization_id, subject).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_message_never_carries_the_key_and_stays_short() {
        let message = scrubbed(
            "401 invalid key sk-secret-1234 for sk-secret-1234",
            "sk-secret-1234",
        );
        assert_eq!(message, "401 invalid key *** for ***");
        assert_eq!(scrubbed("down", ""), "down");
        assert_eq!(
            scrubbed(&"x".repeat(500), "k").chars().count(),
            FAILURE_MAX_CHARS
        );
    }
}
