extern crate self as capitainerie_core;

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    AgentDelegation, CapitainerieUseCase, MeetingPipeline, PipelineConfig, RunContext, RunTimeline,
    StartedExecution, SubmittedTranscript,
};
pub use domain::organization::{Organization, OrganizationSlug};
