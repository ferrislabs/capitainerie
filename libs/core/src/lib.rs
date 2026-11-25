extern crate self as capitainerie_core;

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    AgentDelegation, AgentTaskConfig, AgentTaskPipeline, AgentTaskSource, CapitainerieUseCase,
    ExecuteConfig, ExecutePipeline, ExecutionContext, Executor, IdentitySweep, InterventionConfig,
    InterventionPipeline, MeetingPipeline, PipelineConfig, RunCardLinks, RunContext, RunDelegation,
    RunTimeline, StartedAgentRun, StartedExecution, StartedIntervention, SubmittedTranscript,
};
pub use domain::organization::{Organization, OrganizationSlug};
