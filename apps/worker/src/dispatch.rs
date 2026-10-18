use capitainerie_common::{CoreError, RunId};
use capitainerie_core::{
    CapitainerieUseCase, ExecutePipeline, MeetingPipeline,
    domain::{
        agent::ports::AgentRuntime,
        delegation::ports::TokenExchanger,
        execution::ports::{CodeHost, TaskRunner},
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

pub struct Dispatcher<M, E> {
    meeting: M,
    execution: E,
}

impl<M, E> Dispatcher<M, E> {
    pub fn new(meeting: M, execution: E) -> Self {
        Self { meeting, execution }
    }
}

impl<M: RunPipelines, E: RunPipelines> RunPipelines for Dispatcher<M, E> {
    fn usecase(&self) -> &CapitainerieUseCase {
        self.meeting.usecase()
    }

    fn stale_before(&self) -> DateTime<Utc> {
        self.meeting
            .stale_before()
            .min(self.execution.stale_before())
    }

    async fn execute(&self, run_id: RunId, traceparent: Option<&str>) -> Result<(), CoreError> {
        if self.meeting.usecase().is_execution_run(run_id).await? {
            self.execution.execute(run_id, traceparent).await
        } else {
            self.meeting.execute(run_id, traceparent).await
        }
    }
}
