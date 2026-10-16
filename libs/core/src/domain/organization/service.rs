use capitainerie_common::CoreError;

use super::{Organization, ports::OrganizationRepository};

pub struct OrganizationService<R> {
    organizations: R,
}

impl<R: OrganizationRepository> OrganizationService<R> {
    pub fn new(organizations: R) -> Self {
        Self { organizations }
    }

    /// The organization whose realm issued a token. An unknown issuer is
    /// `NotFound`: tokens from realms no organization owns are never trusted.
    pub async fn by_issuer(mut self, issuer_url: &str) -> Result<Organization, CoreError> {
        self.organizations
            .find_by_issuer(issuer_url)
            .await?
            .ok_or(CoreError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;
    use chrono::Utc;

    use super::*;
    use crate::{OrganizationSlug, domain::organization::ports::MockOrganizationRepository};

    fn organization() -> Organization {
        Organization {
            id: OrganizationId::new(),
            slug: OrganizationSlug::try_from("demo".to_string()).unwrap(),
            name: "Demo".into(),
            issuer_url: "http://idp/realms/demo".into(),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn returns_the_organization_owning_the_issuer() {
        let expected = organization();
        let found = expected.clone();
        let mut repository = MockOrganizationRepository::new();
        repository
            .expect_find_by_issuer()
            .withf(|issuer| issuer == "http://idp/realms/demo")
            .returning(move |_| {
                let found = found.clone();
                Box::pin(async move { Ok(Some(found)) })
            });

        let organization = OrganizationService::new(repository)
            .by_issuer("http://idp/realms/demo")
            .await
            .unwrap();

        assert_eq!(organization, expected);
    }

    #[tokio::test]
    async fn an_unknown_issuer_is_not_found() {
        let mut repository = MockOrganizationRepository::new();
        repository
            .expect_find_by_issuer()
            .returning(|_| Box::pin(async { Ok(None) }));

        let error = OrganizationService::new(repository)
            .by_issuer("http://evil/realms/x")
            .await
            .unwrap_err();

        assert!(matches!(error, CoreError::NotFound));
    }
}
