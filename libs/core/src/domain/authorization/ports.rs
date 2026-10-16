use capitainerie_common::{CoreError, OrganizationId, RunId};

use super::{AccessRequest, Decision, DecisionRecord};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait Authorizer: Send + Sync {
    fn evaluate(
        &self,
        request: &AccessRequest,
    ) -> impl Future<Output = Result<Decision, CoreError>> + Send;
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait DecisionLogRepository: Send {
    fn list_for_run(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> impl Future<Output = Result<Vec<DecisionRecord>, CoreError>> + Send;
}
