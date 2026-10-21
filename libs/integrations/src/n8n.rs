use std::borrow::Cow;

use capitainerie_core::domain::integration::action::{ActionRequest, N8nId};
use capitainerie_core::domain::integration::{ConnectorFailure, InstanceConnection, ProbeReport};
use reqwest::header::{ACCEPT, HeaderValue};
use reqwest::{Client, Method, RequestBuilder};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::guard::{remote_failure, secret_header, send};
use crate::odoo::NOT_FOR_THIS_INSTANCE;

const API_KEY_HEADER: &str = "X-N8N-API-KEY";
const DISCOVERY_VERSION: &str = "2";
const LEGACY_VERSION: &str = "1";
const CREDENTIAL_FIELDS: [&str; 5] = ["id", "name", "type", "createdAt", "updatedAt"];
const VERSIONS_UNSUPPORTED: &str = "workflow versions need n8n 2.x";
const ARCHIVE_UNSUPPORTED: &str = "archiving needs n8n 2.x";
const NO_BODY: Option<&()> = None;

type Query<'a> = Vec<(&'static str, Cow<'a, str>)>;

#[derive(Serialize)]
struct WorkflowBody<'a> {
    name: &'a str,
    nodes: &'a [Value],
    connections: &'a Map<String, Value>,
    settings: &'a Map<String, Value>,
}

#[derive(Serialize)]
struct IdRef<'a> {
    id: &'a str,
}

#[derive(Serialize)]
struct NameBody<'a> {
    name: &'a str,
}

fn text<'a>(query: &mut Query<'a>, name: &'static str, value: Option<&'a str>) {
    if let Some(value) = value {
        query.push((name, Cow::Borrowed(value)));
    }
}

fn number(query: &mut Query<'_>, name: &'static str, value: u32) {
    query.push((name, Cow::Owned(value.to_string())));
}

fn flag(query: &mut Query<'_>, name: &'static str, value: bool) {
    query.push((name, Cow::Borrowed(if value { "true" } else { "false" })));
}

fn unsupported_on_404(message: &'static str) -> impl FnOnce(ConnectorFailure) -> ConnectorFailure {
    move |failure| match failure {
        ConnectorFailure::Remote { status: 404, .. } => {
            ConnectorFailure::NotSupported(message.into())
        }
        other => other,
    }
}

fn credential_metadata(body: Value) -> Result<Value, ConnectorFailure> {
    let Value::Object(mut page) = body else {
        return Err(unexpected_credentials());
    };
    let Some(Value::Array(items)) = page.remove("data") else {
        return Err(unexpected_credentials());
    };
    let data: Vec<Value> = items
        .into_iter()
        .filter_map(|item| match item {
            Value::Object(mut fields) => Some(Value::Object(
                CREDENTIAL_FIELDS
                    .iter()
                    .filter_map(|key| fields.remove(*key).map(|value| ((*key).to_owned(), value)))
                    .collect(),
            )),
            _ => None,
        })
        .collect();
    let mut kept = Map::new();
    kept.insert("data".into(), Value::Array(data));
    if let Some(next) = page.remove("nextCursor") {
        kept.insert("nextCursor".into(), next);
    }
    Ok(Value::Object(kept))
}

fn unexpected_credentials() -> ConnectorFailure {
    ConnectorFailure::Protocol("the credentials list has an unexpected shape".into())
}

pub(crate) struct N8n<'a> {
    client: &'a Client,
    connection: &'a InstanceConnection,
}

impl<'a> N8n<'a> {
    pub(crate) fn new(client: &'a Client, connection: &'a InstanceConnection) -> Self {
        Self { client, connection }
    }

