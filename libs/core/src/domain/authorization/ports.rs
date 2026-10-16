use capitainerie_common::CoreError;

use super::{AccessRequest, Decision};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait Authorizer: Send + Sync {
    fn evaluate(
        &self,
        request: &AccessRequest,
    ) -> impl Future<Output = Result<Decision, CoreError>> + Send;
}
