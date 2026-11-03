use capitainerie_common::{ClientId, IntegrationInstanceId};
use capitainerie_core::domain::integration::{
    ConnectorFailure, Environment, IntegrationInstance, IntegrationKind, ProbeReport,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationKindDto {
    OdooJson2,
    #[serde(rename = "odoo_jsonrpc")]
    OdooJsonRpc,
    N8n,
}

impl From<IntegrationKind> for IntegrationKindDto {
    fn from(kind: IntegrationKind) -> Self {
        match kind {
            IntegrationKind::OdooJson2 => Self::OdooJson2,
            IntegrationKind::OdooJsonRpc => Self::OdooJsonRpc,
            IntegrationKind::N8n => Self::N8n,
        }
    }
}

impl From<IntegrationKindDto> for IntegrationKind {
    fn from(kind: IntegrationKindDto) -> Self {
        match kind {
            IntegrationKindDto::OdooJson2 => Self::OdooJson2,
            IntegrationKindDto::OdooJsonRpc => Self::OdooJsonRpc,
            IntegrationKindDto::N8n => Self::N8n,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationEnvironmentDto {
    Production,
    Staging,
    Development,
}

impl From<Environment> for IntegrationEnvironmentDto {
    fn from(environment: Environment) -> Self {
        match environment {
            Environment::Production => Self::Production,
            Environment::Staging => Self::Staging,
            Environment::Development => Self::Development,
        }
    }
}

impl From<IntegrationEnvironmentDto> for Environment {
    fn from(environment: IntegrationEnvironmentDto) -> Self {
        match environment {
            IntegrationEnvironmentDto::Production => Self::Production,
            IntegrationEnvironmentDto::Staging => Self::Staging,
            IntegrationEnvironmentDto::Development => Self::Development,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntegrationInstanceResponse {
    pub id: IntegrationInstanceId,
    pub client_id: ClientId,
    pub kind: IntegrationKindDto,
    pub name: String,
    pub environment: IntegrationEnvironmentDto,
    pub base_url: String,
    pub database: Option<String>,
    pub login: Option<String>,
    pub secret_preview: String,
    pub server_version: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl From<IntegrationInstance> for IntegrationInstanceResponse {
    fn from(instance: IntegrationInstance) -> Self {
        Self {
            id: instance.id,
            client_id: instance.client_id,
            kind: instance.kind.into(),
            name: instance.name.as_str().to_owned(),
            environment: instance.environment.into(),
            base_url: instance.base_url.as_str().to_owned(),
            database: instance.database,
            login: instance.login,
            secret_preview: instance.secret_preview,
            server_version: instance.server_version,
            checked_at: instance.checked_at,
            created_by: instance.created_by,
            created_at: instance.created_at,
            updated_at: instance.updated_at,
            archived_at: instance.archived_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntegrationInstanceListResponse {
    pub items: Vec<IntegrationInstanceResponse>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateIntegrationInstanceRequest {
    pub kind: IntegrationKindDto,
    pub name: String,
    pub environment: IntegrationEnvironmentDto,
    pub base_url: String,
    pub database: Option<String>,
    pub login: Option<String>,
    pub secret: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateIntegrationInstanceRequest {
    pub name: Option<String>,
    pub environment: Option<IntegrationEnvironmentDto>,
    pub base_url: Option<String>,
    pub database: Option<String>,
    pub login: Option<String>,
    pub secret: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InstanceFailureResponse {
    pub code: &'static str,
    pub message: String,
}

impl From<&ConnectorFailure> for InstanceFailureResponse {
    fn from(failure: &ConnectorFailure) -> Self {
        Self {
            code: failure.code(),
            message: failure.to_string(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckIntegrationInstanceResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<InstanceFailureResponse>,
    pub instance: IntegrationInstanceResponse,
}

impl CheckIntegrationInstanceResponse {
    pub fn new(
        instance: IntegrationInstance,
        outcome: Result<ProbeReport, ConnectorFailure>,
    ) -> Self {
        let instance = instance.into();
        match outcome {
            Ok(report) => Self {
                ok: true,
                server_version: Some(report.server_version),
                warning: report.warning,
                error: None,
                instance,
            },
            Err(failure) => Self {
                ok: false,
                server_version: None,
                warning: None,
                error: Some(InstanceFailureResponse::from(&failure)),
                instance,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_are_spelled_as_the_core_spells_them() {
        for kind in IntegrationKind::ALL {
            let dto = IntegrationKindDto::from(kind);
            assert_eq!(serde_json::to_value(dto).unwrap(), kind.as_str());
            assert_eq!(
                serde_json::from_value::<IntegrationKindDto>(kind.as_str().into()).unwrap(),
                dto
            );
            assert_eq!(IntegrationKind::from(dto), kind);
        }
        for environment in Environment::ALL {
            let dto = IntegrationEnvironmentDto::from(environment);
            assert_eq!(serde_json::to_value(dto).unwrap(), environment.as_str());
            assert_eq!(Environment::from(dto), environment);
        }
    }
}