    pub(crate) async fn probe(&self) -> Result<ProbeReport, ConnectorFailure> {
        let secret = self.connection.secret.expose();
        let key = secret_header("", secret)?;
        let workflows = send(
            self.client
                .get(self.connection.base_url.join("api/v1/workflows?limit=1"))
                .header(API_KEY_HEADER, key.clone())
                .header(ACCEPT, HeaderValue::from_static("application/json")),
        )
        .await?;
        match workflows.status.as_u16() {
            200 => {}
            401 | 403 => return Err(ConnectorFailure::Unauthorized),
            404 => {
                return Err(ConnectorFailure::Protocol(
                    "no n8n public API at this URL (is the public API enabled?)".into(),
                ));
            }
            _ => return Err(remote_failure(&workflows, secret)),
        }
        let discovery = send(
            self.client
                .get(self.connection.base_url.join("api/v1/discover"))
                .header(API_KEY_HEADER, key)
                .header(ACCEPT, HeaderValue::from_static("application/json")),
        )
        .await?;
        let server_version = if discovery.status.as_u16() == 404 {
            LEGACY_VERSION
        } else {
            DISCOVERY_VERSION
        };
        Ok(ProbeReport {
            server_version: server_version.to_owned(),
            warning: None,
        })
    }

    pub(crate) async fn perform(&self, request: &ActionRequest) -> Result<Value, ConnectorFailure> {
        match request {
            ActionRequest::N8nListWorkflows {
                name,
                tags,
                active,
                project_id,
                limit,
                cursor,
            } => {
                let mut query = Query::new();
                text(&mut query, "name", name.as_deref());
                text(&mut query, "tags", tags.as_deref());
                if let Some(active) = active {
                    flag(&mut query, "active", *active);
                }
                text(
                    &mut query,
                    "projectId",
                    project_id.as_ref().map(N8nId::as_str),
                );
                number(&mut query, "limit", *limit);
                text(&mut query, "cursor", cursor.as_deref());
                self.get("workflows", &query).await
            }
            ActionRequest::N8nGetWorkflow { workflow_id } => {
                self.get(
                    &format!("workflows/{}", workflow_id.as_str()),
                    &Query::new(),
                )
                .await
            }
            ActionRequest::N8nListWorkflowVersions { workflow_id } => self
                .get(
                    &format!("workflows/{}/history", workflow_id.as_str()),
                    &Query::new(),
                )
                .await
                .map_err(unsupported_on_404(VERSIONS_UNSUPPORTED)),
            ActionRequest::N8nGetWorkflowTags { workflow_id } => {
                self.get(
                    &format!("workflows/{}/tags", workflow_id.as_str()),
                    &Query::new(),
                )
                .await
            }
            ActionRequest::N8nListExecutions {
                workflow_id,
                status,
                limit,
                cursor,
            } => {
                let mut query = Query::new();
                text(
                    &mut query,
                    "workflowId",
                    workflow_id.as_ref().map(N8nId::as_str),
                );
                text(&mut query, "status", status.map(|status| status.as_str()));
                number(&mut query, "limit", *limit);
                text(&mut query, "cursor", cursor.as_deref());
                self.get("executions", &query).await
            }
            ActionRequest::N8nGetExecution {
                execution_id,
                include_data,
            } => {
                let mut query = Query::new();
                flag(&mut query, "includeData", *include_data);
                self.get(&format!("executions/{}", execution_id.as_str()), &query)
                    .await
            }
            ActionRequest::N8nListTags { limit, cursor } => {
                let mut query = Query::new();
                number(&mut query, "limit", *limit);
                text(&mut query, "cursor", cursor.as_deref());
                self.get("tags", &query).await
            }
            ActionRequest::N8nListCredentials { limit, cursor } => {
                let mut query = Query::new();
                number(&mut query, "limit", *limit);
                text(&mut query, "cursor", cursor.as_deref());
                credential_metadata(self.get("credentials", &query).await?)
            }
            ActionRequest::N8nCreateWorkflow {
                name,
                nodes,
                connections,
                settings,
            } => {
                let body = WorkflowBody {
                    name,
                    nodes,
                    connections,
                    settings,
                };
                self.send_with(Method::POST, "workflows", &Query::new(), Some(&body))
                    .await
            }
            ActionRequest::N8nUpdateWorkflow {
                workflow_id,
                name,
                nodes,
                connections,
                settings,
                publish,
            } => {
                let (Some(name), Some(settings)) = (name, settings) else {
                    return Err(ConnectorFailure::Protocol(
                        "the update needs the full workflow: name and settings".into(),
                    ));
                };
                let body = WorkflowBody {
                    name,
                    nodes,
                    connections,
                    settings,
                };
                let mut query = Query::new();
                flag(&mut query, "publishIfActive", *publish);
                self.send_with(
                    Method::PUT,
                    &format!("workflows/{}", workflow_id.as_str()),
                    &query,
                    Some(&body),
                )
                .await
            }
            ActionRequest::N8nPublishWorkflow { workflow_id } => {
                self.switch(workflow_id, "publish", "activate").await
            }
            ActionRequest::N8nUnpublishWorkflow { workflow_id } => {
                self.switch(workflow_id, "unpublish", "deactivate").await
            }
            ActionRequest::N8nArchiveWorkflow { workflow_id } => self
                .post(&format!("workflows/{}/archive", workflow_id.as_str()))
                .await
                .map_err(unsupported_on_404(ARCHIVE_UNSUPPORTED)),
            ActionRequest::N8nUnarchiveWorkflow { workflow_id } => self
                .post(&format!("workflows/{}/unarchive", workflow_id.as_str()))
                .await
                .map_err(unsupported_on_404(ARCHIVE_UNSUPPORTED)),
            ActionRequest::N8nSetWorkflowTags {
                workflow_id,
                tag_ids,
            } => {
                let body: Vec<IdRef<'_>> = tag_ids
                    .iter()
                    .map(|tag| IdRef { id: tag.as_str() })
                    .collect();
                self.send_with(
                    Method::PUT,
                    &format!("workflows/{}/tags", workflow_id.as_str()),
                    &Query::new(),
                    Some(&body),
                )
                .await
            }
            ActionRequest::N8nCreateTag { name } => {
                self.send_with(
                    Method::POST,
                    "tags",
                    &Query::new(),
                    Some(&NameBody { name }),
                )
                .await
            }
            ActionRequest::N8nDeleteTag { tag_id } => {
                self.send_with(
                    Method::DELETE,
                    &format!("tags/{}", tag_id.as_str()),
                    &Query::new(),
                    NO_BODY,
                )
                .await
            }
            ActionRequest::N8nRetryExecution { execution_id } => {
                self.send_with(
                    Method::POST,
                    &format!("executions/{}/retry", execution_id.as_str()),
                    &Query::new(),
                    Some(&json!({})),
                )
                .await
            }
            ActionRequest::N8nStopExecution { execution_id } => {
                self.post(&format!("executions/{}/stop", execution_id.as_str()))
                    .await
            }
            _ => Err(ConnectorFailure::NotSupported(NOT_FOR_THIS_INSTANCE.into())),
        }
    }

