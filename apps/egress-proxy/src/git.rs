use std::{io, sync::Arc};

use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Uri, header},
    response::{IntoResponse, Response},
};
use capitainerie_common::RunId;
use futures_util::{StreamExt, stream};

use crate::{
    auth,
    credentials::Credentials,
    error::ProxyError,
    forward::{self, MAX_BODY},
    pktline::{self, CommandSection},
    proxy::Proxy,
};

const GIT_PROTOCOL: HeaderName = HeaderName::from_static("git-protocol");
const KEPT: [HeaderName; 4] = [
    header::CONTENT_TYPE,
    header::ACCEPT,
    GIT_PROTOCOL,
    header::USER_AGENT,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    InfoRefs(&'static str),
    UploadPack,
    ReceivePack,
}

impl Operation {
    fn classify(method: &Method, rest: &str, query: Option<&str>) -> Option<Self> {
        match (method.as_str(), rest, query) {
            ("GET", "info/refs", Some("service=git-upload-pack")) => {
                Some(Self::InfoRefs("git-upload-pack"))
            }
            ("GET", "info/refs", Some("service=git-receive-pack")) => {
                Some(Self::InfoRefs("git-receive-pack"))
            }
            ("POST", "git-upload-pack", _) => Some(Self::UploadPack),
            ("POST", "git-receive-pack", _) => Some(Self::ReceivePack),
            _ => None,
        }
    }

    fn class(op: Option<Self>) -> &'static str {
        match op {
            Some(Self::InfoRefs(_)) => "git:info-refs",
            Some(Self::UploadPack) => "git:upload-pack",
            Some(Self::ReceivePack) => "git:receive-pack",
            None => "git:other",
        }
    }
}

pub async fn handle<C: Credentials>(
    State(proxy): State<Arc<Proxy<C>>>,
    method: Method,
    Path((owner, repo, rest)): Path<(String, String, String)>,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let operation = Operation::classify(&method, &rest, uri.query());
    match run(&proxy, operation, &owner, &repo, &headers, body).await {
        Ok(response) => response,
        Err(error) => {
            error.log(&method, Operation::class(operation));
            error.into_response()
        }
    }
}

async fn run<C: Credentials>(
    proxy: &Proxy<C>,
    operation: Option<Operation>,
    owner: &str,
    repo: &str,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response, ProxyError> {
    let token = auth::basic_token(headers).ok_or(ProxyError::Unauthorized {
        reason: "no token presented",
    })?;
    let grant = proxy.authenticate(&token).await?;
    let run_id = grant.run_id;
    let permit = proxy.acquire(run_id)?;
    let repository = &grant.repository;
    if owner != repository.owner() || repo.strip_suffix(".git") != Some(repository.name()) {
        return Err(ProxyError::forbidden(
            run_id,
            "the repository is not the one of this execution",
        ));
    }
    let operation = operation.ok_or(ProxyError::forbidden(
        run_id,
        "only fetch and push of the execution branch are allowed",
    ))?;
    if forward::declared_length(headers).is_some_and(|length| length > MAX_BODY) {
        return Err(ProxyError::TooLarge { run_id });
    }

    let mut upstream_headers = forward::copy_allowed(headers, &KEPT);
    let base = format!(
        "{}/{}/{}.git",
        proxy.config.github_base,
        repository.owner(),
        repository.name()
    );

    let request = match operation {
        Operation::InfoRefs(service) => {
            installation_token(proxy, &grant, &mut upstream_headers).await?;
            proxy
                .http
                .get(format!("{base}/info/refs?service={service}"))
                .headers(upstream_headers)
        }
        Operation::UploadPack => {
            if let Some(encoding) = headers.get(header::CONTENT_ENCODING) {
                upstream_headers.insert(header::CONTENT_ENCODING, encoding.clone());
            }
            installation_token(proxy, &grant, &mut upstream_headers).await?;
            proxy
                .http
                .post(format!("{base}/git-upload-pack"))
                .headers(upstream_headers)
                .body(reqwest::Body::wrap_stream(forward::capped(body)))
        }
        Operation::ReceivePack => {
            if headers.contains_key(header::CONTENT_ENCODING) {
                return Err(ProxyError::forbidden(
                    run_id,
                    "an encoded push cannot be inspected",
                ));
            }
            let mut stream = Box::pin(forward::capped(body));
            let prefix = read_commands(&mut stream, &grant.branch, run_id).await?;
            installation_token(proxy, &grant, &mut upstream_headers).await?;
            let body = stream::once(async move { Ok::<Bytes, io::Error>(prefix) }).chain(stream);
            proxy
                .http
                .post(format!("{base}/git-receive-pack"))
                .headers(upstream_headers)
                .body(reqwest::Body::wrap_stream(body))
        }
    };
    let upstream = request
        .send()
        .await
        .map_err(|error| ProxyError::Upstream(error.without_url().to_string()))?;
    Ok(forward::relay(upstream, permit))
}

async fn installation_token<C: Credentials>(
    proxy: &Proxy<C>,
    grant: &capitainerie_core::domain::execution::TaskGrant,
    headers: &mut HeaderMap,
) -> Result<(), ProxyError> {
    let token = proxy
        .credentials
        .github_token(grant.organization_id, &grant.repository)
        .await?
        .ok_or(ProxyError::forbidden(
            grant.run_id,
            "GitHub is not connected for the organization",
        ))?;
    let mut value = HeaderValue::from_str(&auth::basic_header(token.expose())).map_err(|_| {
        ProxyError::forbidden(grant.run_id, "the GitHub installation token is unusable")
    })?;
    value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, value);
    Ok(())
}

async fn read_commands<S>(
    stream: &mut std::pin::Pin<Box<S>>,
    branch: &str,
    run_id: RunId,
) -> Result<Bytes, ProxyError>
where
    S: futures_util::Stream<Item = Result<Bytes, io::Error>>,
{
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        match pktline::parse_commands(&buffer) {
            Ok(CommandSection::Complete(commands)) => {
                pktline::check_push(&commands, branch)
                    .map_err(|reason| ProxyError::forbidden(run_id, reason))?;
                return Ok(Bytes::from(buffer));
            }
            Ok(CommandSection::Incomplete) => {}
            Err(_) => {
                return Err(ProxyError::forbidden(
                    run_id,
                    "the push commands could not be read",
                ));
            }
        }
        match stream.next().await {
            Some(Ok(chunk)) => buffer.extend_from_slice(&chunk),
            Some(Err(_)) => {
                return Err(ProxyError::forbidden(
                    run_id,
                    "the push body could not be read",
                ));
            }
            None => {
                return Err(ProxyError::forbidden(
                    run_id,
                    "the push commands could not be read",
                ));
            }
        }
    }
}
