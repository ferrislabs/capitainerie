use capitainerie_common::{CoreError, IntegrationInstanceId};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use super::action::{
    ActionCall, ActionRequest, ExecutionStatus, N8N_DEFAULT_LIMIT, N8N_MAX_LIMIT, N8nId,
    ODOO_DEFAULT_LIMIT, ODOO_MAX_DELETE_IDS, ODOO_MAX_FIELDS, ODOO_MAX_IDS, ODOO_MAX_LIMIT,
    OdooMethod, OdooModel,
};
use crate::domain::conversation::turn::ToolSpec;

const TEXT_MAX_CHARS: usize = 200;
const WORKFLOW_NAME_MAX_CHARS: usize = 128;
const TAG_NAME_MAX_CHARS: usize = 64;
const TAG_IDS_MAX: usize = 50;

const STATUSES: [ExecutionStatus; 5] = [
    ExecutionStatus::Canceled,
    ExecutionStatus::Error,
    ExecutionStatus::Running,
    ExecutionStatus::Success,
    ExecutionStatus::Waiting,
];

const LOGICAL_OPERATORS: [&str; 3] = ["&", "|", "!"];

const COMPARISON_OPERATORS: [&str; 19] = [
    "=",
    "!=",
    ">",
    ">=",
    "<",
    "<=",
    "=?",
    "=like",
    "like",
    "not like",
    "ilike",
    "not ilike",
    "=ilike",
    "in",
    "not in",
    "child_of",
    "parent_of",
    "any",
    "not any",
];

macro_rules! write_note {
    () => {
        " The platform keeps a copy of the state before and refuses instances not granted for writing."
    };
}

#[derive(Clone, Copy)]
enum Schema {
    Instance,
    Model,
    Method,
    Ids(usize, usize),
    Fields,
    Domain,
    Limit(u32, u32),
    Offset,
    Order,
    Values,
    Object(&'static str),
    Args,
    AttachmentId,
    Text(usize, usize),
    Tags,
    N8nId,
    Status,
    Flag,
    Bool,
    Nodes,
    TagIds,
}

impl Schema {
    fn json(self) -> Value {
        match self {
            Self::Instance => json!({
                "type": "string",
                "format": "uuid",
                "description": "Instance id, from list_instances"
            }),
            Self::Model => json!({
                "type": "string",
                "description": "Odoo model, like res.partner"
            }),
            Self::Method => json!({
                "type": "string",
                "description": "Public method name, never starting with _"
            }),
            Self::Ids(min, max) => json!({
                "type": "array",
                "items": {"type": "integer", "minimum": 1},
                "minItems": min,
                "maxItems": max
            }),
            Self::Fields => json!({
                "type": "array",
                "items": {"type": "string"},
                "maxItems": ODOO_MAX_FIELDS,
                "description": "Field names; omit or leave empty for all fields"
            }),
            Self::Domain => json!({
                "type": "array",
                "items": {"type": ["string", "array"]},
                "description": "Odoo domain like [[\"name\", \"ilike\", \"acme\"]]; \"&\", \"|\", \"!\" combine terms"
            }),
            Self::Limit(default, max) => json!({
                "type": "integer",
                "minimum": 1,
                "maximum": max,
                "default": default
            }),
            Self::Offset => json!({"type": "integer", "minimum": 0, "default": 0}),
            Self::Order => json!({
                "type": "string",
                "description": "Like name asc, id desc"
            }),
            Self::Values => json!({
                "type": "object",
                "description": "Field values by field name"
            }),
            Self::Object(help) => json!({"type": "object", "description": help}),
            Self::Args => json!({
                "type": "array",
                "items": {"type": ["string", "number", "boolean", "object", "array", "null"]},
                "description": "Positional arguments; refused by Odoo 19+ instances"
            }),
            Self::AttachmentId => json!({"type": "integer", "minimum": 1}),
            Self::Text(min, max) => json!({
                "type": "string",
                "minLength": min,
                "maxLength": max
            }),
            Self::Tags => json!({
                "type": "string",
                "maxLength": TEXT_MAX_CHARS,
                "description": "Comma-separated tag names"
            }),
            Self::N8nId => json!({"type": "string"}),
            Self::Status => json!({
                "type": "string",
                "enum": STATUSES.map(ExecutionStatus::as_str)
            }),
            Self::Flag => json!({"type": "boolean", "default": false}),
            Self::Bool => json!({"type": "boolean"}),
            Self::Nodes => json!({
                "type": "array",
                "minItems": 1,
                "items": {"type": "object"},
                "description": "The n8n nodes, each an object"
            }),
            Self::TagIds => json!({
                "type": "array",
                "maxItems": TAG_IDS_MAX,
                "items": {"type": "string"},
                "description": "Every tag id the workflow must carry; empty removes all its tags"
            }),
        }
    }
}

struct Param {
    name: &'static str,
    required: bool,
    schema: Schema,
}

const fn required(name: &'static str, schema: Schema) -> Param {
    Param {
        name,
        required: true,
        schema,
    }
}

const fn optional(name: &'static str, schema: Schema) -> Param {
    Param {
        name,
        required: false,
        schema,
    }
}

struct ToolDef {
    name: &'static str,
    description: &'static str,
    params: &'static [Param],
}

impl ToolDef {
    fn accepts(&self, key: &str) -> bool {
        key == "instance" || self.params.iter().any(|param| param.name == key)
    }

