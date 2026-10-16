use aws_lc_rs::{
    aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey},
    rand::{SecureRandom, SystemRandom},
};
use capitainerie_common::CoreError;

/// AES-256-GCM: the tag makes a tampered or swapped ciphertext fail instead of
/// decrypting to garbage. The associated data binds a ciphertext to the row it
/// belongs to.
pub struct SecretCipher {
    key: LessSafeKey,
    rng: SystemRandom,
}

pub struct Sealed {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl SecretCipher {
    pub fn new(key: &[u8; 32]) -> Result<Self, CoreError> {
        let unbound = UnboundKey::new(&AES_256_GCM, key)
            .map_err(|_| CoreError::Internal("the delegation key is unusable".into()))?;
        Ok(Self {
            key: LessSafeKey::new(unbound),
            rng: SystemRandom::new(),
        })
    }

    /// A key of the wrong length is refused, never padded or truncated.
    pub fn from_base64(encoded: &str) -> Result<Self, CoreError> {
        use base64::{Engine, engine::general_purpose::STANDARD};

        let decoded = STANDARD
            .decode(encoded.trim())
            .map_err(|_| CoreError::Internal("DELEGATION_KEY is not valid base64".into()))?;
        let key: [u8; 32] = decoded.as_slice().try_into().map_err(|_| {
            CoreError::Internal(format!(
                "DELEGATION_KEY must decode to 32 bytes, got {}",
                decoded.len()
            ))
        })?;
        Self::new(&key)
    }

    pub fn seal(&self, plaintext: &[u8], associated: &[u8]) -> Result<Sealed, CoreError> {
        let mut nonce = [0u8; NONCE_LEN];
        self.rng
            .fill(&mut nonce)
            .map_err(|_| CoreError::Internal("the system random source failed".into()))?;
        let mut in_out = plaintext.to_vec();
        self.key
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(associated),
                &mut in_out,
            )
            .map_err(|_| CoreError::Internal("sealing a delegation failed".into()))?;
        Ok(Sealed {
            nonce: nonce.to_vec(),
            ciphertext: in_out,
        })
    }

    pub fn open(&self, sealed: &Sealed, associated: &[u8]) -> Result<Vec<u8>, CoreError> {
        let nonce: [u8; NONCE_LEN] = sealed
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| CoreError::Internal("a stored nonce has the wrong length".into()))?;
        let mut in_out = sealed.ciphertext.clone();
        let plaintext = self
            .key
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(associated),
                &mut in_out,
            )
            .map_err(|_| CoreError::Internal("a stored delegation failed authentication".into()))?;
        Ok(plaintext.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> SecretCipher {
        SecretCipher::new(&[7u8; 32]).unwrap()
    }

    #[test]
    fn sealed_then_opened_with_the_same_binding() {
        let sealed = cipher().seal(b"token", b"run-1/agent").unwrap();
        assert_ne!(sealed.ciphertext, b"token");
        assert_eq!(cipher().open(&sealed, b"run-1/agent").unwrap(), b"token");
    }

    #[test]
    fn a_ciphertext_moved_to_another_row_does_not_open() {
        let sealed = cipher().seal(b"token", b"run-1/agent").unwrap();
        assert!(cipher().open(&sealed, b"run-2/agent").is_err());
    }

    #[test]
    fn a_tampered_ciphertext_does_not_open() {
        let mut sealed = cipher().seal(b"token", b"row").unwrap();
        sealed.ciphertext[0] ^= 1;
        assert!(cipher().open(&sealed, b"row").is_err());
    }

    #[test]
    fn keys_of_the_wrong_size_are_refused() {
        use base64::{Engine, engine::general_purpose::STANDARD};
        assert!(SecretCipher::from_base64(&STANDARD.encode([1u8; 16])).is_err());
        assert!(SecretCipher::from_base64("not base64!").is_err());
        assert!(SecretCipher::from_base64(&STANDARD.encode([1u8; 32])).is_ok());
    }
}
