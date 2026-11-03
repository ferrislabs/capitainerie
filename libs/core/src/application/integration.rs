use capitainerie_common::{ClientId, CoreError, IntegrationInstanceId};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::Subject,
        integration::{
            ConnectorFailure, InstanceError, IntegrationInstance, ProbeReport,
            ports::IntegrationConnector,
            service::{
                CreateInstance, IntegrationInstanceService, PreparedProbe, PreparedUpdate,
                UpdateInstance,
            },
        },
    },
    infrastructure::integration::postgres::PgIntegrationInstanceRepository,
};

type Stored<T> = Result<Result<T, InstanceError>, CoreError>;

fn settled<T>(outcome: Result<T, InstanceError>) -> Stored<T> {
    match outcome {
        Err(InstanceError::Core(error)) => Err(error),
        other => Ok(other),
    }
}

impl CapitainerieUseCase {
    #[transactional(client, authz)]
    pub async fn list_integration_instances(
        &self,
        subject: Subject,
        client_id: ClientId,
        include_archived: bool,
    ) -> Result<Vec<IntegrationInstance>, CoreError> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        IntegrationInstanceService::new(instance_repository, client_repository, authz)
            .list(subject, client_id, include_archived)
            .await
    }

    pub async fn create_integration_instance<C: IntegrationConnector>(
        &self,
        subject: Subject,
        command: CreateInstance,
        connector: &C,
    ) -> Result<IntegrationInstance, InstanceError> {
        let prepared = self.prepare_integration_instance(subject, command).await?;
        let report = connector.probe(&prepared.connection).await?;
        self.store_integration_instance(prepared, report).await?
    }

    pub async fn update_integration_instance<C: IntegrationConnector>(
        &self,
        subject: Subject,
        id: IntegrationInstanceId,
        command: UpdateInstance,
        connector: &C,
    ) -> Result<IntegrationInstance, InstanceError> {
        let prepared = self
            .prepare_integration_instance_update(subject, id, command)
            .await?;
        let report = match &prepared.probe {
            Some(connection) => Some(connector.probe(connection).await?),
            None => None,
        };
        self.store_integration_instance_update(prepared, report)
            .await?
    }

    pub async fn check_integration_instance<C: IntegrationConnector>(
        &self,
        subject: Subject,
        id: IntegrationInstanceId,
        connector: &C,
    ) -> Result<(IntegrationInstance, Result<ProbeReport, ConnectorFailure>), InstanceError> {
        let prepared = self.prepare_integration_instance_check(subject, id).await?;
        let outcome = connector.probe(&prepared.connection).await;
        let instance = self
            .record_integration_instance_check(prepared.instance, &outcome)
            .await?;
        Ok((instance, outcome))
    }

    #[transactional(client, authz)]
    pub async fn archive_integration_instance(
        &self,
        subject: Subject,
        id: IntegrationInstanceId,
    ) -> Result<IntegrationInstance, CoreError> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        let held = instance_repository
            .held_by_an_open_intervention(subject.organization_id, id)
            .await?;
        IntegrationInstanceService::new(instance_repository, client_repository, authz)
            .archive(subject, id, held)
            .await
    }

    #[transactional(client, authz)]
    async fn prepare_integration_instance(
        &self,
        subject: Subject,
        command: CreateInstance,
    ) -> Result<PreparedProbe, CoreError> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        let prepared =
            IntegrationInstanceService::new(instance_repository, client_repository, authz)
                .prepare_create(subject, command)
                .await?;
        self.require_secrets_key()?;
        Ok(prepared)
    }

    #[transactional(client, authz)]
    async fn store_integration_instance(
        &self,
        prepared: PreparedProbe,
        report: ProbeReport,
    ) -> Stored<IntegrationInstance> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        settled(
            IntegrationInstanceService::new(instance_repository, client_repository, authz)
                .save_create(prepared, report)
                .await,
        )
    }

    #[transactional(client, authz)]
    async fn prepare_integration_instance_update(
        &self,
        subject: Subject,
        id: IntegrationInstanceId,
        command: UpdateInstance,
    ) -> Result<PreparedUpdate, CoreError> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        let prepared =
            IntegrationInstanceService::new(instance_repository, client_repository, authz)
                .prepare_update(subject, id, command)
                .await?;
        if prepared.new_secret {
            self.require_secrets_key()?;
        }
        Ok(prepared)
    }

    #[transactional(client, authz)]
    async fn store_integration_instance_update(
        &self,
        prepared: PreparedUpdate,
        report: Option<ProbeReport>,
    ) -> Stored<IntegrationInstance> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        settled(
            IntegrationInstanceService::new(instance_repository, client_repository, authz)
                .save_update(prepared, report)
                .await,
        )
    }

    #[transactional(client, authz)]
    async fn prepare_integration_instance_check(
        &self,
        subject: Subject,
        id: IntegrationInstanceId,
    ) -> Result<PreparedProbe, CoreError> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        IntegrationInstanceService::new(instance_repository, client_repository, authz)
            .prepare_check(subject, id)
            .await
    }

    #[transactional(client, authz)]
    async fn record_integration_instance_check(
        &self,
        instance: IntegrationInstance,
        outcome: &Result<ProbeReport, ConnectorFailure>,
    ) -> Result<IntegrationInstance, CoreError> {
        let instance_repository =
            PgIntegrationInstanceRepository::new(&tx, self.secrets.as_deref());
        IntegrationInstanceService::new(instance_repository, client_repository, authz)
            .record_check(instance, outcome)
            .await
    }

    fn require_secrets_key(&self) -> Result<(), CoreError> {
        self.secrets
            .as_deref()
            .map(|_| ())
            .ok_or_else(|| CoreError::Internal("SECRETS_KEY is not configured".into()))
    }
}
