use capitainerie_common::{CoreError, OrganizationId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::{Action, Subject},
        provider::{
            ModelAssignments, ProviderAccess, ProviderKind, ProviderSummary,
            ports::{CatalogModel, ModelAssignmentRepository, ModelCatalog, ProviderRepository},
            service::{ProviderService, PutProvider, validate_assignments},
        },
    },
    infrastructure::provider::postgres::PgProviderRepository,
};

/// What the configured providers announce, and which of them failed to answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelListing {
    pub models: Vec<CatalogModel>,
    pub failures: Vec<ProviderFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderFailure {
    pub provider: ProviderKind,
    pub message: String,
}

impl CapitainerieUseCase {
    #[transactional(model_assignment, authz)]
    pub async fn list_providers(
        &self,
        subject: Subject,
    ) -> Result<Vec<ProviderSummary>, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .list(subject)
            .await
    }

    #[transactional(model_assignment, authz)]
    pub async fn put_provider(&self, command: PutProvider) -> Result<ProviderSummary, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .put(command)
            .await
    }

    #[transactional(model_assignment, authz)]
    pub async fn delete_provider(
        &self,
        subject: Subject,
        kind: ProviderKind,
    ) -> Result<(), CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .delete(subject, kind)
            .await
    }

    /// Calls the provider with the stored key; the transaction is closed
    /// before the network call.
    pub async fn test_provider<C: ModelCatalog>(
        &self,
        subject: Subject,
        kind: ProviderKind,
        catalog: &C,
    ) -> Result<Vec<CatalogModel>, CoreError> {
        let access = self.manageable_provider(subject, kind).await?;
        catalog.list_models(&access).await
    }

    pub async fn list_models<C: ModelCatalog>(
        &self,
        subject: Subject,
        catalog: &C,
    ) -> Result<ModelListing, CoreError> {
        let accesses = self.accesses(subject, Action::ProviderRead).await?;
        let mut listing = ModelListing::default();
        for access in &accesses {
            match catalog.list_models(access).await {
                Ok(models) => listing.models.extend(models),
                Err(error) => listing.failures.push(ProviderFailure {
                    provider: access.kind,
                    message: error.to_string(),
                }),
            }
        }
        Ok(listing)
    }

    #[transactional(model_assignment, authz)]
    pub async fn get_model_assignments(
        &self,
        subject: Subject,
    ) -> Result<Option<ModelAssignments>, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .assignments(subject)
            .await
    }

    /// Checks each choice against what its provider announces right now.
    pub async fn put_model_assignments<C: ModelCatalog>(
        &self,
        subject: Subject,
        assignments: ModelAssignments,
        catalog: &C,
    ) -> Result<ModelAssignments, CoreError> {
        let accesses = self.accesses(subject.clone(), Action::ModelsAssign).await?;
        let mut announced = Vec::new();
        for access in &accesses {
            let chosen = [
                &assignments.extractor,
                &assignments.judge,
                &assignments.writer,
                &assignments.embeddings,
            ]
            .iter()
            .any(|model| model.provider == access.kind);
            if !chosen {
                continue;
            }
            let models = catalog.list_models(access).await.map_err(|error| {
                CoreError::Invalid(format!(
                    "cannot list the models of {}: {error}",
                    access.kind.as_str()
                ))
            })?;
            announced.extend(models);
        }
        validate_assignments(&assignments, &announced)?;
        self.store_assignments(subject, assignments).await
    }

    /// For the worker, which acts for no user: reads the stored key without a
    /// policy decision. Never reachable from an API handler.
    pub async fn provider_access(
        &self,
        organization_id: OrganizationId,
        kind: ProviderKind,
    ) -> Result<Option<ProviderAccess>, CoreError> {
        Ok(self
            .stored_credential(organization_id, kind)
            .await?
            .map(ProviderAccess::from))
    }

    /// For the worker, which acts for no user: the organization's choice, if any.
    #[transactional(model_assignment)]
    pub async fn model_assignments_for(
        &self,
        organization_id: OrganizationId,
    ) -> Result<Option<ModelAssignments>, CoreError> {
        let mut model_assignment_repository = model_assignment_repository;
        model_assignment_repository.get(organization_id).await
    }

    #[transactional()]
    async fn stored_credential(
        &self,
        organization_id: OrganizationId,
        kind: ProviderKind,
    ) -> Result<Option<crate::domain::provider::ProviderCredential>, CoreError> {
        let mut provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        provider_repository.find(organization_id, kind).await
    }

    /// The stored key of a provider, for an admin about to try it. The
    /// transaction is closed when this returns.
    #[transactional(model_assignment, authz)]
    pub async fn manageable_provider(
        &self,
        subject: Subject,
        kind: ProviderKind,
    ) -> Result<ProviderAccess, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .access_to_manage(subject, kind)
            .await
    }

    #[transactional(model_assignment, authz)]
    async fn accesses(
        &self,
        subject: Subject,
        action: Action,
    ) -> Result<Vec<ProviderAccess>, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .accesses(subject, action)
            .await
    }

    #[transactional(model_assignment, authz)]
    async fn store_assignments(
        &self,
        subject: Subject,
        assignments: ModelAssignments,
    ) -> Result<ModelAssignments, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .assign(subject, assignments)
            .await
    }
}