    fn spec(&self) -> ToolSpec {
        let mut properties = Map::new();
        properties.insert("instance".to_owned(), Schema::Instance.json());
        for param in self.params {
            properties.insert(param.name.to_owned(), param.schema.json());
        }
        let required: Vec<&str> = std::iter::once("instance")
            .chain(
                self.params
                    .iter()
                    .filter(|param| param.required)
                    .map(|param| param.name),
            )
            .collect();
        ToolSpec {
            name: self.name.to_owned(),
            description: self.description.to_owned(),
            parameters: json!({
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            }),
        }
    }
}

const ODOO_LIMIT: Schema = Schema::Limit(ODOO_DEFAULT_LIMIT, ODOO_MAX_LIMIT);
const N8N_LIMIT: Schema = Schema::Limit(N8N_DEFAULT_LIMIT, N8N_MAX_LIMIT);
const CURSOR: Schema = Schema::Text(0, TEXT_MAX_CHARS);

static TOOLS: [ToolDef; 26] = [
    ToolDef {
        name: "odoo_list_models",
        description: "Lists the Odoo models, optionally filtered by name. Read-only.",
        params: &[
            optional("query", Schema::Text(0, TEXT_MAX_CHARS)),
            optional("limit", ODOO_LIMIT),
        ],
    },
    ToolDef {
        name: "odoo_describe_model",
        description: "Describes the fields of an Odoo model: type, required, readonly, relation, selection. Read-only.",
        params: &[required("model", Schema::Model)],
    },
    ToolDef {
        name: "odoo_search_read",
        description: "Searches the records of an Odoo model and returns the requested fields. Read-only.",
        params: &[
            required("model", Schema::Model),
            optional("domain", Schema::Domain),
            optional("fields", Schema::Fields),
            optional("limit", ODOO_LIMIT),
            optional("offset", Schema::Offset),
            optional("order", Schema::Order),
        ],
    },
    ToolDef {
        name: "odoo_read",
        description: "Reads Odoo records by id. Read-only.",
        params: &[
            required("model", Schema::Model),
            required("ids", Schema::Ids(1, ODOO_MAX_IDS)),
            optional("fields", Schema::Fields),
        ],
    },
    ToolDef {
        name: "odoo_count",
        description: "Counts the records of an Odoo model that match a domain. Read-only.",
        params: &[
            required("model", Schema::Model),
            optional("domain", Schema::Domain),
        ],
    },
    ToolDef {
        name: "odoo_read_attachment",
        description: "Reads an Odoo attachment: its text for text, JSON, XML, CSV and PDF text layers, its metadata only otherwise. Read-only.",
        params: &[required("attachment_id", Schema::AttachmentId)],
    },
    ToolDef {
        name: "odoo_create",
        description: concat!("Creates an Odoo record.", write_note!()),
        params: &[
            required("model", Schema::Model),
            required("values", Schema::Values),
        ],
    },
    ToolDef {
        name: "odoo_write",
        description: concat!("Updates Odoo records.", write_note!()),
        params: &[
            required("model", Schema::Model),
            required("ids", Schema::Ids(1, ODOO_MAX_IDS)),
            required("values", Schema::Values),
        ],
    },
    ToolDef {
        name: "odoo_delete",
        description: concat!("Deletes Odoo records, at most 50 at once.", write_note!()),
        params: &[
            required("model", Schema::Model),
            required("ids", Schema::Ids(1, ODOO_MAX_DELETE_IDS)),
        ],
    },
    ToolDef {
        name: "odoo_call_method",
        description: concat!(
            "Runs a public model method; counts as a write.",
            write_note!()
        ),
        params: &[
            required("model", Schema::Model),
            required("method", Schema::Method),
            optional("ids", Schema::Ids(0, ODOO_MAX_IDS)),
            optional("args", Schema::Args),
            optional("kwargs", Schema::Object("Keyword arguments")),
        ],
    },
    ToolDef {
        name: "n8n_list_workflows",
        description: "Lists the n8n workflows, with optional filters. Read-only.",
        params: &[
            optional("name", Schema::Text(0, TEXT_MAX_CHARS)),
            optional("tags", Schema::Tags),
            optional("active", Schema::Bool),
            optional("project_id", Schema::N8nId),
            optional("limit", N8N_LIMIT),
            optional("cursor", CURSOR),
        ],
    },
    ToolDef {
        name: "n8n_get_workflow",
        description: "Gets one n8n workflow with its nodes, connections and settings. Read-only.",
        params: &[required("workflow_id", Schema::N8nId)],
    },
    ToolDef {
        name: "n8n_list_workflow_versions",
        description: "Lists the saved versions of an n8n workflow (n8n 2.x only). Read-only.",
        params: &[required("workflow_id", Schema::N8nId)],
    },
    ToolDef {
        name: "n8n_list_executions",
        description: "Lists the n8n executions, optionally of one workflow or one status. Read-only.",
        params: &[
            optional("workflow_id", Schema::N8nId),
            optional("status", Schema::Status),
            optional("limit", N8N_LIMIT),
            optional("cursor", CURSOR),
        ],
    },
    ToolDef {
        name: "n8n_get_execution",
        description: "Gets one n8n execution; its data only with include_data. Read-only.",
        params: &[
            required("execution_id", Schema::N8nId),
            optional("include_data", Schema::Flag),
        ],
    },
    ToolDef {
        name: "n8n_list_tags",
        description: "Lists the n8n tags. Read-only.",
        params: &[optional("limit", N8N_LIMIT), optional("cursor", CURSOR)],
    },
    ToolDef {
        name: "n8n_list_credentials",
        description: "Lists the n8n credentials: names and types only, never secrets. Read-only.",
        params: &[optional("limit", N8N_LIMIT), optional("cursor", CURSOR)],
    },
    ToolDef {
        name: "n8n_create_workflow",
        description: concat!("Creates an unpublished n8n workflow.", write_note!()),
        params: &[
            required("name", Schema::Text(1, WORKFLOW_NAME_MAX_CHARS)),
            required("nodes", Schema::Nodes),
            optional("connections", Schema::Object("Connections by node name")),
            optional("settings", Schema::Object("Workflow settings")),
        ],
    },
    ToolDef {
        name: "n8n_update_workflow",
        description: concat!(
            "Replaces the nodes and connections of an n8n workflow: send them in full. Values shown as \"[masqué]\" are put back from the current workflow. With publish false a published workflow keeps its published version live on n8n 2.x.",
            write_note!()
        ),
        params: &[
            required("workflow_id", Schema::N8nId),
            optional("name", Schema::Text(1, WORKFLOW_NAME_MAX_CHARS)),
            required("nodes", Schema::Nodes),
            required("connections", Schema::Object("Connections by node name")),
            optional("settings", Schema::Object("Workflow settings")),
            optional("publish", Schema::Flag),
        ],
    },
    ToolDef {
        name: "n8n_publish_workflow",
        description: concat!("Publishes an n8n workflow.", write_note!()),
        params: &[required("workflow_id", Schema::N8nId)],
    },
    ToolDef {
        name: "n8n_unpublish_workflow",
        description: concat!("Unpublishes an n8n workflow.", write_note!()),
        params: &[required("workflow_id", Schema::N8nId)],
    },
    ToolDef {
        name: "n8n_archive_workflow",
        description: concat!("Archives an n8n workflow (n8n 2.x only).", write_note!()),
        params: &[required("workflow_id", Schema::N8nId)],
    },
    ToolDef {
        name: "n8n_set_workflow_tags",
        description: concat!("Replaces the tags of an n8n workflow.", write_note!()),
        params: &[
            required("workflow_id", Schema::N8nId),
            required("tag_ids", Schema::TagIds),
        ],
    },
    ToolDef {
        name: "n8n_create_tag",
        description: concat!("Creates an n8n tag.", write_note!()),
        params: &[required("name", Schema::Text(1, TAG_NAME_MAX_CHARS))],
    },
    ToolDef {
        name: "n8n_retry_execution",
        description: concat!("Retries an n8n execution.", write_note!()),
        params: &[required("execution_id", Schema::N8nId)],
    },
    ToolDef {
        name: "n8n_stop_execution",
        description: concat!("Stops a running n8n execution.", write_note!()),
        params: &[required("execution_id", Schema::N8nId)],
    },
];

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

fn must_be(key: &str, what: &str) -> CoreError {
    invalid(format!("{key} must be {what}"))
}

fn prefixed(key: &str, error: CoreError) -> CoreError {
    match error {
        CoreError::Invalid(message) => invalid(format!("{key}: {message}")),
        other => other,
    }
}

fn is_field_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_field_path(text: &str) -> bool {
    text.split('.').all(is_field_name)
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_order(text: &str) -> bool {
    text.split(',').enumerate().all(|(index, term)| {
        let term = if index == 0 {
            term
        } else {
            term.strip_prefix(' ').unwrap_or(term)
        };
        let mut parts = term.split(' ');
        let name = parts.next().unwrap_or_default();
        let direction = parts.next();
        parts.next().is_none()
            && is_identifier(name)
            && direction.is_none_or(|direction| {
                direction.eq_ignore_ascii_case("asc") || direction.eq_ignore_ascii_case("desc")
            })
    })
}

fn check_term(index: usize, term: &Value) -> Result<(), CoreError> {
    let bad = |why: &str| invalid(format!("domain[{index}] {why}"));
    match term {
        Value::String(operator) if LOGICAL_OPERATORS.contains(&operator.as_str()) => Ok(()),
        Value::Array(leaf) => match leaf.as_slice() {
            [Value::String(path), Value::String(operator), _] => {
                if !is_field_path(path) {
                    Err(bad("has an invalid field name"))
                } else if !COMPARISON_OPERATORS.contains(&operator.as_str()) {
                    Err(bad("has an unknown operator"))
                } else {
                    Ok(())
                }
            }
            _ => Err(bad("must be [field, operator, value]")),
        },
        _ => Err(bad(
            "must be \"&\", \"|\", \"!\" or a [field, operator, value] term",
        )),
    }
}

fn n8n_id_of(key: &str, value: &Value) -> Result<N8nId, CoreError> {
    let number;
    let raw = match value {
        Value::String(text) => text.as_str(),
        Value::Number(count) => {
            let id = count.as_u64().ok_or_else(|| must_be(key, "an id string"))?;
            number = id.to_string();
            number.as_str()
        }
        _ => return Err(must_be(key, "an id string")),
    };
    N8nId::try_from(raw).map_err(|error| prefixed(key, error))
}

struct Args<'a> {
    map: &'a Map<String, Value>,
}

impl<'a> Args<'a> {
    fn value(&self, key: &str) -> Option<&'a Value> {
        self.map.get(key).filter(|value| !value.is_null())
    }

