use capitainerie_common::CoreError;

use super::{Extraction, WriteReport, WriteRequest};

/// Runs the agents of the meeting pipeline. Implementations must not leak
/// their framework types through this port.
#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait AgentRuntime: Send + Sync {
    /// The extractor: transcript in, typed action items out. No tools.
    fn extract(
        &self,
        transcript: &str,
    ) -> impl Future<Output = Result<Extraction, CoreError>> + Send;

    /// The writer: turns located actions into proposed cards, through the
    /// Kanban MCP server only.
    fn write(
        &self,
        request: WriteRequest,
    ) -> impl Future<Output = Result<WriteReport, CoreError>> + Send;
}
