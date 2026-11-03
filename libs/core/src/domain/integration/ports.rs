use capitainerie_common::{ClientId, CoreError, IntegrationInstanceId, OrganizationId};

use super::{ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport};
use crate::domain::agent::SecretToken;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait IntegrationInstanceRepository: Send {
    fn insert(
        &mut self,
        instance: &IntegrationInstance,
        secret: &SecretToken,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    #[allow(clippy::needless_lifetimes)]
    fn update<'a>(
        &mut self,
        instance: &IntegrationInstance,
        secret: Option<&'a SecretToken>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationInstanceId,
    ) -> impl Future<Output = Result<Option<IntegrationInstance>, CoreError>> + Send;

    fn connection(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationInstanceId,
    ) -> impl Future<Output = Result<Option<InstanceConnection>, CoreError>> + Send;

    fn list_for_client(
        &mut self,
        organization_id: OrganizationId,
        client_id: ClientId,
        include_archived: bool,
    ) -> impl Future<Output = Result<Vec<IntegrationInstance>, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait IntegrationConnector: Send + Sync {
    fn probe(
        &self,
        connection: &InstanceConnection,
    ) -> impl Future<Output = Result<ProbeReport, ConnectorFailure>> + Send;
}
