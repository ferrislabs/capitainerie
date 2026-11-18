use aws_lc_rs::{
    digest::{SHA256, digest},
    rand::{SecureRandom, SystemRandom},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use capitainerie_common::CoreError;

const VERIFIER_BYTES: usize = 32;
const STATE_BYTES: usize = 32;

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn generate() -> Result<Self, CoreError> {
        let verifier = random_token(VERIFIER_BYTES)?;
        let challenge = challenge_of(&verifier);
        Ok(Self {
            verifier,
            challenge,
        })
    }
}

pub fn challenge_of(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(digest(&SHA256, verifier.as_bytes()))
}

pub fn random_state() -> Result<String, CoreError> {
    random_token(STATE_BYTES)
}

fn random_token(length: usize) -> Result<String, CoreError> {
    let mut bytes = vec![0u8; length];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| CoreError::Internal("the system random source failed".into()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_matches_the_rfc_7636_example() {
        assert_eq!(
            challenge_of("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn a_verifier_is_long_url_safe_and_never_repeats() {
        let first = Pkce::generate().unwrap();
        let second = Pkce::generate().unwrap();
        assert_eq!(first.verifier.len(), 43);
        assert!(
            first
                .verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert_ne!(first.verifier, second.verifier);
        assert_eq!(first.challenge, challenge_of(&first.verifier));
    }

    #[test]
    fn states_are_unpredictable() {
        assert_ne!(random_state().unwrap(), random_state().unwrap());
        assert_eq!(random_state().unwrap().len(), 43);
    }
}
