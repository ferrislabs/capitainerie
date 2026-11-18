use axum::http::{HeaderMap, header::HOST};

/// Where the hooks are reachable from outside, e.g. `https://api.example.com`.
/// `None` derives it from the request's `X-Forwarded-*` or `Host` headers.
#[derive(Debug, Clone, Default)]
pub struct PublicBaseUrl(pub Option<String>);

impl PublicBaseUrl {
    /// The base, without a trailing slash; empty when nothing tells it.
    pub fn resolve(&self, headers: &HeaderMap) -> String {
        if let Some(configured) = &self.0 {
            return configured.trim_end_matches('/').to_owned();
        }
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        };
        let Some(host) = header("x-forwarded-host").or_else(|| header(HOST.as_str())) else {
            return String::new();
        };
        let scheme = header("x-forwarded-proto").unwrap_or("http");
        format!("{scheme}://{host}")
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn the_base_is_the_configured_url_else_the_forwarded_or_host_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("api.internal:8080"));
        let configured = PublicBaseUrl(Some("https://api.example.com/".into()));
        assert_eq!(configured.resolve(&headers), "https://api.example.com");
        let derived = PublicBaseUrl(None);
        assert_eq!(derived.resolve(&headers), "http://api.internal:8080");
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("api.example.com"),
        );
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https, http"));
        assert_eq!(derived.resolve(&headers), "https://api.example.com");
        assert_eq!(derived.resolve(&HeaderMap::new()), "");
    }
}
