use std::error::Error as _;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use capitainerie_common::CoreError;
use capitainerie_core::domain::integration::{BaseUrl, ConnectorFailure};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::HeaderValue;
use reqwest::{Client, RequestBuilder, StatusCode, Url, redirect};
use serde_json::Value;

use crate::NetworkPolicy;

const TOTAL_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;
const MAX_MESSAGE_CHARS: usize = 300;
const MAX_VERSION_CHARS: usize = 64;

#[derive(Debug, thiserror::Error)]
#[error("the network policy blocks {0}")]
struct BlockedHost(String);

pub(crate) fn is_refused(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_refused_v4(address),
        IpAddr::V6(address) => is_refused_v6(address),
    }
}

fn is_refused_v4(address: Ipv4Addr) -> bool {
    let [first, second, third, _] = address.octets();
    address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_multicast()
        || first == 0
        || first >= 240
        || (first == 100 && second & 0xc0 == 64)
        || (first == 192 && second == 0 && third == 0)
        || (first == 198 && second & 0xfe == 18)
}

fn is_refused_v6(address: Ipv6Addr) -> bool {
    if address.is_loopback() || address.is_unspecified() || address.is_multicast() {
        return true;
    }
    let segments = address.segments();
    let first = segments[0];
    if first & 0xfe00 == 0xfc00 || first & 0xffc0 == 0xfe80 {
        return true;
    }
    if first == 0x2002 {
        return is_refused_v4(embedded_v4(segments[1], segments[2]));
    }
    if segments[..6] == [0x0064, 0xff9b, 0, 0, 0, 0] {
        return is_refused_v4(embedded_v4(segments[6], segments[7]));
    }
    address.to_ipv4().is_some_and(is_refused_v4)
}

fn embedded_v4(high: u16, low: u16) -> Ipv4Addr {
    let [a, b] = high.to_be_bytes();
    let [c, d] = low.to_be_bytes();
    Ipv4Addr::new(a, b, c, d)
}

impl NetworkPolicy {
    pub(crate) fn refuses(self, address: IpAddr) -> bool {
        !self.allow_private_networks && is_refused(address)
    }
}

enum Resolved {
    Allowed(Vec<SocketAddr>),
    Blocked,
}

async fn resolve_host(
    policy: NetworkPolicy,
    host: &str,
    port: u16,
) -> Result<Resolved, std::io::Error> {
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port)).await?.collect();
    if addresses.is_empty() {
        return Err(std::io::Error::other("the name has no address"));
    }
    let allowed: Vec<SocketAddr> = addresses
        .into_iter()
        .filter(|address| !policy.refuses(address.ip()))
        .collect();
    if allowed.is_empty() {
        Ok(Resolved::Blocked)
    } else {
        Ok(Resolved::Allowed(allowed))
    }
}

