#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("malformed token: {0}")]
    Malformed(String),
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("token expired")]
    Expired,
    #[error("untrusted client: {0}")]
    UntrustedClient(String),
    #[error("signing key not found: {0}")]
    KeyNotFound(String),
    #[error("issuer discovery failed: {0}")]
    Discovery(String),
}
