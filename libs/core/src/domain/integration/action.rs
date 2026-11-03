use capitainerie_common::{CoreError, IntegrationInstanceId};
use serde_json::{Map, Value};

use super::{IntegrationKind, catalog as parse};
use crate::domain::conversation::turn::ToolSpec;

pub const ODOO_MAX_IDS: usize = 200;
pub const ODOO_MAX_DELETE_IDS: usize = 50;
pub const ODOO_MAX_LIMIT: u32 = 200;
pub const ODOO_DEFAULT_LIMIT: u32 = 50;
pub const ODOO_MAX_FIELDS: usize = 100;
pub const N8N_MAX_LIMIT: u32 = 100;
pub const N8N_DEFAULT_LIMIT: u32 = 20;
pub const ATTACHMENT_MAX_BYTES: usize = 7 * 1024 * 1024;
pub const ATTACHMENT_MAX_CHARS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Access {
    Read,
    Write,
}

impl Access {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }

    pub fn allows(self, needed: Access) -> bool {
        matches!((self, needed), (Self::Write, _) | (Self::Read, Self::Read))
    }
}

impl TryFrom<&str> for Access {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            other => Err(CoreError::Invalid(format!("unknown access {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Odoo,
    N8n,
}

impl IntegrationKind {
    pub fn family(self) -> Family {
        match self {
            Self::OdooJson2 | Self::OdooJsonRpc => Family::Odoo,
            Self::N8n => Family::N8n,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OdooModel(String);

impl OdooModel {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for OdooModel {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let valid = !value.is_empty()
            && value.len() <= 128
            && value.split('.').all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            });
        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err(CoreError::Invalid(format!(
                "{value:?} is not an Odoo model name (like res.partner)"
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OdooMethod(String);

impl OdooMethod {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for OdooMethod {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut chars = value.chars();
        let valid = value.len() <= 128
            && chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err(CoreError::Invalid(format!(
                "{value:?} is not a public Odoo method name (private methods start with _ and are refused)"
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct N8nId(String);

impl N8nId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for N8nId {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err(CoreError::Invalid(format!("{value:?} is not an n8n id")))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    Canceled,
    Error,
    Running,
    Success,
    Waiting,
}

impl ExecutionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canceled => "canceled",
            Self::Error => "error",
            Self::Running => "running",
            Self::Success => "success",
            Self::Waiting => "waiting",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionRequest {
    OdooListModels {
        query: Option<String>,
        limit: u32,
    },
    OdooDescribeModel {
        model: OdooModel,
    },
    OdooSearchRead {
        model: OdooModel,
        domain: Vec<Value>,
        fields: Vec<String>,
        limit: u32,
        offset: u32,
        order: Option<String>,
    },
    OdooRead {
        model: OdooModel,
        ids: Vec<i64>,
        fields: Vec<String>,
    },
    OdooCount {
        model: OdooModel,
        domain: Vec<Value>,
    },
    OdooReadAttachment {
        attachment_id: i64,
    },
    OdooCreate {
        model: OdooModel,
        values: Map<String, Value>,
    },
    OdooWrite {
        model: OdooModel,
        ids: Vec<i64>,
        values: Map<String, Value>,
    },
    OdooDelete {
        model: OdooModel,
        ids: Vec<i64>,
    },
    OdooCallMethod {
        model: OdooModel,
        method: OdooMethod,
        ids: Vec<i64>,
        args: Vec<Value>,
        kwargs: Map<String, Value>,
    },
    N8nListWorkflows {
        name: Option<String>,
        tags: Option<String>,
        active: Option<bool>,
        project_id: Option<N8nId>,
        limit: u32,
        cursor: Option<String>,
    },
    N8nGetWorkflow {
        workflow_id: N8nId,
    },
    N8nListWorkflowVersions {
        workflow_id: N8nId,
    },
    N8nGetWorkflowTags {
        workflow_id: N8nId,
    },
    N8nListExecutions {
        workflow_id: Option<N8nId>,
        status: Option<ExecutionStatus>,
        limit: u32,
        cursor: Option<String>,
    },
    N8nGetExecution {
        execution_id: N8nId,
        include_data: bool,
    },
    N8nListTags {
        limit: u32,
        cursor: Option<String>,
    },
    N8nListCredentials {
        limit: u32,
        cursor: Option<String>,
    },
    N8nCreateWorkflow {
        name: String,
        nodes: Vec<Value>,
        connections: Map<String, Value>,
        settings: Map<String, Value>,
    },
    N8nUpdateWorkflow {
        workflow_id: N8nId,
        name: Option<String>,
        nodes: Vec<Value>,
        connections: Map<String, Value>,
        settings: Option<Map<String, Value>>,
        publish: bool,
    },
    N8nPublishWorkflow {
        workflow_id: N8nId,
    },
    N8nUnpublishWorkflow {
        workflow_id: N8nId,
    },
    N8nArchiveWorkflow {
        workflow_id: N8nId,
    },
    N8nUnarchiveWorkflow {
        workflow_id: N8nId,
    },
    N8nSetWorkflowTags {
        workflow_id: N8nId,
        tag_ids: Vec<N8nId>,
    },
    N8nCreateTag {
        name: String,
    },
    N8nDeleteTag {
        tag_id: N8nId,
    },
    N8nRetryExecution {
        execution_id: N8nId,
    },
    N8nStopExecution {
        execution_id: N8nId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionCall {
    pub instance: IntegrationInstanceId,
    pub request: ActionRequest,
}

impl ActionCall {
    pub fn parse(tool: &str, arguments: &Value) -> Result<Self, CoreError> {
        parse::call(tool, arguments)
    }
}

impl ActionRequest {
    pub fn tool(&self) -> &'static str {
        parse::tool_of(self)
    }

    pub fn access(&self) -> Access {
        match self {
            Self::OdooListModels { .. }
            | Self::OdooDescribeModel { .. }
            | Self::OdooSearchRead { .. }
            | Self::OdooRead { .. }
            | Self::OdooCount { .. }
            | Self::OdooReadAttachment { .. }
            | Self::N8nListWorkflows { .. }
            | Self::N8nGetWorkflow { .. }
            | Self::N8nListWorkflowVersions { .. }
            | Self::N8nGetWorkflowTags { .. }
            | Self::N8nListExecutions { .. }
            | Self::N8nGetExecution { .. }
            | Self::N8nListTags { .. }
            | Self::N8nListCredentials { .. } => Access::Read,
            Self::OdooCreate { .. }
            | Self::OdooWrite { .. }
            | Self::OdooDelete { .. }
            | Self::OdooCallMethod { .. }
            | Self::N8nCreateWorkflow { .. }
            | Self::N8nUpdateWorkflow { .. }
            | Self::N8nPublishWorkflow { .. }
            | Self::N8nUnpublishWorkflow { .. }
            | Self::N8nArchiveWorkflow { .. }
            | Self::N8nUnarchiveWorkflow { .. }
            | Self::N8nSetWorkflowTags { .. }
            | Self::N8nCreateTag { .. }
            | Self::N8nDeleteTag { .. }
            | Self::N8nRetryExecution { .. }
            | Self::N8nStopExecution { .. } => Access::Write,
        }
    }

    pub fn family(&self) -> Family {
        match self {
            Self::OdooListModels { .. }
            | Self::OdooDescribeModel { .. }
            | Self::OdooSearchRead { .. }
            | Self::OdooRead { .. }
            | Self::OdooCount { .. }
            | Self::OdooReadAttachment { .. }
            | Self::OdooCreate { .. }
            | Self::OdooWrite { .. }
            | Self::OdooDelete { .. }
            | Self::OdooCallMethod { .. } => Family::Odoo,
            _ => Family::N8n,
        }
    }

    pub fn is_public(&self) -> bool {
        !matches!(
            self,
            Self::N8nGetWorkflowTags { .. }
                | Self::N8nUnarchiveWorkflow { .. }
                | Self::N8nDeleteTag { .. }
        )
    }

    pub fn odoo_model(&self) -> Option<&str> {
        match self {
            Self::OdooDescribeModel { model }
            | Self::OdooSearchRead { model, .. }
            | Self::OdooRead { model, .. }
            | Self::OdooCount { model, .. }
            | Self::OdooCreate { model, .. }
            | Self::OdooWrite { model, .. }
            | Self::OdooDelete { model, .. }
            | Self::OdooCallMethod { model, .. } => Some(model.as_str()),
            Self::OdooReadAttachment { .. } => Some("ir.attachment"),
            Self::OdooListModels { .. } => Some("ir.model"),
            _ => None,
        }
    }

    pub fn target(&self) -> Value {
        parse::target_of(self)
    }

    pub fn arguments(&self) -> Value {
        parse::arguments_of(self)
    }

    pub fn before_read(&self) -> Option<ActionRequest> {
        parse::before_read(self)
    }

    pub fn after_read(&self, result: &Value) -> Option<ActionRequest> {
        parse::after_read(self, result)
    }

    pub fn is_revertible(&self, server_version: Option<&str>) -> bool {
        match self {
            Self::OdooCreate { .. }
            | Self::OdooWrite { .. }
            | Self::N8nUpdateWorkflow { .. }
            | Self::N8nPublishWorkflow { .. }
            | Self::N8nUnpublishWorkflow { .. }
            | Self::N8nArchiveWorkflow { .. }
            | Self::N8nSetWorkflowTags { .. }
            | Self::N8nCreateTag { .. } => true,
            Self::N8nCreateWorkflow { .. } => server_version == Some("2"),
            _ => false,
        }
    }
}

pub fn catalog() -> Vec<ToolSpec> {
    parse::tool_specs()
}