    fn required(&self, key: &str) -> Result<&'a Value, CoreError> {
        self.value(key)
            .ok_or_else(|| invalid(format!("{key} is required")))
    }

    fn string(&self, key: &str) -> Result<&'a str, CoreError> {
        self.required(key)?
            .as_str()
            .ok_or_else(|| must_be(key, "a string"))
    }

    fn optional_string(&self, key: &str) -> Result<Option<&'a str>, CoreError> {
        self.value(key)
            .map(|value| value.as_str().ok_or_else(|| must_be(key, "a string")))
            .transpose()
    }

    fn text(&self, key: &str) -> Result<Option<String>, CoreError> {
        match self.optional_string(key)? {
            Some(text) if text.chars().count() > TEXT_MAX_CHARS => Err(invalid(format!(
                "{key} exceeds {TEXT_MAX_CHARS} characters"
            ))),
            Some(text) if !text.is_empty() => Ok(Some(text.to_owned())),
            _ => Ok(None),
        }
    }

    fn label(&self, key: &str, max: usize) -> Result<String, CoreError> {
        let text = self.string(key)?;
        if text.trim().is_empty() {
            return Err(invalid(format!("{key} must not be empty")));
        }
        if text.chars().count() > max {
            return Err(invalid(format!("{key} exceeds {max} characters")));
        }
        Ok(text.to_owned())
    }

    fn optional_label(&self, key: &str, max: usize) -> Result<Option<String>, CoreError> {
        self.value(key).map(|_| self.label(key, max)).transpose()
    }

    fn flag(&self, key: &str) -> Result<Option<bool>, CoreError> {
        self.value(key)
            .map(|value| value.as_bool().ok_or_else(|| must_be(key, "true or false")))
            .transpose()
    }

    fn array(&self, key: &str) -> Result<Option<&'a Vec<Value>>, CoreError> {
        self.value(key)
            .map(|value| value.as_array().ok_or_else(|| must_be(key, "an array")))
            .transpose()
    }

    fn object(&self, key: &str) -> Result<Option<&'a Map<String, Value>>, CoreError> {
        self.value(key)
            .map(|value| value.as_object().ok_or_else(|| must_be(key, "an object")))
            .transpose()
    }

    fn list(&self, key: &str) -> Result<Vec<Value>, CoreError> {
        Ok(self
            .array(key)?
            .map(|items| items.to_vec())
            .unwrap_or_default())
    }

    fn map(&self, key: &str) -> Result<Map<String, Value>, CoreError> {
        Ok(self.object(key)?.cloned().unwrap_or_default())
    }

    fn required_map(&self, key: &str) -> Result<Map<String, Value>, CoreError> {
        self.required(key)?
            .as_object()
            .cloned()
            .ok_or_else(|| must_be(key, "an object"))
    }

    fn positive(&self, key: &str) -> Result<i64, CoreError> {
        self.required(key)?
            .as_i64()
            .filter(|id| *id > 0)
            .ok_or_else(|| must_be(key, "a positive integer"))
    }

    fn model(&self) -> Result<OdooModel, CoreError> {
        OdooModel::try_from(self.string("model")?).map_err(|error| prefixed("model", error))
    }

    fn method(&self) -> Result<OdooMethod, CoreError> {
        OdooMethod::try_from(self.string("method")?).map_err(|error| prefixed("method", error))
    }

    fn ids(&self, non_empty: bool, max: usize) -> Result<Vec<i64>, CoreError> {
        let items = if non_empty {
            self.required("ids")?
                .as_array()
                .ok_or_else(|| must_be("ids", "an array of positive integers"))?
        } else {
            match self.array("ids")? {
                Some(items) => items,
                None => return Ok(Vec::new()),
            }
        };
        if non_empty && items.is_empty() {
            return Err(invalid("ids must not be empty"));
        }
        if items.len() > max {
            return Err(invalid(format!("ids holds at most {max} ids")));
        }
        let mut ids = Vec::with_capacity(items.len());
        for item in items {
            let id = item
                .as_i64()
                .filter(|id| *id > 0)
                .ok_or_else(|| must_be("ids", "an array of positive integers"))?;
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        Ok(ids)
    }

    fn fields(&self) -> Result<Vec<String>, CoreError> {
        let Some(items) = self.array("fields")? else {
            return Ok(Vec::new());
        };
        if items.len() > ODOO_MAX_FIELDS {
            return Err(invalid(format!(
                "fields holds at most {ODOO_MAX_FIELDS} names"
            )));
        }
        items
            .iter()
            .map(|item| {
                item.as_str()
                    .filter(|name| is_field_name(name))
                    .map(str::to_owned)
                    .ok_or_else(|| must_be("fields", "an array of field names like partner_id"))
            })
            .collect()
    }

    fn domain(&self) -> Result<Vec<Value>, CoreError> {
        let Some(terms) = self.array("domain")? else {
            return Ok(Vec::new());
        };
        for (index, term) in terms.iter().enumerate() {
            check_term(index, term)?;
        }
        Ok(terms.to_vec())
    }

    fn limit(&self, default: u32, max: u32) -> Result<u32, CoreError> {
        let Some(value) = self.value("limit") else {
            return Ok(default);
        };
        value
            .as_u64()
            .and_then(|limit| u32::try_from(limit).ok())
            .filter(|limit| (1..=max).contains(limit))
            .ok_or_else(|| invalid(format!("limit must be an integer between 1 and {max}")))
    }

    fn offset(&self) -> Result<u32, CoreError> {
        let Some(value) = self.value("offset") else {
            return Ok(0);
        };
        value
            .as_u64()
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or_else(|| must_be("offset", "a non-negative integer"))
    }

    fn order(&self) -> Result<Option<String>, CoreError> {
        match self.optional_string("order")? {
            Some("") | None => Ok(None),
            Some(order) if is_order(order) => Ok(Some(order.to_owned())),
            Some(_) => Err(invalid(
                "order must be field names with an optional asc or desc, like name asc, id desc",
            )),
        }
    }

    fn values(&self, non_empty: bool) -> Result<Map<String, Value>, CoreError> {
        let values = self.required_map("values")?;
        if non_empty && values.is_empty() {
            return Err(invalid("values must not be empty"));
        }
        match values.keys().find(|key| !is_field_name(key)) {
            Some(key) => Err(invalid(format!(
                "values has the invalid field name {key:?}"
            ))),
            None => Ok(values),
        }
    }

    fn n8n_id(&self, key: &str) -> Result<N8nId, CoreError> {
        n8n_id_of(key, self.required(key)?)
    }

    fn optional_n8n_id(&self, key: &str) -> Result<Option<N8nId>, CoreError> {
        self.value(key)
            .map(|value| n8n_id_of(key, value))
            .transpose()
    }

    fn tag_ids(&self) -> Result<Vec<N8nId>, CoreError> {
        let items = self
            .required("tag_ids")?
            .as_array()
            .ok_or_else(|| must_be("tag_ids", "an array of tag ids"))?;
        if items.len() > TAG_IDS_MAX {
            return Err(invalid(format!("tag_ids holds at most {TAG_IDS_MAX} ids")));
        }
        let mut ids: Vec<N8nId> = Vec::with_capacity(items.len());
        for item in items {
            let id = n8n_id_of("tag_ids", item)?;
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        Ok(ids)
    }

    fn nodes(&self) -> Result<Vec<Value>, CoreError> {
        let items = self
            .required("nodes")?
            .as_array()
            .filter(|items| !items.is_empty() && items.iter().all(Value::is_object))
            .ok_or_else(|| must_be("nodes", "a non-empty array of node objects"))?;
        Ok(items.to_vec())
    }

    fn status(&self) -> Result<Option<ExecutionStatus>, CoreError> {
        let Some(raw) = self.optional_string("status")? else {
            return Ok(None);
        };
        STATUSES
            .into_iter()
            .find(|status| status.as_str() == raw)
            .map(Some)
            .ok_or_else(|| {
                invalid(format!(
                    "status must be one of {}",
                    STATUSES.map(ExecutionStatus::as_str).join(", ")
                ))
            })
    }
}

pub(super) fn call(tool: &str, arguments: &Value) -> Result<ActionCall, CoreError> {
    let definition = TOOLS
        .iter()
        .find(|definition| definition.name == tool)
        .ok_or_else(|| invalid(format!("unknown tool {tool}")))?;
    let map = arguments
        .as_object()
        .ok_or_else(|| invalid(format!("the arguments of {tool} must be a JSON object")))?;
    if let Some(key) = map.keys().find(|key| !definition.accepts(key)) {
        return Err(invalid(format!("unknown argument {key} for tool {tool}")));
    }
    let args = Args { map };
    let instance = Uuid::parse_str(args.string("instance")?)
        .map(IntegrationInstanceId)
        .map_err(|_| invalid("instance must be the UUID of an instance"))?;
    Ok(ActionCall {
        instance,
        request: request(definition.name, &args)?,
    })
}

fn request(tool: &str, a: &Args) -> Result<ActionRequest, CoreError> {
    use ActionRequest as R;
    Ok(match tool {
        "odoo_list_models" => R::OdooListModels {
            query: a.text("query")?,
            limit: a.limit(ODOO_DEFAULT_LIMIT, ODOO_MAX_LIMIT)?,
        },
        "odoo_describe_model" => R::OdooDescribeModel { model: a.model()? },
        "odoo_search_read" => R::OdooSearchRead {
            model: a.model()?,
            domain: a.domain()?,
            fields: a.fields()?,
            limit: a.limit(ODOO_DEFAULT_LIMIT, ODOO_MAX_LIMIT)?,
            offset: a.offset()?,
            order: a.order()?,
        },
        "odoo_read" => R::OdooRead {
            model: a.model()?,
            ids: a.ids(true, ODOO_MAX_IDS)?,
            fields: a.fields()?,
        },
        "odoo_count" => R::OdooCount {
            model: a.model()?,
            domain: a.domain()?,
        },
        "odoo_read_attachment" => R::OdooReadAttachment {
            attachment_id: a.positive("attachment_id")?,
        },
        "odoo_create" => R::OdooCreate {
            model: a.model()?,
            values: a.values(false)?,
        },
        "odoo_write" => R::OdooWrite {
            model: a.model()?,
            ids: a.ids(true, ODOO_MAX_IDS)?,
            values: a.values(true)?,
        },
        "odoo_delete" => R::OdooDelete {
            model: a.model()?,
            ids: a.ids(true, ODOO_MAX_DELETE_IDS)?,
        },
        "odoo_call_method" => R::OdooCallMethod {
            model: a.model()?,
            method: a.method()?,
            ids: a.ids(false, ODOO_MAX_IDS)?,
            args: a.list("args")?,
            kwargs: a.map("kwargs")?,
        },
        "n8n_list_workflows" => R::N8nListWorkflows {
            name: a.text("name")?,
            tags: a.text("tags")?,
            active: a.flag("active")?,
            project_id: a.optional_n8n_id("project_id")?,
            limit: a.limit(N8N_DEFAULT_LIMIT, N8N_MAX_LIMIT)?,
            cursor: a.text("cursor")?,
        },
        "n8n_get_workflow" => R::N8nGetWorkflow {
            workflow_id: a.n8n_id("workflow_id")?,
        },
        "n8n_list_workflow_versions" => R::N8nListWorkflowVersions {
            workflow_id: a.n8n_id("workflow_id")?,
        },
        "n8n_list_executions" => R::N8nListExecutions {
            workflow_id: a.optional_n8n_id("workflow_id")?,
            status: a.status()?,
            limit: a.limit(N8N_DEFAULT_LIMIT, N8N_MAX_LIMIT)?,
            cursor: a.text("cursor")?,
        },
        "n8n_get_execution" => R::N8nGetExecution {
            execution_id: a.n8n_id("execution_id")?,
            include_data: a.flag("include_data")?.unwrap_or(false),
        },
        "n8n_list_tags" => R::N8nListTags {
            limit: a.limit(N8N_DEFAULT_LIMIT, N8N_MAX_LIMIT)?,
            cursor: a.text("cursor")?,
        },
        "n8n_list_credentials" => R::N8nListCredentials {
            limit: a.limit(N8N_DEFAULT_LIMIT, N8N_MAX_LIMIT)?,
            cursor: a.text("cursor")?,
        },
        "n8n_create_workflow" => R::N8nCreateWorkflow {
            name: a.label("name", WORKFLOW_NAME_MAX_CHARS)?,
            nodes: a.nodes()?,
            connections: a.map("connections")?,
            settings: a.map("settings")?,
        },
        "n8n_update_workflow" => R::N8nUpdateWorkflow {
            workflow_id: a.n8n_id("workflow_id")?,
            name: a.optional_label("name", WORKFLOW_NAME_MAX_CHARS)?,
            nodes: a.nodes()?,
            connections: a.required_map("connections")?,
            settings: a.object("settings")?.cloned(),
            publish: a.flag("publish")?.unwrap_or(false),
        },
        "n8n_publish_workflow" => R::N8nPublishWorkflow {
            workflow_id: a.n8n_id("workflow_id")?,
        },
        "n8n_unpublish_workflow" => R::N8nUnpublishWorkflow {
            workflow_id: a.n8n_id("workflow_id")?,
        },
        "n8n_archive_workflow" => R::N8nArchiveWorkflow {
            workflow_id: a.n8n_id("workflow_id")?,
        },
        "n8n_set_workflow_tags" => R::N8nSetWorkflowTags {
            workflow_id: a.n8n_id("workflow_id")?,
            tag_ids: a.tag_ids()?,
        },
        "n8n_create_tag" => R::N8nCreateTag {
            name: a.label("name", TAG_NAME_MAX_CHARS)?,
        },
        "n8n_retry_execution" => R::N8nRetryExecution {
            execution_id: a.n8n_id("execution_id")?,
        },
        "n8n_stop_execution" => R::N8nStopExecution {
            execution_id: a.n8n_id("execution_id")?,
        },
        other => return Err(invalid(format!("unknown tool {other}"))),
    })
}

pub(super) fn tool_of(request: &ActionRequest) -> &'static str {
    use ActionRequest as R;
    match request {
        R::OdooListModels { .. } => "odoo_list_models",
        R::OdooDescribeModel { .. } => "odoo_describe_model",
        R::OdooSearchRead { .. } => "odoo_search_read",
        R::OdooRead { .. } => "odoo_read",
        R::OdooCount { .. } => "odoo_count",
        R::OdooReadAttachment { .. } => "odoo_read_attachment",
        R::OdooCreate { .. } => "odoo_create",
        R::OdooWrite { .. } => "odoo_write",
        R::OdooDelete { .. } => "odoo_delete",
        R::OdooCallMethod { .. } => "odoo_call_method",
        R::N8nListWorkflows { .. } => "n8n_list_workflows",
        R::N8nGetWorkflow { .. } => "n8n_get_workflow",
        R::N8nListWorkflowVersions { .. } => "n8n_list_workflow_versions",
        R::N8nGetWorkflowTags { .. } => "n8n_get_workflow_tags",
        R::N8nListExecutions { .. } => "n8n_list_executions",
        R::N8nGetExecution { .. } => "n8n_get_execution",
        R::N8nListTags { .. } => "n8n_list_tags",
        R::N8nListCredentials { .. } => "n8n_list_credentials",
        R::N8nCreateWorkflow { .. } => "n8n_create_workflow",
        R::N8nUpdateWorkflow { .. } => "n8n_update_workflow",
        R::N8nPublishWorkflow { .. } => "n8n_publish_workflow",
        R::N8nUnpublishWorkflow { .. } => "n8n_unpublish_workflow",
        R::N8nArchiveWorkflow { .. } => "n8n_archive_workflow",
        R::N8nUnarchiveWorkflow { .. } => "n8n_unarchive_workflow",
        R::N8nSetWorkflowTags { .. } => "n8n_set_workflow_tags",
        R::N8nCreateTag { .. } => "n8n_create_tag",
        R::N8nDeleteTag { .. } => "n8n_delete_tag",
        R::N8nRetryExecution { .. } => "n8n_retry_execution",
        R::N8nStopExecution { .. } => "n8n_stop_execution",
    }
}