struct GuardedResolver {
    policy: NetworkPolicy,
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let policy = self.policy;
        Box::pin(async move {
            match resolve_host(policy, name.as_str(), 0).await? {
                Resolved::Allowed(addresses) => Ok(Box::new(addresses.into_iter()) as Addrs),
                Resolved::Blocked => Err(Box::new(BlockedHost(name.as_str().to_owned()))
                    as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }
}

pub(crate) fn build_client(policy: NetworkPolicy) -> Result<Client, CoreError> {
    Client::builder()
        .redirect(redirect::Policy::none())
        .timeout(TOTAL_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .no_proxy()
        .dns_resolver(Arc::new(GuardedResolver { policy }))
        .build()
        .map_err(|error| {
            CoreError::Internal(format!(
                "cannot build the HTTP client: {}",
                error.without_url()
            ))
        })
}

pub(crate) async fn check_target(
    policy: NetworkPolicy,
    base_url: &BaseUrl,
) -> Result<(), ConnectorFailure> {
    if !base_url.is_https() && !policy.allow_insecure_http {
        return Err(ConnectorFailure::Blocked(format!(
            "{} (plain http is refused)",
            base_url.host()
        )));
    }
    let url = Url::parse(base_url.as_str())
        .map_err(|_| ConnectorFailure::Protocol("the instance URL is invalid".into()))?;
    let host = url
        .host_str()
        .ok_or_else(|| ConnectorFailure::Protocol("the instance URL has no host".into()))?;
    let literal = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(address) = literal.parse::<IpAddr>() {
        return if policy.refuses(address) {
            Err(ConnectorFailure::Blocked(address.to_string()))
        } else {
            Ok(())
        };
    }
    let port = url.port_or_known_default().unwrap_or(0);
    match resolve_host(policy, host, port).await {
        Ok(Resolved::Allowed(_)) => Ok(()),
        Ok(Resolved::Blocked) => Err(ConnectorFailure::Blocked(host.to_owned())),
        Err(error) => Err(ConnectorFailure::Unreachable(format!(
            "cannot resolve {host}: {error}"
        ))),
    }
}

pub(crate) struct Answer {
    pub(crate) status: StatusCode,
    pub(crate) body: Vec<u8>,
}

impl Answer {
    pub(crate) fn json(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }
}

fn too_large() -> ConnectorFailure {
    ConnectorFailure::Protocol("the answer exceeds 10 MiB".into())
}

pub(crate) async fn send(request: RequestBuilder) -> Result<Answer, ConnectorFailure> {
    let mut response = request.send().await.map_err(map_error)?;
    let status = response.status();
    if status.is_redirection() {
        return Err(ConnectorFailure::Protocol(
            "redirects are not followed".into(),
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BODY_BYTES as u64)
    {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_error)? {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Answer { status, body })
}

fn map_error(error: reqwest::Error) -> ConnectorFailure {
    if let Some(blocked) = blocked_in_chain(&error) {
        return ConnectorFailure::Blocked(blocked.0.clone());
    }
    if error.is_timeout() {
        return ConnectorFailure::Timeout;
    }
    let error = error.without_url();
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    ConnectorFailure::Unreachable(message)
}

fn blocked_in_chain(error: &reqwest::Error) -> Option<&BlockedHost> {
    let mut source = error.source();
    while let Some(cause) = source {
        if let Some(blocked) = cause.downcast_ref::<BlockedHost>() {
            return Some(blocked);
        }
        source = cause.source();
    }
    None
}

pub(crate) fn secret_header(prefix: &str, secret: &str) -> Result<HeaderValue, ConnectorFailure> {
    let mut value = HeaderValue::from_str(&format!("{prefix}{secret}")).map_err(|_| {
        ConnectorFailure::Protocol("the secret cannot be sent as an HTTP header".into())
    })?;
    value.set_sensitive(true);
    Ok(value)
}

pub(crate) fn scrub(message: &str, secret: &str) -> String {
    let redacted = if secret.is_empty() {
        message.to_owned()
    } else {
        message.replace(secret, "***")
    };
    redacted.chars().take(MAX_MESSAGE_CHARS).collect()
}

pub(crate) fn clean_version(version: &str) -> String {
    version.chars().take(MAX_VERSION_CHARS).collect()
}

pub(crate) fn remote_failure(answer: &Answer, secret: &str) -> ConnectorFailure {
    let message = answer
        .json()
        .and_then(|body| {
            body.get("message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| answer.status.canonical_reason().map(str::to_owned))
        .unwrap_or_else(|| "no details".to_owned());
    ConnectorFailure::Remote {
        status: answer.status.as_u16(),
        message: scrub(&message, secret),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(address: &str) -> bool {
        is_refused(address.parse().unwrap())
    }

    #[test]
    fn private_and_special_ipv4_addresses_are_refused() {
        for address in [
            "127.0.0.1",
            "127.255.255.254",
            "10.0.0.5",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "100.127.255.255",
            "0.0.0.0",
            "0.1.2.3",
            "192.0.0.8",
            "198.18.0.1",
            "198.19.255.255",
            "255.255.255.255",
            "224.0.0.1",
            "239.255.255.250",
            "240.0.0.1",
        ] {
            assert!(refused(address), "{address} should be refused");
        }
    }

    #[test]
    fn public_ipv4_addresses_are_allowed() {
        for address in [
            "1.1.1.1",
            "8.8.8.8",
            "172.15.0.1",
            "172.32.0.1",
            "100.63.255.255",
            "100.128.0.1",
            "192.0.2.1",
            "198.17.0.1",
            "198.20.0.1",
            "93.184.216.34",
        ] {
            assert!(!refused(address), "{address} should be allowed");
        }
    }

    #[test]
    fn private_and_special_ipv6_addresses_are_refused() {
        for address in [
            "::1",
            "::",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "febf::1",
            "ff02::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.5",
            "::ffff:192.168.0.1",
            "::ffff:169.254.169.254",
            "::127.0.0.1",
            "::10.0.0.1",
            "64:ff9b::a00:5",
            "64:ff9b::7f00:1",
            "2002:a00:5::1",
            "2002:c0a8:101::1",
        ] {
            assert!(refused(address), "{address} should be refused");
        }
    }

    #[test]
    fn public_ipv6_addresses_are_allowed() {
        for address in [
            "2606:4700::1111",
            "2001:4860:4860::8888",
            "::ffff:1.1.1.1",
            "64:ff9b::101:101",
            "2002:101:101::1",
        ] {
            assert!(!refused(address), "{address} should be allowed");
        }
    }

    #[test]
    fn the_policy_lifts_the_address_refusal_only_when_asked() {
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();
        let public: IpAddr = "1.1.1.1".parse().unwrap();
        assert!(NetworkPolicy::default().refuses(loopback));
        assert!(!NetworkPolicy::default().refuses(public));
        let open = NetworkPolicy {
            allow_insecure_http: false,
            allow_private_networks: true,
        };
        assert!(!open.refuses(loopback));
    }

    #[test]
    fn a_message_is_stripped_of_the_secret_and_truncated() {
        assert_eq!(scrub("bad key s3cret!", "s3cret"), "bad key ***!");
        assert_eq!(scrub("kept", ""), "kept");
        assert_eq!(scrub(&"é".repeat(400), "s3cret").chars().count(), 300);
    }

    #[test]
    fn a_secret_that_is_not_a_header_value_is_refused_without_echoing_it() {
        let failure = secret_header("bearer ", "line\nbreak").unwrap_err();
        assert!(matches!(failure, ConnectorFailure::Protocol(_)));
        assert!(!failure.to_string().contains("line"));
        assert!(secret_header("bearer ", "plain").unwrap().is_sensitive());
    }

    #[tokio::test]
    async fn the_resolver_refuses_a_name_that_only_resolves_to_a_refused_address() {
        let policy = NetworkPolicy::default();
        assert!(matches!(
            resolve_host(policy, "localhost", 0).await.unwrap(),
            Resolved::Blocked
        ));
        let open = NetworkPolicy {
            allow_insecure_http: false,
            allow_private_networks: true,
        };
        assert!(matches!(
            resolve_host(open, "localhost", 0).await.unwrap(),
            Resolved::Allowed(addresses) if !addresses.is_empty()
        ));
    }

    #[tokio::test]
    async fn the_client_resolver_blocks_a_refused_name_before_connecting() {
        let client = build_client(NetworkPolicy::default()).unwrap();
        let failure = send(client.get("http://localhost:1/")).await.err().unwrap();
        assert_eq!(failure, ConnectorFailure::Blocked("localhost".into()));
    }
}
