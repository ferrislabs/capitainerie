use capitainerie_common::{
    CardId, ClientId, CoreError, IntegrationActionId, IntegrationInstanceId, OrganizationId, RunId,
};
use serde_json::Value;

use super::{
    ConnectorFailure, InstanceConnection, IntegrationInstance, ProbeReport, action::ActionRequest,
    intervention::Intervention, journal::IntegrationAction,
};
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

    fn perform(
        &self,
        connection: &InstanceConnection,
        request: &ActionRequest,
    ) -> impl Future<Output = Result<Value, ConnectorFailure>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait IntegrationActionRepository: Send {
    #[allow(clippy::needless_lifetimes)]
    fn insert<'a>(
        &mut self,
        action: &IntegrationAction,
        snapshot: Option<&'a Value>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn complete(
        &mut self,
        action: &IntegrationAction,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
    ) -> impl Future<Output = Result<Option<IntegrationAction>, CoreError>> + Send;

    fn snapshot(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
    ) -> impl Future<Output = Result<Option<Value>, CoreError>> + Send;

    fn list_for_run(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> impl Future<Output = Result<Vec<IntegrationAction>, CoreError>> + Send;

    fn list_for_card(
        &mut self,
        organization_id: OrganizationId,
        card_id: CardId,
    ) -> impl Future<Output = Result<Vec<IntegrationAction>, CoreError>> + Send;

    fn mark_reverted(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
        by: IntegrationActionId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn claim_revert(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
    ) -> impl Future<Output = Result<bool, CoreError>> + Send;

    fn release_revert(
        &mut self,
        organization_id: OrganizationId,
        id: IntegrationActionId,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn count_writes_not_denied(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> impl Future<Output = Result<i64, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait InterventionRepository: Send {
    fn insert(
        &mut self,
        intervention: &Intervention,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> impl Future<Output = Result<Option<Intervention>, CoreError>> + Send;
}
