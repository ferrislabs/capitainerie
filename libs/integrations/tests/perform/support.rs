use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use capitainerie_common::IntegrationInstanceId;
use capitainerie_core::domain::agent::SecretToken;
use capitainerie_core::domain::integration::{
    BaseUrl, ConnectorFailure, InstanceConnection, IntegrationKind,
    action::{ActionRequest, N8nId, OdooMethod, OdooModel},
    ports::IntegrationConnector,
};
use capitainerie_integrations::{HttpConnector, NetworkPolicy};
use serde_json::{Map, Value};

pub const SECRET: &str = "s3cr3t-token-value";

pub const OPEN: NetworkPolicy = NetworkPolicy {
    allow_insecure_http: true,
    allow_private_networks: true,
};

#[derive(Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: HeaderMap,
    pub body: String,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

pub struct Reply {
    status: u16,
    content_type: Option<&'static str>,
    body: Vec<u8>,
}

impl Reply {
    pub fn status(status: u16) -> Self {
        Self {
            status,
            content_type: None,
            body: Vec::new(),
        }
    }

    pub fn json(status: u16, body: &Value) -> Self {
        Self {
            status,
            content_type: Some("application/json"),
            body: body.to_string().into_bytes(),
        }
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: Some("text/plain"),
            body: body.as_bytes().to_vec(),
        }
    }
}

impl IntoResponse for Reply {
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() = StatusCode::from_u16(self.status).unwrap();
        if let Some(content_type) = self.content_type {
            response.headers_mut().insert(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static(content_type),
            );
        }
        response
    }
}

struct Shared<F> {
    respond: F,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

async fn handle<F>(State(shared): State<Arc<Shared<F>>>, request: Request) -> Response
where
    F: Fn(&Recorded) -> Reply + Send + Sync + 'static,
{
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    let recorded = Recorded {
        method: parts.method.to_string(),
        path: parts
            .uri
            .path_and_query()
            .map(|path| path.as_str().to_owned())
            .unwrap_or_default(),
        headers: parts.headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    };
    let reply = (shared.respond)(&recorded);
    shared.recorded.lock().unwrap().push(recorded);
    reply.into_response()
}

pub struct Fake {
    pub base: String,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

impl Fake {
    pub fn hits(&self) -> usize {
        self.recorded.lock().unwrap().len()
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.recorded.lock().unwrap().clone()
    }

    pub fn only(&self) -> Recorded {
        let mut requests = self.requests();
        assert_eq!(requests.len(), 1, "expected exactly one request");
        requests.remove(0)
    }

    pub fn paths(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|recorded| format!("{} {}", recorded.method, recorded.path))
            .collect()
    }

    pub fn rpc_calls(&self, method: &str) -> Vec<Recorded> {
        self.requests()
            .into_iter()
            .filter(|recorded| recorded.json()["params"]["method"] == method)
            .collect()
    }
}

pub async fn serve<F>(respond: F) -> Fake
where
    F: Fn(&Recorded) -> Reply + Send + Sync + 'static,
{
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::new(Shared {
        respond,
        recorded: Arc::clone(&recorded),
    });
    let app = Router::new().fallback(handle::<F>).with_state(shared);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Fake {
        base: format!("http://{address}"),
        recorded,
    }
}

pub fn connection(
    kind: IntegrationKind,
    base: &str,
    database: Option<&str>,
    login: Option<&str>,
) -> InstanceConnection {
    InstanceConnection {
        instance_id: IntegrationInstanceId::new(),
        kind,
        base_url: BaseUrl::try_from(base.to_owned()).unwrap(),
        database: database.map(str::to_owned),
        login: login.map(str::to_owned),
        secret: SecretToken::new(SECRET),
    }
}

pub fn json2(base: &str) -> InstanceConnection {
    connection(IntegrationKind::OdooJson2, base, Some("prod"), None)
}

pub fn jsonrpc(base: &str) -> InstanceConnection {
    connection(
        IntegrationKind::OdooJsonRpc,
        base,
        Some("prod"),
        Some("admin@acme.test"),
    )
}

pub fn n8n(base: &str) -> InstanceConnection {
    connection(IntegrationKind::N8n, base, None, None)
}

pub fn connector() -> HttpConnector {
    HttpConnector::new(OPEN).unwrap()
}

pub async fn perform_with(
    connector: &HttpConnector,
    connection: &InstanceConnection,
    request: &ActionRequest,
) -> Result<Value, ConnectorFailure> {
    let outcome = connector.perform(connection, request).await;
    match &outcome {
        Ok(value) => assert!(
            !value.to_string().contains(SECRET),
            "the secret leaked in {value}"
        ),
        Err(failure) => {
            assert!(
                !failure.to_string().contains(SECRET),
                "the secret leaked in {failure}"
            );
            assert!(
                !format!("{failure:?}").contains(SECRET),
                "the secret leaked in {failure:?}"
            );
        }
    }
    outcome
}

pub async fn perform(
    connection: &InstanceConnection,
    request: &ActionRequest,
) -> Result<Value, ConnectorFailure> {
    perform_with(&connector(), connection, request).await
}

pub fn model(name: &str) -> OdooModel {
    OdooModel::try_from(name).unwrap()
}

pub fn method(name: &str) -> OdooMethod {
    OdooMethod::try_from(name).unwrap()
}

pub fn id(value: &str) -> N8nId {
    N8nId::try_from(value).unwrap()
}

pub fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("{other} is not an object"),
    }
}

pub fn array(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items,
        other => panic!("{other} is not an array"),
    }
}
