use aws_lc_rs::digest;
use axum::http::{HeaderMap, header::AUTHORIZATION};
use base64::{Engine, engine::general_purpose::STANDARD};

pub const GIT_USER: &str = "x-access-token";

pub fn sha256(token: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(digest::digest(&digest::SHA256, token.as_bytes()).as_ref());
    out
}

pub fn api_token(headers: &HeaderMap) -> Option<&str> {
    if let Some(key) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return Some(key);
    }
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

pub fn basic_token(headers: &HeaderMap) -> Option<String> {
    let encoded = headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Basic ")?;
    let decoded = String::from_utf8(STANDARD.decode(encoded.trim()).ok()?).ok()?;
    let (user, password) = decoded.split_once(':')?;
    (user == GIT_USER).then(|| password.to_owned())
}

pub fn basic_header(password: &str) -> String {
    format!(
        "Basic {}",
        STANDARD.encode(format!("{GIT_USER}:{password}"))
    )
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn the_hash_matches_the_known_sha256_vector() {
        let hash = sha256("abc");
        assert_eq!(
            hash[..4],
            [0xba, 0x78, 0x16, 0xbf],
            "SHA-256(abc) starts with ba7816bf"
        );
    }

    #[test]
    fn basic_needs_the_access_token_user() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&basic_header("secret")).unwrap(),
        );
        assert_eq!(basic_token(&headers).as_deref(), Some("secret"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", STANDARD.encode("someone:secret"))).unwrap(),
        );
        assert_eq!(basic_token(&headers), None);
    }

    #[test]
    fn api_token_prefers_x_api_key_then_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer b"));
        assert_eq!(api_token(&headers), Some("b"));
        headers.insert("x-api-key", HeaderValue::from_static("k"));
        assert_eq!(api_token(&headers), Some("k"));
    }
}
