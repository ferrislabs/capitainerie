use std::io;

use axum::{
    body::{Body, Bytes},
    http::{HeaderMap, HeaderName, header},
    response::Response,
};
use futures_util::{Stream, StreamExt};

use capitainerie_common::RunId;

use crate::{error::ProxyError, proxy::Permit};

pub const MAX_BODY: u64 = 512 * 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = 10 * 1024 * 1024;
/// Coding agents ask for long outputs (64k and more on current models); the
/// run stays bounded by its turns and its deadline.
pub const MAX_OUTPUT_TOKENS_CAP: u64 = 128_000;

const HOP_BY_HOP: [HeaderName; 7] = [
    header::CONNECTION,
    HeaderName::from_static("keep-alive"),
    header::PROXY_AUTHENTICATE,
    header::PROXY_AUTHORIZATION,
    header::TE,
    header::TRAILER,
    header::UPGRADE,
];

fn hop_by_hop(name: &HeaderName) -> bool {
    name == header::TRANSFER_ENCODING || HOP_BY_HOP.contains(name)
}

pub fn declared_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

pub async fn read_request(
    headers: &HeaderMap,
    body: Body,
    run_id: RunId,
) -> Result<Bytes, ProxyError> {
    if declared_length(headers).is_some_and(|length| length > MAX_REQUEST_BYTES as u64) {
        return Err(ProxyError::TooLarge { run_id });
    }
    axum::body::to_bytes(body, MAX_REQUEST_BYTES)
        .await
        .map_err(|_| ProxyError::TooLarge { run_id })
}

pub fn copy_allowed(from: &HeaderMap, names: &[HeaderName]) -> HeaderMap {
    let mut out = HeaderMap::new();
    for name in names {
        for value in from.get_all(name) {
            out.append(name.clone(), value.clone());
        }
    }
    out
}

pub fn capped(body: Body) -> impl Stream<Item = Result<Bytes, io::Error>> + Send + 'static {
    let mut seen = 0u64;
    body.into_data_stream().map(move |chunk| {
        let chunk = chunk.map_err(io::Error::other)?;
        seen += chunk.len() as u64;
        if seen > MAX_BODY {
            return Err(io::Error::other("request body too large"));
        }
        Ok(chunk)
    })
}

pub fn relay(upstream: reqwest::Response, permit: Permit) -> Response {
    let mut headers = HeaderMap::new();
    for (name, value) in upstream.headers() {
        if !hop_by_hop(name) {
            headers.append(name.clone(), value.clone());
        }
    }
    let status = upstream.status();
    let held = upstream.bytes_stream().map(move |chunk| {
        let _ = &permit;
        chunk
    });
    let mut response = Response::new(Body::from_stream(held));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}
