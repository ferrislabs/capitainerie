use capitainerie_common::CoreError;
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{Organization, domain::organization::service::OrganizationService};

impl CapitainerieUseCase {
    #[transactional(organization)]
    pub async fn organization_by_issuer(
        &self,
        issuer_url: &str,
    ) -> Result<Organization, CoreError> {
        OrganizationService::new(organization_repository)
            .by_issuer(issuer_url)
            .await
    }
}
