use capitainerie_common::CoreError;

use super::Organization;

#[cfg_attr(any(test, feature = "mock"), mockall::automock)]
pub trait OrganizationRepository: Send {
    fn find_by_issuer(
        &mut self,
        issuer_url: &str,
    ) -> impl Future<Output = Result<Option<Organization>, CoreError>> + Send;
}
