use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use capitainerie_common::IntegrationInstanceId;
use capitainerie_core::domain::agent::SecretToken;
use capitainerie_core::domain::integration::{
    BaseUrl, ConnectorFailure, InstanceConnection, IntegrationKind, ProbeReport,
    ports::IntegrationConnector,
};
use capitainerie_integrations::{HttpConnector, NetworkPolicy};
use serde_json::Value;

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

enum ReplyBody {
    Bytes(Vec<u8>),
    Stream { chunks: usize, size: usize },
}

pub struct Reply {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: ReplyBody,
}

impl Reply {
    pub fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: ReplyBody::Bytes(Vec::new()),
        }
    }

    pub fn json(status: u16, body: &Value) -> Self {
        Self {
            status,
            headers: vec![("content-type", "application/json".to_owned())],
            body: ReplyBody::Bytes(body.to_string().into_bytes()),
        }
    }

    pub fn redirect(location: &str) -> Self {
        Self {
            status: 302,
            headers: vec![("location", location.to_owned())],
            body: ReplyBody::Bytes(Vec::new()),
        }
    }

    pub fn bytes(status: u16, length: usize) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: ReplyBody::Bytes(vec![b'a'; length]),
        }
    }

    pub fn stream(chunks: usize, size: usize) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: ReplyBody::Stream { chunks, size },
        }
    }
}

impl IntoResponse for Reply {
    fn into_response(self) -> Response {
        let body = match self.body {
            ReplyBody::Bytes(bytes) => Body::from(bytes),
            ReplyBody::Stream { chunks, size } => Body::from_stream(futures_util::stream::iter(
                (0..chunks).map(move |_| Ok::<_, Infallible>(Bytes::from(vec![b'a'; size]))),
            )),
        };
        let mut response = Response::new(body);
        *response.status_mut() = StatusCode::from_u16(self.status).unwrap();
        for (name, value) in self.headers {
            response.headers_mut().insert(
                HeaderName::from_static(name),
                HeaderValue::from_str(&value).unwrap(),
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
    pub port: u16,
    recorded: Arc<Mutex<Vec<Recorded>>>,
}

impl Fake {
    pub fn hits(&self) -> usize {
        self.recorded.lock().unwrap().len()
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.recorded.lock().unwrap().clone()
    }

    pub fn request(&self, method: &str, path: &str) -> Recorded {
        self.requests()
            .into_iter()
            .find(|recorded| recorded.method == method && recorded.path == path)
            .unwrap_or_else(|| panic!("no {method} {path} reached the fake server"))
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
        port: address.port(),
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

pub async fn probe_with(
    policy: NetworkPolicy,
    connection: &InstanceConnection,
) -> Result<ProbeReport, ConnectorFailure> {
    let connector = HttpConnector::new(policy).unwrap();
    let outcome = connector.probe(connection).await;
    if let Err(failure) = &outcome {
        assert!(
            !failure.to_string().contains(SECRET),
            "the secret leaked in {failure}"
        );
        assert!(
            !format!("{failure:?}").contains(SECRET),
            "the secret leaked in {failure:?}"
        );
    }
    outcome
}

pub async fn probe(connection: &InstanceConnection) -> Result<ProbeReport, ConnectorFailure> {
    probe_with(OPEN, connection).await
}

pub fn ok(version: &str) -> Result<ProbeReport, ConnectorFailure> {
    Ok(ProbeReport {
        server_version: version.to_owned(),
        warning: None,
    })
}