    async fn switch(
        &self,
        workflow_id: &N8nId,
        route: &str,
        legacy_route: &str,
    ) -> Result<Value, ConnectorFailure> {
        match self
            .post(&format!("workflows/{}/{route}", workflow_id.as_str()))
            .await
        {
            Err(ConnectorFailure::Remote { status: 404, .. }) => {
                self.post(&format!(
                    "workflows/{}/{legacy_route}",
                    workflow_id.as_str()
                ))
                .await
            }
            other => other,
        }
    }

    async fn get(&self, path: &str, query: &Query<'_>) -> Result<Value, ConnectorFailure> {
        self.send_with(Method::GET, path, query, NO_BODY).await
    }

    async fn post(&self, path: &str) -> Result<Value, ConnectorFailure> {
        self.send_with(Method::POST, path, &Query::new(), NO_BODY)
            .await
    }

    async fn send_with<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        query: &Query<'_>,
        body: Option<&B>,
    ) -> Result<Value, ConnectorFailure> {
        let secret = self.connection.secret.expose();
        let mut request: RequestBuilder = self
            .client
            .request(
                method,
                self.connection.base_url.join(&format!("api/v1/{path}")),
            )
            .header(API_KEY_HEADER, secret_header("", secret)?)
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .query(query);
        if let Some(body) = body {
            request = request.json(body);
        }
        let answer = send(request).await?;
        match answer.status.as_u16() {
            200..=299 if answer.body.iter().all(u8::is_ascii_whitespace) => Ok(Value::Null),
            200..=299 => answer.json().ok_or_else(|| {
                ConnectorFailure::Protocol("the server did not answer with JSON".into())
            }),
            401 | 403 => Err(ConnectorFailure::Unauthorized),
            _ => Err(remote_failure(&answer, secret)),
        }
    }
}
