use capitainerie_common::{CoreError, OrganizationId};

use super::{ModelAssignments, ProviderAccess, ProviderCredential, ProviderKind, ProviderSummary};

pub trait ProviderRepository: Send {
    fn upsert(
        &mut self,
        credential: &ProviderCredential,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn list(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<Vec<ProviderSummary>, CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        kind: ProviderKind,
    ) -> impl Future<Output = Result<Option<ProviderCredential>, CoreError>> + Send;

    fn delete(
        &mut self,
        organization_id: OrganizationId,
        kind: ProviderKind,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;
}

pub trait ModelAssignmentRepository: Send {
    fn get(
        &mut self,
        organization_id: OrganizationId,
    ) -> impl Future<Output = Result<Option<ModelAssignments>, CoreError>> + Send;

    fn put(
        &mut self,
        organization_id: OrganizationId,
        assignments: &ModelAssignments,
        updated_by: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogModel {
    pub provider: ProviderKind,
    pub id: String,
    pub display_name: String,
    pub chat: bool,
    pub embeddings: bool,
    pub context_window: Option<u64>,
    pub input_price_per_mtok_micros: Option<u64>,
    pub output_price_per_mtok_micros: Option<u64>,
}

/// What a provider announces. Implemented by the runtime adapter, which knows
/// each provider's API; the core only consumes it.
pub trait ModelCatalog: Send + Sync {
    fn list_models(
        &self,
        access: &ProviderAccess,
    ) -> impl Future<Output = Result<Vec<CatalogModel>, CoreError>> + Send;
}
