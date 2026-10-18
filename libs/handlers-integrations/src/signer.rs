use aws_lc_rs::{
    hkdf::{HKDF_SHA256, Salt},
    hmac,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use capitainerie_common::OrganizationId;
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

const INFO: &[u8] = b"capitainerie github install state";
const VALIDITY_MINUTES: i64 = 10;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct StateKeyError(&'static str);

#[derive(Debug, Error)]
#[error("the installation state is invalid or expired")]
pub struct InvalidState;

/// Signs the `state` of the GitHub installation round trip, so the callback
/// can trust which organization and which admin started it.
#[derive(Clone)]
pub struct StateSigner {
    key: hmac::Key,
}

impl StateSigner {
    pub fn new(secret: &[u8]) -> Result<Self, StateKeyError> {
        let prk = Salt::new(HKDF_SHA256, &[]).extract(secret);
        let okm = prk
            .expand(&[INFO], hmac::HMAC_SHA256)
            .map_err(|_| StateKeyError("the state key cannot be derived"))?;
        Ok(Self {
            key: hmac::Key::from(okm),
        })
    }

    /// `SECRETS_KEY`: base64 of 32 bytes.
    pub fn from_base64(encoded: &str) -> Result<Self, StateKeyError> {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|_| StateKeyError("the secrets key is not valid base64"))?;
        if decoded.len() != 32 {
            return Err(StateKeyError("the secrets key must decode to 32 bytes"));
        }
        Self::new(&decoded)
    }

    pub fn sign(
        &self,
        organization_id: OrganizationId,
        subject: &str,
        now: DateTime<Utc>,
    ) -> String {
        let expiry = (now + Duration::minutes(VALIDITY_MINUTES)).timestamp();
        let payload = format!("{}|{subject}|{expiry}", organization_id.0);
        let tag = hmac::sign(&self.key, payload.as_bytes());
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(tag.as_ref())
        )
    }

    pub fn verify(
        &self,
        state: &str,
        organization_id: OrganizationId,
        subject: &str,
        now: DateTime<Utc>,
    ) -> Result<(), InvalidState> {
        let (payload, tag) = state.split_once('.').ok_or(InvalidState)?;
        let payload = URL_SAFE_NO_PAD.decode(payload).map_err(|_| InvalidState)?;
        let tag = URL_SAFE_NO_PAD.decode(tag).map_err(|_| InvalidState)?;
        hmac::verify(&self.key, &payload, &tag).map_err(|_| InvalidState)?;
        let payload = String::from_utf8(payload).map_err(|_| InvalidState)?;
        let expiry: i64 = payload
            .strip_prefix(&format!("{}|{subject}|", organization_id.0))
            .and_then(|expiry| expiry.parse().ok())
            .ok_or(InvalidState)?;
        if now.timestamp() >= expiry {
            return Err(InvalidState);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer(byte: u8) -> StateSigner {
        StateSigner::new(&[byte; 32]).unwrap()
    }

    #[test]
    fn a_fresh_state_verifies_for_its_organization_and_subject() {
        let (organization, now) = (OrganizationId::new(), Utc::now());
        let state = signer(1).sign(organization, "user-1", now);
        assert!(
            signer(1)
                .verify(&state, organization, "user-1", now)
                .is_ok()
        );
        assert!(!state.contains(['+', '/', '=']));
    }

    #[test]
    fn another_organization_subject_or_key_is_refused() {
        let (organization, now) = (OrganizationId::new(), Utc::now());
        let state = signer(1).sign(organization, "user-1", now);
        assert!(
            signer(1)
                .verify(&state, OrganizationId::new(), "user-1", now)
                .is_err()
        );
        assert!(
            signer(1)
                .verify(&state, organization, "user-2", now)
                .is_err()
        );
        assert!(
            signer(2)
                .verify(&state, organization, "user-1", now)
                .is_err()
        );
    }

    #[test]
    fn a_state_expires_after_ten_minutes() {
        let (organization, now) = (OrganizationId::new(), Utc::now());
        let state = signer(1).sign(organization, "user-1", now);
        let almost = now + Duration::minutes(10) - Duration::seconds(1);
        assert!(
            signer(1)
                .verify(&state, organization, "user-1", almost)
                .is_ok()
        );
        let later = now + Duration::minutes(10);
        assert!(
            signer(1)
                .verify(&state, organization, "user-1", later)
                .is_err()
        );
    }

    #[test]
    fn a_tampered_or_malformed_state_is_refused() {
        let (organization, now) = (OrganizationId::new(), Utc::now());
        let state = signer(1).sign(organization, "user-1", now);
        let (payload, tag) = state.split_once('.').unwrap();
        let forged = URL_SAFE_NO_PAD.encode(format!(
            "{}|user-1|{}",
            organization.0,
            now.timestamp() + 99_999
        ));
        for bad in [
            format!("{forged}.{tag}"),
            format!("{payload}.{}", URL_SAFE_NO_PAD.encode([0u8; 32])),
            payload.to_owned(),
            String::new(),
            "!!.!!".to_owned(),
        ] {
            assert!(
                signer(1).verify(&bad, organization, "user-1", now).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_secrets_key_must_be_32_base64_bytes() {
        let ok = base64::engine::general_purpose::STANDARD.encode([7u8; 32]);
        assert!(StateSigner::from_base64(&ok).is_ok());
        assert!(StateSigner::from_base64("AAAA").is_err());
        assert!(StateSigner::from_base64("not base64!").is_err());
    }
}
