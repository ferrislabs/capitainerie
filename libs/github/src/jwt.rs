use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::GithubError;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AppClaims {
    pub iat: i64,
    pub exp: i64,
    pub iss: u64,
}

impl AppClaims {
    /// `iat` is backdated to absorb clock drift; GitHub caps `exp` at 10 minutes.
    pub(crate) fn at(app_id: u64, now: DateTime<Utc>) -> Self {
        Self {
            iat: (now - Duration::seconds(60)).timestamp(),
            exp: (now + Duration::minutes(9)).timestamp(),
            iss: app_id,
        }
    }
}

pub(crate) fn sign(
    app_id: u64,
    private_key_pem: &str,
    now: DateTime<Utc>,
) -> Result<String, GithubError> {
    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|error| GithubError::Jwt(error.to_string()))?;
    jsonwebtoken::encode(
        &Header::new(Algorithm::RS256),
        &AppClaims::at(app_id, now),
        &key,
    )
    .map_err(|error| GithubError::Jwt(error.to_string()))
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{DecodingKey, Validation};

    use super::*;

    const PRIVATE: &str = include_str!("../tests/fixtures/test-only-app-key.pem");
    const PUBLIC: &str = include_str!("../tests/fixtures/test-only-app-key.pub.pem");

    #[test]
    fn claims_are_backdated_and_short_lived() {
        let now = Utc::now();
        let claims = AppClaims::at(42, now);
        assert_eq!(claims.iat, now.timestamp() - 60);
        assert_eq!(claims.exp, now.timestamp() + 540);
        assert_eq!(claims.iss, 42);
    }

    #[test]
    fn token_verifies_with_the_public_key() {
        let token = sign(7, PRIVATE, Utc::now()).unwrap();
        let mut validation = Validation::new(Algorithm::RS256);
        validation.required_spec_claims.clear();
        validation.validate_aud = false;
        let decoded = jsonwebtoken::decode::<AppClaims>(
            &token,
            &DecodingKey::from_rsa_pem(PUBLIC.as_bytes()).unwrap(),
            &validation,
        )
        .unwrap();
        assert_eq!(decoded.claims.iss, 7);
    }

    #[test]
    fn invalid_pem_is_an_error() {
        assert!(matches!(
            sign(1, "not a key", Utc::now()),
            Err(GithubError::Jwt(_))
        ));
    }
}