struct Builder(Map<String, Value>);

impl Builder {
    fn new() -> Self {
        Self(Map::new())
    }

    fn put(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.0.insert(key.to_owned(), value.into());
        self
    }

    fn put_some<V: Into<Value>>(self, key: &str, value: Option<V>) -> Self {
        match value {
            Some(value) => self.put(key, value),
            None => self,
        }
    }

    fn finish(self) -> Value {
        Value::Object(self.0)
    }
}

fn id_strings(ids: &[N8nId]) -> Vec<&str> {
    ids.iter().map(N8nId::as_str).collect()
}

pub(super) fn target_of(request: &ActionRequest) -> Value {
    use ActionRequest as R;
    let target = Builder::new();
    match request {
        R::OdooListModels { .. } => target.put("model", "ir.model"),
        R::OdooDescribeModel { model } | R::OdooCreate { model, .. } => {
            target.put("model", model.as_str())
        }
        R::OdooSearchRead { model, domain, .. } | R::OdooCount { model, domain } => target
            .put("model", model.as_str())
            .put("domain", domain.as_slice()),
        R::OdooRead { model, ids, .. }
        | R::OdooWrite { model, ids, .. }
        | R::OdooDelete { model, ids } => target
            .put("model", model.as_str())
            .put_some("ids", (!ids.is_empty()).then_some(ids.as_slice())),
        R::OdooCallMethod {
            model, method, ids, ..
        } => target
            .put("model", model.as_str())
            .put("method", method.as_str())
            .put_some("ids", (!ids.is_empty()).then_some(ids.as_slice())),
        R::OdooReadAttachment { attachment_id } => target.put("attachment_id", *attachment_id),
        R::N8nListWorkflows {
            name,
            tags,
            active,
            project_id,
            ..
        } => target
            .put("resource", "workflows")
            .put_some("name", name.as_deref())
            .put_some("tags", tags.as_deref())
            .put_some("active", *active)
            .put_some("project_id", project_id.as_ref().map(N8nId::as_str)),
        R::N8nListExecutions {
            workflow_id,
            status,
            ..
        } => target
            .put("resource", "executions")
            .put_some("workflow_id", workflow_id.as_ref().map(N8nId::as_str))
            .put_some("status", status.map(ExecutionStatus::as_str)),
        R::N8nListTags { .. } => target.put("resource", "tags"),
        R::N8nListCredentials { .. } => target.put("resource", "credentials"),
        R::N8nGetWorkflow { workflow_id }
        | R::N8nListWorkflowVersions { workflow_id }
        | R::N8nGetWorkflowTags { workflow_id }
        | R::N8nUpdateWorkflow { workflow_id, .. }
        | R::N8nPublishWorkflow { workflow_id }
        | R::N8nUnpublishWorkflow { workflow_id }
        | R::N8nArchiveWorkflow { workflow_id }
        | R::N8nUnarchiveWorkflow { workflow_id }
        | R::N8nSetWorkflowTags { workflow_id, .. } => {
            target.put("workflow_id", workflow_id.as_str())
        }
        R::N8nGetExecution { execution_id, .. }
        | R::N8nRetryExecution { execution_id }
        | R::N8nStopExecution { execution_id } => target.put("execution_id", execution_id.as_str()),
        R::N8nCreateWorkflow { name, .. } => target.put("workflow_name", name.as_str()),
        R::N8nCreateTag { name } => target.put("tag_name", name.as_str()),
        R::N8nDeleteTag { tag_id } => target.put("tag_id", tag_id.as_str()),
    }
    .finish()
}

pub(super) fn arguments_of(request: &ActionRequest) -> Value {
    use ActionRequest as R;
    let arguments = Builder::new();
    match request {
        R::OdooListModels { query, limit } => arguments
            .put_some("query", query.as_deref())
            .put("limit", *limit),
        R::OdooDescribeModel { model } => arguments.put("model", model.as_str()),
        R::OdooSearchRead {
            model,
            domain,
            fields,
            limit,
            offset,
            order,
        } => arguments
            .put("model", model.as_str())
            .put("domain", domain.as_slice())
            .put("fields", fields.as_slice())
            .put("limit", *limit)
            .put("offset", *offset)
            .put_some("order", order.as_deref()),
        R::OdooRead { model, ids, fields } => arguments
            .put("model", model.as_str())
            .put("ids", ids.as_slice())
            .put("fields", fields.as_slice()),
        R::OdooCount { model, domain } => arguments
            .put("model", model.as_str())
            .put("domain", domain.as_slice()),
        R::OdooReadAttachment { attachment_id } => arguments.put("attachment_id", *attachment_id),
        R::OdooCreate { model, values } => arguments
            .put("model", model.as_str())
            .put("values", values.clone()),
        R::OdooWrite { model, ids, values } => arguments
            .put("model", model.as_str())
            .put("ids", ids.as_slice())
            .put("values", values.clone()),
        R::OdooDelete { model, ids } => arguments
            .put("model", model.as_str())
            .put("ids", ids.as_slice()),
        R::OdooCallMethod {
            model,
            method,
            ids,
            args,
            kwargs,
        } => arguments
            .put("model", model.as_str())
            .put("method", method.as_str())
            .put("ids", ids.as_slice())
            .put("args", args.as_slice())
            .put("kwargs", kwargs.clone()),
        R::N8nListWorkflows {
            name,
            tags,
            active,
            project_id,
            limit,
            cursor,
        } => arguments
            .put_some("name", name.as_deref())
            .put_some("tags", tags.as_deref())
            .put_some("active", *active)
            .put_some("project_id", project_id.as_ref().map(N8nId::as_str))
            .put("limit", *limit)
            .put_some("cursor", cursor.as_deref()),
        R::N8nGetWorkflow { workflow_id }
        | R::N8nListWorkflowVersions { workflow_id }
        | R::N8nGetWorkflowTags { workflow_id }
        | R::N8nPublishWorkflow { workflow_id }
        | R::N8nUnpublishWorkflow { workflow_id }
        | R::N8nArchiveWorkflow { workflow_id }
        | R::N8nUnarchiveWorkflow { workflow_id } => {
            arguments.put("workflow_id", workflow_id.as_str())
        }
        R::N8nListExecutions {
            workflow_id,
            status,
            limit,
            cursor,
        } => arguments
            .put_some("workflow_id", workflow_id.as_ref().map(N8nId::as_str))
            .put_some("status", status.map(ExecutionStatus::as_str))
            .put("limit", *limit)
            .put_some("cursor", cursor.as_deref()),
        R::N8nGetExecution {
            execution_id,
            include_data,
        } => arguments
            .put("execution_id", execution_id.as_str())
            .put("include_data", *include_data),
        R::N8nListTags { limit, cursor } | R::N8nListCredentials { limit, cursor } => arguments
            .put("limit", *limit)
            .put_some("cursor", cursor.as_deref()),
        R::N8nCreateWorkflow {
            name,
            nodes,
            connections,
            settings,
        } => arguments
            .put("name", name.as_str())
            .put("nodes", nodes.as_slice())
            .put("connections", connections.clone())
            .put("settings", settings.clone()),
        R::N8nUpdateWorkflow {
            workflow_id,
            name,
            nodes,
            connections,
            settings,
            publish,
        } => arguments
            .put("workflow_id", workflow_id.as_str())
            .put_some("name", name.as_deref())
            .put("nodes", nodes.as_slice())
            .put("connections", connections.clone())
            .put_some("settings", settings.clone())
            .put("publish", *publish),
        R::N8nSetWorkflowTags {
            workflow_id,
            tag_ids,
        } => arguments
            .put("workflow_id", workflow_id.as_str())
            .put("tag_ids", id_strings(tag_ids)),
        R::N8nCreateTag { name } => arguments.put("name", name.as_str()),
        R::N8nDeleteTag { tag_id } => arguments.put("tag_id", tag_id.as_str()),
        R::N8nRetryExecution { execution_id } | R::N8nStopExecution { execution_id } => {
            arguments.put("execution_id", execution_id.as_str())
        }
    }
    .finish()
}

fn read_of(model: &OdooModel, ids: &[i64], fields: Vec<String>) -> ActionRequest {
    ActionRequest::OdooRead {
        model: model.clone(),
        ids: ids.to_vec(),
        fields,
    }
}

fn workflow_of(workflow_id: &N8nId) -> ActionRequest {
    ActionRequest::N8nGetWorkflow {
        workflow_id: workflow_id.clone(),
    }
}

fn tags_of(workflow_id: &N8nId) -> ActionRequest {
    ActionRequest::N8nGetWorkflowTags {
        workflow_id: workflow_id.clone(),
    }
}

fn keys_of(values: &Map<String, Value>) -> Vec<String> {
    values.keys().cloned().collect()
}

pub(super) fn before_read(request: &ActionRequest) -> Option<ActionRequest> {
    use ActionRequest as R;
    match request {
        R::OdooWrite { model, ids, values } => Some(read_of(model, ids, keys_of(values))),
        R::OdooDelete { model, ids } => Some(read_of(model, ids, Vec::new())),
        R::N8nUpdateWorkflow { workflow_id, .. }
        | R::N8nPublishWorkflow { workflow_id }
        | R::N8nUnpublishWorkflow { workflow_id }
        | R::N8nArchiveWorkflow { workflow_id }
        | R::N8nUnarchiveWorkflow { workflow_id } => Some(workflow_of(workflow_id)),
        R::N8nSetWorkflowTags { workflow_id, .. } => Some(tags_of(workflow_id)),
        _ => None,
    }
}

fn created_ids(result: &Value) -> Option<Vec<i64>> {
    let positive = |value: &Value| value.as_i64().filter(|id| *id > 0);
    let ids: Vec<i64> = match result {
        Value::Array(items) => items.iter().map(positive).collect::<Option<_>>()?,
        single => vec![positive(single)?],
    };
    (!ids.is_empty()).then_some(ids)
}

