use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Uri, header},
    response::{IntoResponse, Response},
};

use crate::{
    auth,
    credentials::Credentials,
    error::ProxyError,
    forward::{self, MAX_BODY},
    proxy::Proxy,
};

const CLASS: &str = "anthropic";
const ALLOWED_PATHS: [&str; 2] = ["v1/messages", "v1/messages/count_tokens"];
const KEPT: [HeaderName; 4] = [
    HeaderName::from_static("anthropic-version"),
    HeaderName::from_static("anthropic-beta"),
    header::CONTENT_TYPE,
    header::ACCEPT,
];

pub async fn handle<C: Credentials>(
    State(proxy): State<Arc<Proxy<C>>>,
    method: Method,
    Path(path): Path<String>,
    uri: Uri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    match run(&proxy, &method, &path, &uri, &headers, body).await {
        Ok(response) => response,
        Err(error) => {
            error.log(&method, CLASS);
            error.into_response()
        }
    }
}

async fn run<C: Credentials>(
    proxy: &Proxy<C>,
    method: &Method,
    path: &str,
    uri: &Uri,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response, ProxyError> {
    let token = auth::api_token(headers).ok_or(ProxyError::Unauthorized {
        reason: "no token presented",
    })?;
    let grant = proxy.authenticate(token).await?;
    let run_id = grant.run_id;
    if method != Method::POST || !ALLOWED_PATHS.contains(&path) {
        return Err(ProxyError::forbidden(
            run_id,
            "only POST /v1/messages and /v1/messages/count_tokens are allowed",
        ));
    }
    if forward::declared_length(headers).is_some_and(|length| length > MAX_BODY) {
        return Err(ProxyError::TooLarge { run_id });
    }
    let key = proxy
        .credentials
        .anthropic_key(grant.organization_id)
        .await?
        .ok_or(ProxyError::forbidden(
            run_id,
            "no Anthropic key is configured for the organization",
        ))?;

    let mut upstream_headers = forward::copy_allowed(headers, &KEPT);
    let mut key_value = HeaderValue::from_str(key.expose()).map_err(|_| {
        ProxyError::forbidden(run_id, "the organization's Anthropic key is unusable")
    })?;
    key_value.set_sensitive(true);
    upstream_headers.insert("x-api-key", key_value);

    let url = format!(
        "{}/{path}{}",
        proxy.config.anthropic_base,
        uri.query().map(|q| format!("?{q}")).unwrap_or_default()
    );
    let upstream = proxy
        .http
        .post(url)
        .headers(upstream_headers)
        .body(reqwest::Body::wrap_stream(forward::capped(body)))
        .send()
        .await
        .map_err(|error| ProxyError::Upstream(error.without_url().to_string()))?;
    Ok(forward::relay(upstream))
}
