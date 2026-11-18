use capitainerie_common::{CoreError, OrganizationId, RunId};

use super::AgentTask;
use crate::domain::integration::intervention::{
    IntegratorReport, IntegratorRequest, IntegratorTools,
};
use crate::domain::provider::ModelRef;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait AgentTaskRepository: Send {
    fn insert(&mut self, task: &AgentTask) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
    ) -> impl Future<Output = Result<Option<AgentTask>, CoreError>> + Send;

    /// Keeps the first model a run resolved to and returns what is stored.
    fn freeze_model(
        &mut self,
        organization_id: OrganizationId,
        run_id: RunId,
        model: &ModelRef,
    ) -> impl Future<Output = Result<ModelRef, CoreError>> + Send;
}

pub trait TaskRuntime: Send + Sync {
    fn run_task<T: IntegratorTools>(
        &self,
        system: &str,
        request: &IntegratorRequest,
        tools: &T,
    ) -> impl Future<Output = Result<IntegratorReport, CoreError>> + Send;
}