pub(super) fn after_read(request: &ActionRequest, result: &Value) -> Option<ActionRequest> {
    use ActionRequest as R;
    match request {
        R::OdooWrite { .. } => before_read(request),
        R::OdooCreate { model, values } => {
            created_ids(result).map(|ids| read_of(model, &ids, keys_of(values)))
        }
        R::N8nCreateWorkflow { .. } => result
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| N8nId::try_from(id).ok())
            .map(|workflow_id| workflow_of(&workflow_id)),
        R::N8nUpdateWorkflow { workflow_id, .. }
        | R::N8nPublishWorkflow { workflow_id }
        | R::N8nUnpublishWorkflow { workflow_id }
        | R::N8nUnarchiveWorkflow { workflow_id } => Some(workflow_of(workflow_id)),
        R::N8nSetWorkflowTags { workflow_id, .. } => Some(tags_of(workflow_id)),
        _ => None,
    }
}

pub(super) fn tool_specs() -> Vec<ToolSpec> {
    TOOLS.iter().map(ToolDef::spec).collect()
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{CoreError, IntegrationInstanceId};
    use serde_json::{Value, json};
    use uuid::Uuid;

    use super::super::action::{
        Access, ActionCall, ActionRequest as R, ExecutionStatus, N8nId, OdooModel,
    };
    use super::*;

    const INSTANCE: &str = "018f3b7a-0000-7000-8000-000000000001";

    const PUBLIC: [&str; 26] = [
        "odoo_list_models",
        "odoo_describe_model",
        "odoo_search_read",
        "odoo_read",
        "odoo_count",
        "odoo_read_attachment",
        "odoo_create",
        "odoo_write",
        "odoo_delete",
        "odoo_call_method",
        "n8n_list_workflows",
        "n8n_get_workflow",
        "n8n_list_workflow_versions",
        "n8n_list_executions",
        "n8n_get_execution",
        "n8n_list_tags",
        "n8n_list_credentials",
        "n8n_create_workflow",
        "n8n_update_workflow",
        "n8n_publish_workflow",
        "n8n_unpublish_workflow",
        "n8n_archive_workflow",
        "n8n_set_workflow_tags",
        "n8n_create_tag",
        "n8n_retry_execution",
        "n8n_stop_execution",
    ];

    const WRITES: [&str; 13] = [
        "odoo_create",
        "odoo_write",
        "odoo_delete",
        "odoo_call_method",
        "n8n_create_workflow",
        "n8n_update_workflow",
        "n8n_publish_workflow",
        "n8n_unpublish_workflow",
        "n8n_archive_workflow",
        "n8n_set_workflow_tags",
        "n8n_create_tag",
        "n8n_retry_execution",
        "n8n_stop_execution",
    ];

    fn minimal() -> Vec<(&'static str, Value)> {
        let partner = json!({"model": "res.partner"});
        let workflow = json!({"workflow_id": "wf1"});
        let execution = json!({"execution_id": "42"});
        vec![
            ("odoo_list_models", json!({})),
            ("odoo_describe_model", partner.clone()),
            ("odoo_search_read", partner.clone()),
            ("odoo_read", json!({"model": "res.partner", "ids": [1]})),
            ("odoo_count", partner),
            ("odoo_read_attachment", json!({"attachment_id": 7})),
            (
                "odoo_create",
                json!({"model": "res.partner", "values": {"name": "Acme"}}),
            ),
            (
                "odoo_write",
                json!({"model": "res.partner", "ids": [1], "values": {"name": "Acme"}}),
            ),
            ("odoo_delete", json!({"model": "res.partner", "ids": [1]})),
            (
                "odoo_call_method",
                json!({"model": "sale.order", "method": "action_confirm"}),
            ),
            ("n8n_list_workflows", json!({})),
            ("n8n_get_workflow", workflow.clone()),
            ("n8n_list_workflow_versions", workflow.clone()),
            ("n8n_list_executions", json!({})),
            ("n8n_get_execution", execution.clone()),
            ("n8n_list_tags", json!({})),
            ("n8n_list_credentials", json!({})),
            (
                "n8n_create_workflow",
                json!({"name": "Relance", "nodes": [{"name": "Start"}]}),
            ),
            (
                "n8n_update_workflow",
                json!({"workflow_id": "wf1", "nodes": [{"name": "Start"}], "connections": {}}),
            ),
            ("n8n_publish_workflow", workflow.clone()),
            ("n8n_unpublish_workflow", workflow.clone()),
            ("n8n_archive_workflow", workflow),
            (
                "n8n_set_workflow_tags",
                json!({"workflow_id": "wf1", "tag_ids": []}),
            ),
            ("n8n_create_tag", json!({"name": "urgent"})),
            ("n8n_retry_execution", execution.clone()),
            ("n8n_stop_execution", execution),
        ]
    }

    fn with_instance(mut arguments: Value) -> Value {
        arguments["instance"] = json!(INSTANCE);
        arguments
    }

    fn parse(tool: &str, arguments: Value) -> R {
        call(tool, &with_instance(arguments))
            .unwrap_or_else(|error| panic!("{tool} should parse: {error}"))
            .request
    }

    fn refusal_of(tool: &str, arguments: &Value) -> String {
        match call(tool, arguments) {
            Err(CoreError::Invalid(message)) => message,
            other => panic!("{tool} should be refused, got {other:?}"),
        }
    }

    fn refusal(tool: &str, arguments: Value) -> String {
        refusal_of(tool, &with_instance(arguments))
    }

    fn model(name: &str) -> OdooModel {
        OdooModel::try_from(name).unwrap()
    }

    fn id(raw: &str) -> N8nId {
        N8nId::try_from(raw).unwrap()
    }

    fn partner_read(ids: Vec<i64>, fields: &[&str]) -> R {
        R::OdooRead {
            model: model("res.partner"),
            ids,
            fields: fields.iter().map(|field| (*field).to_owned()).collect(),
        }
    }

    #[test]
    fn the_minimal_table_lists_the_public_tools_in_order() {
        let names: Vec<&str> = minimal().iter().map(|(tool, _)| *tool).collect();
        assert_eq!(names, PUBLIC);
    }

    #[test]
    fn every_public_tool_parses_a_minimal_call_and_names_itself() {
        for (tool, arguments) in minimal() {
            let call = ActionCall::parse(tool, &with_instance(arguments)).unwrap();
            assert_eq!(
                call.instance,
                IntegrationInstanceId(Uuid::parse_str(INSTANCE).unwrap())
            );
            assert_eq!(call.request.tool(), tool);
            assert!(call.request.is_public());
        }
    }

    #[test]
    fn normalized_arguments_parse_back_to_the_same_request() {
        for (tool, arguments) in minimal() {
            let request = parse(tool, arguments);
            assert_eq!(parse(tool, request.arguments()), request, "{tool}");
        }
        let full = json!({
            "model": "res.partner",
            "domain": ["|", ["name", "ilike", "a"], ["parent_id.name", "=", "b"]],
            "fields": ["name", "email"],
            "limit": 10,
            "offset": 5,
            "order": "name desc, id"
        });
        let request = parse("odoo_search_read", full);
        assert_eq!(parse("odoo_search_read", request.arguments()), request);
    }

    #[test]
    fn the_specs_are_the_public_tools_in_order_and_ask_for_the_instance() {
        let specs = tool_specs();
        let names: Vec<&str> = specs.iter().map(|spec| spec.name.as_str()).collect();
        assert_eq!(names, PUBLIC);
        for spec in &specs {
            let parameters = &spec.parameters;
            assert_eq!(parameters["type"], "object", "{}", spec.name);
            assert_eq!(parameters["additionalProperties"], false, "{}", spec.name);
            let required: Vec<&str> = parameters["required"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .collect();
            assert!(required.contains(&"instance"), "{}", spec.name);
            for name in required {
                assert!(
                    parameters["properties"].get(name).is_some(),
                    "{}",
                    spec.name
                );
            }
            assert!(parameters["properties"]["instance"].is_object());
            assert!(!spec.description.is_empty());
        }
    }

    #[test]
    fn the_specs_say_what_reads_and_what_writes() {
        for spec in tool_specs() {
            if WRITES.contains(&spec.name.as_str()) {
                assert!(spec.description.contains("copy"), "{}", spec.name);
                assert!(spec.description.contains("writing"), "{}", spec.name);
            } else {
                assert!(spec.description.contains("Read-only"), "{}", spec.name);
            }
        }
        let by_name = |name: &str| {
            tool_specs()
                .into_iter()
                .find(|spec| spec.name == name)
                .unwrap()
        };
        assert!(
            by_name("odoo_call_method")
                .description
                .contains("counts as a write")
        );
        let update = by_name("n8n_update_workflow").description;
        assert!(update.contains("[masqué]"));
        assert!(update.contains("publish"));
        assert!(by_name("n8n_archive_workflow").description.contains("2.x"));
        let search = by_name("odoo_search_read").parameters;
        assert_eq!(search["properties"]["limit"]["maximum"], 200);
        assert_eq!(search["properties"]["limit"]["default"], 50);
        assert_eq!(
            by_name("n8n_list_executions").parameters["properties"]["limit"]["maximum"],
            100
        );
    }

    #[test]
    fn a_required_argument_left_out_is_refused_by_name() {
        for spec in tool_specs() {
            let (_, arguments) = minimal()
                .into_iter()
                .find(|(tool, _)| *tool == spec.name)
                .unwrap();
            let mut required: Vec<&str> = spec.parameters["required"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .filter(|name| *name != "instance")
                .collect();
            let mut given: Vec<&str> = arguments
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            required.sort_unstable();
            given.sort_unstable();
            assert_eq!(required, given, "{}", spec.name);
            for name in required {
                let mut cut = arguments.clone();
                cut.as_object_mut().unwrap().remove(name);
                let message = refusal(&spec.name, cut);
                assert!(message.contains(name), "{}: {message}", spec.name);
            }
        }
    }

    #[test]
    fn the_instance_is_required_and_must_be_a_uuid() {
        let message = refusal_of("odoo_count", &json!({"model": "res.partner"}));
        assert!(message.contains("instance"), "{message}");
        for bad in [json!("nope"), json!(7), json!(""), json!(["x"])] {
            let arguments = json!({"instance": bad, "model": "res.partner"});
            let message = refusal_of("odoo_count", &arguments);
            assert!(message.contains("instance"), "{message}");
        }
    }

    #[test]
    fn the_arguments_must_be_an_object() {
        for bad in [json!([]), json!("x"), Value::Null, json!(3)] {
            let message = refusal_of("odoo_count", &bad);
            assert!(message.contains("object"), "{message}");
        }
    }

    #[test]
    fn an_unknown_tool_is_refused() {
        assert_eq!(
            refusal("odoo_nope", json!({})),
            "unknown tool odoo_nope".to_owned()
        );
        assert_eq!(
            refusal("list_instances", json!({})),
            "unknown tool list_instances".to_owned()
        );
        assert_eq!(refusal("", json!({})), "unknown tool ".to_owned());
    }

    #[test]
    fn an_internal_tool_name_is_never_parsed() {
        for tool in [
            "n8n_get_workflow_tags",
            "n8n_unarchive_workflow",
            "n8n_delete_tag",
        ] {
            let arguments = json!({"workflow_id": "wf1", "tag_id": "t1"});
            assert_eq!(refusal(tool, arguments), format!("unknown tool {tool}"));
        }
    }

    #[test]
    fn the_internal_variants_still_name_themselves() {
        let workflow_id = id("wf1");
        let cases = [
            (
                R::N8nGetWorkflowTags {
                    workflow_id: workflow_id.clone(),
                },
                "n8n_get_workflow_tags",
            ),
            (
                R::N8nUnarchiveWorkflow { workflow_id },
                "n8n_unarchive_workflow",
            ),
            (R::N8nDeleteTag { tag_id: id("t1") }, "n8n_delete_tag"),
        ];
        for (request, name) in cases {
            assert_eq!(request.tool(), name);
            assert!(!request.is_public());
        }
    }

    #[test]
    fn an_unknown_argument_is_refused_with_its_name() {
        assert_eq!(
            refusal("odoo_count", json!({"model": "res.partner", "limit": 3})),
            "unknown argument limit for tool odoo_count"
        );
        assert_eq!(
            refusal("n8n_list_tags", json!({"workflow_id": "wf1"})),
            "unknown argument workflow_id for tool n8n_list_tags"
        );
    }

    #[test]
    fn a_null_optional_argument_counts_as_absent() {
        let request = parse(
            "odoo_search_read",
            json!({"model": "res.partner", "order": null, "limit": null, "fields": null}),
        );
        assert!(matches!(
            request,
            R::OdooSearchRead {
                order: None,
                limit: 50,
                ..
            }
        ));
    }

    #[test]
    fn a_model_and_a_method_are_checked_by_their_types() {
        for bad in [
            "",
            "Res.Partner",
            "res partner",
            "res..partner",
            "res.partner;",
        ] {
            let message = refusal("odoo_describe_model", json!({"model": bad}));
            assert!(message.contains("model"), "{bad}: {message}");
        }
        assert!(refusal("odoo_describe_model", json!({"model": 3})).contains("model"));
        for bad in ["_unlink", "Unlink", "", "do it", "__init__"] {
            let arguments = json!({"model": "res.partner", "method": bad});
            let message = refusal("odoo_call_method", arguments);
            assert!(message.contains("method"), "{bad}: {message}");
        }
    }

    #[test]
    fn ids_must_be_positive_integers() {
        for bad in [
            json!([0]),
            json!([-1]),
            json!([1.5]),
            json!(["1"]),
            json!([null]),
            json!([]),
            json!("1"),
            json!(1),
        ] {
            let message = refusal("odoo_read", json!({"model": "res.partner", "ids": bad}));
            assert!(message.contains("ids"), "{bad}: {message}");
        }
    }

    #[test]
    fn ids_are_capped_and_deduplicated_in_order() {
        let many: Vec<i64> = (1..=201).collect();
        let message = refusal("odoo_read", json!({"model": "res.partner", "ids": many}));
        assert!(message.contains("200"), "{message}");
        let limit: Vec<i64> = (1..=200).collect();
        let request = parse("odoo_read", json!({"model": "res.partner", "ids": limit}));
        assert!(matches!(request, R::OdooRead { ids, .. } if ids.len() == 200));
        let request = parse(
            "odoo_read",
            json!({"model": "res.partner", "ids": [3, 1, 3, 2, 1]}),
        );
        assert_eq!(request, partner_read(vec![3, 1, 2], &[]));
    }

    #[test]
    fn a_delete_takes_at_most_fifty_ids() {
        let over: Vec<i64> = (1..=51).collect();
        let message = refusal("odoo_delete", json!({"model": "res.partner", "ids": over}));
        assert!(message.contains("50"), "{message}");
        let fifty: Vec<i64> = (1..=50).collect();
        let request = parse("odoo_delete", json!({"model": "res.partner", "ids": fifty}));
        assert!(matches!(request, R::OdooDelete { ids, .. } if ids.len() == 50));
    }

    #[test]
    fn a_method_call_takes_optional_ids_args_and_kwargs() {
        let request = parse(
            "odoo_call_method",
            json!({"model": "sale.order", "method": "action_confirm"}),
        );
        assert!(matches!(
            &request,
            R::OdooCallMethod { ids, args, kwargs, .. }
                if ids.is_empty() && args.is_empty() && kwargs.is_empty()
        ));
        let request = parse(
            "odoo_call_method",
            json!({
                "model": "sale.order",
                "method": "action_confirm",
                "ids": [4, 4, 5],
                "args": [1, "x"],
                "kwargs": {"context": {"lang": "fr_FR"}}
            }),
        );
        assert!(matches!(
            &request,
            R::OdooCallMethod { ids, args, kwargs, .. }
                if ids == &[4, 5] && args.len() == 2 && kwargs.contains_key("context")
        ));
        let arguments = json!({"model": "sale.order", "method": "m", "args": {"a": 1}});
        assert!(refusal("odoo_call_method", arguments).contains("args"));
        let arguments = json!({"model": "sale.order", "method": "m", "kwargs": [1]});
        assert!(refusal("odoo_call_method", arguments).contains("kwargs"));
        let arguments = json!({"model": "sale.order", "method": "m", "ids": [0]});
        assert!(refusal("odoo_call_method", arguments).contains("ids"));
    }

    #[test]
    fn fields_are_names_and_may_be_empty() {
        let request = parse("odoo_search_read", json!({"model": "res.partner"}));
        assert!(matches!(request, R::OdooSearchRead { fields, .. } if fields.is_empty()));
        let request = parse(
            "odoo_search_read",
            json!({"model": "res.partner", "fields": []}),
        );
        assert!(matches!(request, R::OdooSearchRead { fields, .. } if fields.is_empty()));
        for bad in [
            json!(["Name"]),
            json!(["a.b"]),
            json!([1]),
            json!("name"),
            json!([""]),
        ] {
            let arguments = json!({"model": "res.partner", "fields": bad});
            assert!(refusal("odoo_search_read", arguments).contains("fields"));
        }
        let too_many: Vec<String> = (0..101).map(|n| format!("f{n}")).collect();
        let arguments = json!({"model": "res.partner", "fields": too_many});
        assert!(refusal("odoo_search_read", arguments).contains("100"));
    }

    #[test]
    fn the_limit_defaults_and_is_refused_not_clamped() {
        let request = parse("odoo_search_read", json!({"model": "res.partner"}));
        assert!(matches!(
            request,
            R::OdooSearchRead {
                limit: 50,
                offset: 0,
                ..
            }
        ));
        let request = parse(
            "odoo_search_read",
            json!({"model": "res.partner", "limit": 200}),
        );
        assert!(matches!(request, R::OdooSearchRead { limit: 200, .. }));
        for bad in [json!(0), json!(201), json!(-1), json!("5"), json!(2.5)] {
            let arguments = json!({"model": "res.partner", "limit": bad});
            let message = refusal("odoo_search_read", arguments);
            assert!(message.contains("limit"), "{bad}: {message}");
        }
        assert!(refusal("odoo_list_models", json!({"limit": 201})).contains("limit"));
        assert!(matches!(
            parse("odoo_list_models", json!({})),
            R::OdooListModels {
                limit: 50,
                query: None
            }
        ));
    }

    #[test]
    fn the_n8n_limit_defaults_to_twenty_and_stops_at_a_hundred() {
        for tool in [
            "n8n_list_workflows",
            "n8n_list_executions",
            "n8n_list_tags",
            "n8n_list_credentials",
        ] {
            assert!(refusal(tool, json!({"limit": 101})).contains("limit"));
            assert!(refusal(tool, json!({"limit": 0})).contains("limit"));
            assert!(call(tool, &with_instance(json!({"limit": 100}))).is_ok());
            assert_eq!(parse(tool, json!({})).arguments()["limit"], 20, "{tool}");
        }
    }

    #[test]
    fn the_offset_is_a_non_negative_integer() {
        let request = parse(
            "odoo_search_read",
            json!({"model": "res.partner", "offset": 40}),
        );
        assert!(matches!(request, R::OdooSearchRead { offset: 40, .. }));
        for bad in [json!(-1), json!("1"), json!(1.5), json!(5_000_000_000_u64)] {
            let arguments = json!({"model": "res.partner", "offset": bad});
            assert!(refusal("odoo_search_read", arguments).contains("offset"));
        }
    }

    #[test]
    fn a_domain_takes_terms_and_logical_operators() {
        let request = parse("odoo_count", json!({"model": "res.partner"}));
        assert!(matches!(request, R::OdooCount { domain, .. } if domain.is_empty()));
        for operator in [
            "=",
            "!=",
            ">",
            ">=",
            "<",
            "<=",
            "=?",
            "=like",
            "like",
            "not like",
            "ilike",
            "not ilike",
            "=ilike",
            "in",
            "not in",
            "child_of",
            "parent_of",
            "any",
            "not any",
        ] {
            let domain = json!([["partner_id.name", operator, "x"]]);
            let request = parse(
                "odoo_count",
                json!({"model": "res.partner", "domain": domain}),
            );
            assert!(matches!(request, R::OdooCount { domain, .. } if domain.len() == 1));
        }
        let domain = json!([
            "|",
            "!",
            ["name", "ilike", "a"],
            "&",
            ["id", "in", [1, 2]],
            ["child_ids", "any", [["active", "=", true]]]
        ]);
        let request = parse(
            "odoo_count",
            json!({"model": "res.partner", "domain": domain}),
        );
        assert!(matches!(request, R::OdooCount { domain, .. } if domain.len() == 6));
    }

    #[test]
    fn a_malformed_domain_is_refused() {
        for bad in [
            json!([["name", "~", "a"]]),
            json!([["name", "=="]]),
            json!([["name", "=", "a", "b"]]),
            json!([["Name", "=", "a"]]),
            json!([["na me", "=", "a"]]),
            json!([[1, "=", 1]]),
            json!([["name", 3, "a"]]),
            json!(["^"]),
            json!(["name = 1"]),
            json!([3]),
            json!([{"name": "a"}]),
            json!("name = 1"),
            json!({"name": "a"}),
        ] {
            let arguments = json!({"model": "res.partner", "domain": bad});
            let message = refusal("odoo_count", arguments);
            assert!(message.contains("domain"), "{bad}: {message}");
        }
    }

    #[test]
    fn an_order_is_a_comma_separated_list_of_fields_with_a_direction() {
        for good in [
            "name",
            "name asc",
            "name DESC, id",
            "name desc,id asc",
            "NAME Asc",
            "_x_1",
        ] {
            let request = parse(
                "odoo_search_read",
                json!({"model": "res.partner", "order": good}),
            );
            assert!(
                matches!(&request, R::OdooSearchRead { order: Some(order), .. } if order == good),
                "{good}"
            );
        }
        for bad in [
            "name; drop table",
            "name  asc",
            "name up",
            "1name",
            "name,",
            ", name",
            "name asc desc",
            " name",
            "name,  id",
            "name->x",
            "name\nid",
        ] {
            let arguments = json!({"model": "res.partner", "order": bad});
            let message = refusal("odoo_search_read", arguments);
            assert!(message.contains("order"), "{bad}: {message}");
        }
        assert!(
            refusal(
                "odoo_search_read",
                json!({"model": "res.partner", "order": 3})
            )
            .contains("order")
        );
    }

    #[test]
    fn values_are_an_object_of_field_values() {
        let arguments = json!({"model": "res.partner", "ids": [1], "values": {}});
        assert!(refusal("odoo_write", arguments).contains("values"));
        for bad in [json!([1]), json!("x"), json!(3)] {
            let arguments = json!({"model": "res.partner", "ids": [1], "values": bad});
            assert!(refusal("odoo_write", arguments).contains("values"));
        }
        let arguments = json!({"model": "res.partner", "ids": [1], "values": {"bad key": 1}});
        assert!(refusal("odoo_write", arguments).contains("values"));
        let request = parse("odoo_create", json!({"model": "res.partner", "values": {}}));
        assert!(matches!(request, R::OdooCreate { values, .. } if values.is_empty()));
    }

    #[test]
    fn an_attachment_id_is_a_positive_integer() {
        for bad in [json!(0), json!(-3), json!("7"), json!(1.5)] {
            let message = refusal("odoo_read_attachment", json!({"attachment_id": bad}));
            assert!(message.contains("attachment_id"), "{bad}: {message}");
        }
    }

    #[test]
    fn text_arguments_are_capped_at_two_hundred_characters() {
        let long = "é".repeat(201);
        assert!(refusal("odoo_list_models", json!({"query": long})).contains("query"));
        for key in ["name", "tags", "cursor"] {
            let message = refusal("n8n_list_workflows", json!({ key: long }));
            assert!(message.contains(key), "{message}");
        }
        assert!(refusal("n8n_list_tags", json!({"cursor": long})).contains("cursor"));
        let fits = "é".repeat(200);
        assert!(call("n8n_list_tags", &with_instance(json!({"cursor": fits}))).is_ok());
        assert!(refusal("n8n_list_tags", json!({"cursor": 3})).contains("cursor"));
        let request = parse(
            "n8n_list_workflows",
            json!({"name": "Relance", "tags": "a,b", "active": true, "project_id": "p1", "cursor": "abc"}),
        );
        assert_eq!(
            request,
            R::N8nListWorkflows {
                name: Some("Relance".into()),
                tags: Some("a,b".into()),
                active: Some(true),
                project_id: Some(id("p1")),
                limit: 20,
                cursor: Some("abc".into()),
            }
        );
    }

    #[test]
    fn n8n_ids_are_checked_and_numbers_are_accepted() {
        for bad in [
            json!("a b"),
            json!(""),
            json!("a/b"),
            json!(-4),
            json!(1.5),
            json!([1]),
        ] {
            let message = refusal("n8n_get_workflow", json!({"workflow_id": bad}));
            assert!(message.contains("workflow_id"), "{bad}: {message}");
        }
        let request = parse("n8n_get_execution", json!({"execution_id": 42}));
        assert_eq!(
            request,
            R::N8nGetExecution {
                execution_id: id("42"),
                include_data: false
            }
        );
    }

    #[test]
    fn an_execution_filter_takes_a_known_status() {
        for status in ["canceled", "error", "running", "success", "waiting"] {
            let request = parse("n8n_list_executions", json!({"status": status}));
            assert!(
                matches!(request, R::N8nListExecutions { status: Some(s), .. } if s.as_str() == status)
            );
        }
        let message = refusal("n8n_list_executions", json!({"status": "done"}));
        assert!(message.contains("status"), "{message}");
        assert!(refusal("n8n_list_executions", json!({"status": 1})).contains("status"));
    }

    #[test]
    fn flags_must_be_booleans() {
        for (tool, key) in [
            ("n8n_get_execution", "include_data"),
            ("n8n_list_workflows", "active"),
        ] {
            let mut arguments = json!({ key: "yes" });
            if tool == "n8n_get_execution" {
                arguments["execution_id"] = json!("42");
            }
            assert!(refusal(tool, arguments).contains(key));
        }
        let arguments =
            json!({"workflow_id": "wf1", "nodes": [{"a": 1}], "connections": {}, "publish": 1});
        assert!(refusal("n8n_update_workflow", arguments).contains("publish"));
    }

    #[test]
    fn a_workflow_is_created_from_a_name_nodes_and_defaults() {
        let request = parse(
            "n8n_create_workflow",
            json!({"name": "Relance", "nodes": [{"name": "Start"}]}),
        );
        assert!(matches!(
            &request,
            R::N8nCreateWorkflow { name, nodes, connections, settings }
                if name == "Relance" && nodes.len() == 1 && connections.is_empty() && settings.is_empty()
        ));
        for bad in [json!(""), json!("  "), json!("x".repeat(129)), json!(4)] {
            let arguments = json!({"name": bad, "nodes": [{"a": 1}]});
            assert!(refusal("n8n_create_workflow", arguments).contains("name"));
        }
        assert!(
            call(
                "n8n_create_workflow",
                &with_instance(json!({"name": "x".repeat(128), "nodes": [{"a": 1}]}))
            )
            .is_ok()
        );
        for bad in [json!([]), json!([1]), json!({"a": 1}), json!("x")] {
            let arguments = json!({"name": "ok", "nodes": bad});
            assert!(refusal("n8n_create_workflow", arguments).contains("nodes"));
        }
        let arguments = json!({"name": "ok", "nodes": [{"a": 1}], "connections": []});
        assert!(refusal("n8n_create_workflow", arguments).contains("connections"));
        let arguments = json!({"name": "ok", "nodes": [{"a": 1}], "settings": 3});
        assert!(refusal("n8n_create_workflow", arguments).contains("settings"));
    }

    #[test]
    fn a_workflow_update_keeps_name_and_settings_optional() {
        let request = parse(
            "n8n_update_workflow",
            json!({
                "workflow_id": "wf1",
                "name": "Renamed",
                "nodes": [{"name": "Start"}],
                "connections": {"Start": {}},
                "settings": {"timezone": "Europe/Paris"},
                "publish": true
            }),
        );
        assert!(matches!(
            &request,
            R::N8nUpdateWorkflow { name: Some(name), settings: Some(settings), publish: true, .. }
                if name == "Renamed" && settings.contains_key("timezone")
        ));
        let request = parse(
            "n8n_update_workflow",
            json!({"workflow_id": "wf1", "nodes": [{"a": 1}], "connections": {}}),
        );
        assert!(matches!(
            request,
            R::N8nUpdateWorkflow {
                name: None,
                settings: None,
                publish: false,
                ..
            }
        ));
    }

    #[test]
    fn workflow_tags_are_a_bounded_list_of_ids() {
        let request = parse(
            "n8n_set_workflow_tags",
            json!({"workflow_id": "wf1", "tag_ids": ["a", "b", "a"]}),
        );
        assert_eq!(
            request,
            R::N8nSetWorkflowTags {
                workflow_id: id("wf1"),
                tag_ids: vec![id("a"), id("b")]
            }
        );
        let many: Vec<String> = (0..51).map(|n| format!("t{n}")).collect();
        let arguments = json!({"workflow_id": "wf1", "tag_ids": many});
        assert!(refusal("n8n_set_workflow_tags", arguments).contains("50"));
        for bad in [json!(["a b"]), json!([3.5]), json!("a"), json!([null])] {
            let arguments = json!({"workflow_id": "wf1", "tag_ids": bad});
            assert!(refusal("n8n_set_workflow_tags", arguments).contains("tag_ids"));
        }
    }

    #[test]
    fn a_tag_name_is_one_to_sixty_four_characters() {
        for bad in [json!(""), json!(" "), json!("x".repeat(65)), json!(3)] {
            assert!(refusal("n8n_create_tag", json!({"name": bad})).contains("name"));
        }
        assert!(
            call(
                "n8n_create_tag",
                &with_instance(json!({"name": "x".repeat(64)}))
            )
            .is_ok()
        );
    }

    #[test]
    fn parsed_calls_carry_the_access_of_their_variant() {
        for (tool, arguments) in minimal() {
            let expected = if WRITES.contains(&tool) {
                Access::Write
            } else {
                Access::Read
            };
            assert_eq!(parse(tool, arguments).access(), expected, "{tool}");
        }
    }

    #[test]
    fn a_write_is_read_before_on_the_keys_it_changes() {
        let write = parse(
            "odoo_write",
            json!({"model": "res.partner", "ids": [1, 2], "values": {"name": "Acme"}}),
        );
        let read = partner_read(vec![1, 2], &["name"]);
        assert_eq!(write.before_read(), Some(read.clone()));
        assert_eq!(write.after_read(&json!(true)), Some(read));
        let wide = parse(
            "odoo_write",
            json!({"model": "res.partner", "ids": [1], "values": {"name": "A", "email": "b"}}),
        );
        let Some(R::OdooRead { mut fields, .. }) = wide.before_read() else {
            panic!("a write must be read before");
        };
        fields.sort();
        assert_eq!(fields, ["email", "name"]);
    }

    #[test]
    fn a_delete_is_read_in_full_before_and_not_after() {
        let delete = parse(
            "odoo_delete",
            json!({"model": "res.partner", "ids": [4, 5]}),
        );
        assert_eq!(delete.before_read(), Some(partner_read(vec![4, 5], &[])));
        assert_eq!(delete.after_read(&json!(true)), None);
    }

    #[test]
    fn a_create_is_read_after_from_the_ids_odoo_returned() {
        let create = parse(
            "odoo_create",
            json!({"model": "res.partner", "values": {"name": "Acme"}}),
        );
        assert_eq!(create.before_read(), None);
        let read = Some(partner_read(vec![9], &["name"]));
        assert_eq!(create.after_read(&json!(9)), read);
        assert_eq!(create.after_read(&json!([9])), read);
        assert_eq!(
            create.after_read(&json!([9, 10])),
            Some(partner_read(vec![9, 10], &["name"]))
        );
        for bad in [
            json!(null),
            json!(0),
            json!(-2),
            json!("9"),
            json!([]),
            json!([9, "x"]),
            json!({"id": 9}),
            json!(true),
        ] {
            assert_eq!(create.after_read(&bad), None, "{bad}");
        }
    }

    #[test]
    fn a_workflow_write_is_read_before_and_after_as_a_workflow() {
        let get = Some(R::N8nGetWorkflow {
            workflow_id: id("wf1"),
        });
        for tool in [
            "n8n_publish_workflow",
            "n8n_unpublish_workflow",
            "n8n_archive_workflow",
        ] {
            let request = parse(tool, json!({"workflow_id": "wf1"}));
            assert_eq!(request.before_read(), get, "{tool}");
        }
        let update = parse(
            "n8n_update_workflow",
            json!({"workflow_id": "wf1", "nodes": [{"a": 1}], "connections": {}}),
        );
        assert_eq!(update.before_read(), get);
        assert_eq!(update.after_read(&json!({"id": "wf1"})), get);
        for tool in ["n8n_publish_workflow", "n8n_unpublish_workflow"] {
            let request = parse(tool, json!({"workflow_id": "wf1"}));
            assert_eq!(request.after_read(&json!({})), get, "{tool}");
        }
        let archive = parse("n8n_archive_workflow", json!({"workflow_id": "wf1"}));
        assert_eq!(archive.after_read(&json!({})), None);
        let unarchive = R::N8nUnarchiveWorkflow {
            workflow_id: id("wf1"),
        };
        assert_eq!(unarchive.before_read(), get);
        assert_eq!(unarchive.after_read(&json!({})), get);
    }

    #[test]
    fn workflow_tags_are_read_before_and_after_as_tags() {
        let request = parse(
            "n8n_set_workflow_tags",
            json!({"workflow_id": "wf1", "tag_ids": ["a"]}),
        );
        let tags = Some(R::N8nGetWorkflowTags {
            workflow_id: id("wf1"),
        });
        assert_eq!(request.before_read(), tags);
        assert_eq!(request.after_read(&json!([])), tags);
    }

    #[test]
    fn a_created_workflow_is_read_after_from_its_returned_id() {
        let create = parse(
            "n8n_create_workflow",
            json!({"name": "Relance", "nodes": [{"a": 1}]}),
        );
        assert_eq!(create.before_read(), None);
        assert_eq!(
            create.after_read(&json!({"id": "wf9", "name": "Relance"})),
            Some(R::N8nGetWorkflow {
                workflow_id: id("wf9")
            })
        );
        for bad in [
            json!({}),
            json!({"id": null}),
            json!({"id": "a b"}),
            json!({"id": 3}),
            json!("wf9"),
            json!(null),
        ] {
            assert_eq!(create.after_read(&bad), None, "{bad}");
        }
    }

    #[test]
    fn the_other_actions_have_no_copy() {
        for tool in [
            "odoo_call_method",
            "n8n_create_tag",
            "n8n_retry_execution",
            "n8n_stop_execution",
        ] {
            let (_, arguments) = minimal()
                .into_iter()
                .find(|(name, _)| *name == tool)
                .unwrap();
            let request = parse(tool, arguments);
            assert_eq!(request.before_read(), None, "{tool}");
            assert_eq!(request.after_read(&json!({"id": "1"})), None, "{tool}");
        }
        let delete_tag = R::N8nDeleteTag { tag_id: id("t1") };
        assert_eq!(delete_tag.before_read(), None);
        assert_eq!(delete_tag.after_read(&json!({})), None);
        for (tool, arguments) in minimal() {
            if !WRITES.contains(&tool) {
                let request = parse(tool, arguments);
                assert_eq!(request.before_read(), None, "{tool}");
                assert_eq!(request.after_read(&json!({"id": "1"})), None, "{tool}");
            }
        }
    }

    #[test]
    fn an_odoo_write_describes_its_target_and_arguments() {
        let write = parse(
            "odoo_write",
            json!({"model": "res.partner", "ids": [2, 1], "values": {"name": "Acme"}}),
        );
        assert_eq!(
            write.target(),
            json!({"model": "res.partner", "ids": [2, 1]})
        );
        assert_eq!(
            write.arguments(),
            json!({"model": "res.partner", "ids": [2, 1], "values": {"name": "Acme"}})
        );
    }

    #[test]
    fn an_n8n_update_describes_its_target_and_arguments_with_defaults() {
        let update = parse(
            "n8n_update_workflow",
            json!({"workflow_id": "wf1", "nodes": [{"name": "Start"}], "connections": {}}),
        );
        assert_eq!(update.target(), json!({"workflow_id": "wf1"}));
        assert_eq!(
            update.arguments(),
            json!({
                "workflow_id": "wf1",
                "nodes": [{"name": "Start"}],
                "connections": {},
                "publish": false
            })
        );
        assert!(update.arguments().get("instance").is_none());
    }

    #[test]
    fn odoo_targets_name_the_model_and_what_it_touches() {
        let target = |tool: &str, arguments: Value| parse(tool, arguments).target();
        assert_eq!(
            target("odoo_list_models", json!({})),
            json!({"model": "ir.model"})
        );
        assert_eq!(
            target("odoo_describe_model", json!({"model": "res.partner"})),
            json!({"model": "res.partner"})
        );
        assert_eq!(
            target(
                "odoo_search_read",
                json!({"model": "res.partner", "domain": [["name", "=", "a"]]})
            ),
            json!({"model": "res.partner", "domain": [["name", "=", "a"]]})
        );
        assert_eq!(
            target("odoo_count", json!({"model": "res.partner"})),
            json!({"model": "res.partner", "domain": []})
        );
        assert_eq!(
            target("odoo_read", json!({"model": "res.partner", "ids": [1]})),
            json!({"model": "res.partner", "ids": [1]})
        );
        assert_eq!(
            target("odoo_read_attachment", json!({"attachment_id": 7})),
            json!({"attachment_id": 7})
        );
        assert_eq!(
            target(
                "odoo_create",
                json!({"model": "res.partner", "values": {"a": 1}})
            ),
            json!({"model": "res.partner"})
        );
        assert_eq!(
            target("odoo_delete", json!({"model": "res.partner", "ids": [3]})),
            json!({"model": "res.partner", "ids": [3]})
        );
        assert_eq!(
            target(
                "odoo_call_method",
                json!({"model": "sale.order", "method": "action_confirm"})
            ),
            json!({"model": "sale.order", "method": "action_confirm"})
        );
        assert_eq!(
            target(
                "odoo_call_method",
                json!({"model": "sale.order", "method": "action_confirm", "ids": [8]})
            ),
            json!({"model": "sale.order", "method": "action_confirm", "ids": [8]})
        );
    }

    #[test]
    fn n8n_targets_name_the_resource_and_the_filters_given() {
        let target = |tool: &str, arguments: Value| parse(tool, arguments).target();
        assert_eq!(
            target("n8n_list_workflows", json!({})),
            json!({"resource": "workflows"})
        );
        assert_eq!(
            target(
                "n8n_list_workflows",
                json!({"name": "Relance", "active": true, "tags": "a", "project_id": "p1", "limit": 5})
            ),
            json!({"resource": "workflows", "name": "Relance", "active": true, "tags": "a", "project_id": "p1"})
        );
        assert_eq!(
            target(
                "n8n_list_executions",
                json!({"workflow_id": "wf1", "status": "error"})
            ),
            json!({"resource": "executions", "workflow_id": "wf1", "status": "error"})
        );
        assert_eq!(
            target("n8n_list_tags", json!({})),
            json!({"resource": "tags"})
        );
        assert_eq!(
            target("n8n_list_credentials", json!({})),
            json!({"resource": "credentials"})
        );
        assert_eq!(
            target("n8n_get_workflow", json!({"workflow_id": "wf1"})),
            json!({"workflow_id": "wf1"})
        );
        assert_eq!(
            target("n8n_get_execution", json!({"execution_id": "42"})),
            json!({"execution_id": "42"})
        );
        assert_eq!(
            target("n8n_stop_execution", json!({"execution_id": "42"})),
            json!({"execution_id": "42"})
        );
        assert_eq!(
            target(
                "n8n_create_workflow",
                json!({"name": "Relance", "nodes": [{"a": 1}]})
            ),
            json!({"workflow_name": "Relance"})
        );
        assert_eq!(
            target("n8n_create_tag", json!({"name": "urgent"})),
            json!({"tag_name": "urgent"})
        );
        assert_eq!(
            R::N8nDeleteTag { tag_id: id("t1") }.target(),
            json!({"tag_id": "t1"})
        );
        assert_eq!(
            R::N8nGetWorkflowTags {
                workflow_id: id("wf1")
            }
            .target(),
            json!({"workflow_id": "wf1"})
        );
    }

    #[test]
    fn arguments_fill_their_defaults_and_leave_out_what_was_not_given() {
        let arguments = |tool: &str, arguments: Value| parse(tool, arguments).arguments();
        assert_eq!(
            arguments("odoo_search_read", json!({"model": "res.partner"})),
            json!({"model": "res.partner", "domain": [], "fields": [], "limit": 50, "offset": 0})
        );
        assert_eq!(
            arguments("odoo_list_models", json!({"query": "sale"})),
            json!({"query": "sale", "limit": 50})
        );
        assert_eq!(
            arguments(
                "odoo_call_method",
                json!({"model": "sale.order", "method": "action_confirm"})
            ),
            json!({"model": "sale.order", "method": "action_confirm", "ids": [], "args": [], "kwargs": {}})
        );
        assert_eq!(
            arguments("n8n_list_executions", json!({})),
            json!({"limit": 20})
        );
        assert_eq!(
            arguments("n8n_get_execution", json!({"execution_id": "42"})),
            json!({"execution_id": "42", "include_data": false})
        );
        assert_eq!(
            arguments(
                "n8n_create_workflow",
                json!({"name": "Relance", "nodes": [{"a": 1}]})
            ),
            json!({"name": "Relance", "nodes": [{"a": 1}], "connections": {}, "settings": {}})
        );
        assert_eq!(
            arguments(
                "n8n_set_workflow_tags",
                json!({"workflow_id": "wf1", "tag_ids": ["a"]})
            ),
            json!({"workflow_id": "wf1", "tag_ids": ["a"]})
        );
        let status = R::N8nListExecutions {
            workflow_id: None,
            status: Some(ExecutionStatus::Waiting),
            limit: 20,
            cursor: Some("c".into()),
        };
        assert_eq!(
            status.arguments(),
            json!({"status": "waiting", "limit": 20, "cursor": "c"})
        );
    }
}
