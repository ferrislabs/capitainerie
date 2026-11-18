use std::collections::HashMap;

use capitainerie_common::{CoreError, OrganizationId};
use uuid::Uuid;

use super::{AuditEvent, AuditFilter};

/// A crew agent as the audit log names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedAgent {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
}

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait AuditRepository: Send {
    /// Newest first, at most `filter.limit` events.
    fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &AuditFilter,
    ) -> impl Future<Output = Result<Vec<AuditEvent>, CoreError>> + Send;

    /// Names of the members among `subjects`, by subject.
    fn member_names(
        &mut self,
        organization_id: OrganizationId,
        subjects: &[String],
    ) -> impl Future<Output = Result<HashMap<String, String>, CoreError>> + Send;

    fn agents_by_id(
        &mut self,
        organization_id: OrganizationId,
        ids: &[Uuid],
    ) -> impl Future<Output = Result<Vec<NamedAgent>, CoreError>> + Send;

    fn agents_by_slug(
        &mut self,
        organization_id: OrganizationId,
        slugs: &[String],
    ) -> impl Future<Output = Result<Vec<NamedAgent>, CoreError>> + Send;

    /// The agent of each agent task run among `run_ids`, by run.
    fn run_agents(
        &mut self,
        organization_id: OrganizationId,
        run_ids: &[Uuid],
    ) -> impl Future<Output = Result<HashMap<Uuid, NamedAgent>, CoreError>> + Send;

    fn card_titles(
        &mut self,
        organization_id: OrganizationId,
        card_ids: &[Uuid],
    ) -> impl Future<Output = Result<HashMap<Uuid, String>, CoreError>> + Send;
}
