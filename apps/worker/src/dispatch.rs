use capitainerie_common::{CoreError, RunId};
use capitainerie_core::{
    CapitainerieUseCase, ExecutePipeline, InterventionPipeline, MeetingPipeline,
    domain::{
        agent::ports::AgentRuntime,
        delegation::ports::TokenExchanger,
        execution::ports::{CodeHost, TaskRunner},
        integration::{intervention::IntegratorRuntime, ports::IntegrationConnector},
        similarity::ports::{Embedder, VectorStore},
    },
};
use chrono::{DateTime, Utc};

pub trait RunPipelines: Send + Sync + 'static {
    fn usecase(&self) -> &CapitainerieUseCase;

    fn stale_before(&self) -> DateTime<Utc>;

    /// Card executions are paced apart from the other runs.
    fn is_execution(&self, run_id: RunId) -> impl Future<Output = Result<bool, CoreError>> + Send {
        self.usecase().is_execution_run(run_id)
    }

    fn execute(
        &self,
        run_id: RunId,
        traceparent: Option<&str>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send;
}

impl<R, X, E, V> RunPipelines for MeetingPipeline<R, X, E, V>
where
    R: AgentRuntime + 'static,
    X: TokenExchanger + 'static,
    E: Embedder + 'static,
    V: VectorStore + 'static,
{
    fn usecase(&self) -> &CapitainerieUseCase {
        MeetingPipeline::usecase(self)
    }

    fn stale_before(&self) -> DateTime<Utc> {
        MeetingPipeline::stale_before(self)
    }

    fn execute(
        &self,
        run_id: RunId,
        traceparent: Option<&str>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        MeetingPipeline::execute(self, run_id, traceparent)
    }
}

impl<T, H> RunPipelines for ExecutePipeline<T, H>
where
    T: TaskRunner + 'static,
    H: CodeHost + 'static,
{
    fn usecase(&self) -> &CapitainerieUseCase {
        ExecutePipeline::usecase(self)
    }

    fn stale_before(&self) -> DateTime<Utc> {
        ExecutePipeline::stale_before(self)
    }

    fn execute(
        &self,
        run_id: RunId,
        traceparent: Option<&str>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        ExecutePipeline::execute(self, run_id, traceparent)
    }
}

impl<R, C> RunPipelines for InterventionPipeline<R, C>
where
    R: IntegratorRuntime + 'static,
    C: IntegrationConnector + 'static,
{
    fn usecase(&self) -> &CapitainerieUseCase {
        InterventionPipeline::usecase(self)
    }

    fn stale_before(&self) -> DateTime<Utc> {
        InterventionPipeline::stale_before(self)
    }

    fn execute(
        &self,
        run_id: RunId,
        traceparent: Option<&str>,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        InterventionPipeline::execute(self, run_id, traceparent)
    }
}

pub struct Dispatcher<M, E, I> {
    meeting: M,
    execution: E,
    intervention: I,
}

impl<M, E, I> Dispatcher<M, E, I> {
    pub fn new(meeting: M, execution: E, intervention: I) -> Self {
        Self {
            meeting,
            execution,
            intervention,
        }
    }
}

impl<M: RunPipelines, E: RunPipelines, I: RunPipelines> RunPipelines for Dispatcher<M, E, I> {
    fn usecase(&self) -> &CapitainerieUseCase {
        self.meeting.usecase()
    }

    fn stale_before(&self) -> DateTime<Utc> {
        self.meeting
            .stale_before()
            .min(self.execution.stale_before())
            .min(self.intervention.stale_before())
    }

    async fn execute(&self, run_id: RunId, traceparent: Option<&str>) -> Result<(), CoreError> {
        let usecase = self.meeting.usecase();
        if usecase.is_intervention_run(run_id).await? {
            self.intervention.execute(run_id, traceparent).await
        } else if usecase.is_execution_run(run_id).await? {
            self.execution.execute(run_id, traceparent).await
        } else {
            self.meeting.execute(run_id, traceparent).await
        }
    }
}
