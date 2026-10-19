use capitainerie_common::{ClientId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use super::{Client, ClientListing, ColumnCount, ListFilter};

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait ClientRepository: Send {
    fn insert(&mut self, client: &Client) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn find(
        &mut self,
        organization_id: OrganizationId,
        id: ClientId,
    ) -> impl Future<Output = Result<Option<Client>, CoreError>> + Send;

    fn update(&mut self, client: &Client) -> impl Future<Output = Result<(), CoreError>> + Send;

    fn list(
        &mut self,
        organization_id: OrganizationId,
        filter: &ListFilter,
    ) -> impl Future<Output = Result<Vec<ClientListing>, CoreError>> + Send;

    fn card_counts(
        &mut self,
        organization_id: OrganizationId,
        id: ClientId,
    ) -> impl Future<Output = Result<Vec<ColumnCount>, CoreError>> + Send;

    fn recent(
        &mut self,
        organization_id: OrganizationId,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<(DateTime<Utc>, Client)>, CoreError>> + Send;
}
