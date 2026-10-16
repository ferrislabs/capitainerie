use std::future::Future;

use super::errors::AuthError;
use super::identity::Identity;

pub trait TokenVerifier: Send + Sync {
    fn verify(
        &self,
        token: &str,
        issuer: &str,
    ) -> impl Future<Output = Result<Identity, AuthError>> + Send;
}
