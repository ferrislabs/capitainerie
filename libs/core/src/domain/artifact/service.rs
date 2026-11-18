use capitainerie_common::{ArtifactId, CoreError};

use super::{Artifact, ArtifactFilter, ArtifactView, ports::ArtifactRepository};
use crate::domain::authorization::{
    AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPage {
    pub items: Vec<ArtifactView>,
    pub next_cursor: Option<ArtifactId>,
}

pub struct ArtifactService<'a, R, A> {
    artifacts: R,
    authorizer: &'a A,
}

impl<'a, R: ArtifactRepository, A: Authorizer> ArtifactService<'a, R, A> {
    pub fn new(artifacts: R, authorizer: &'a A) -> Self {
        Self {
            artifacts,
            authorizer,
        }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        mut filter: ArtifactFilter,
    ) -> Result<ArtifactPage, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, None).await?;
        let limit = filter.limit as usize;
        filter.limit += 1;
        let mut items = self.artifacts.list(organization_id, &filter).await?;
        let next_cursor = if items.len() > limit {
            items.truncate(limit);
            items.last().map(|view| view.artifact.id)
        } else {
            None
        };
        Ok(ArtifactPage { items, next_cursor })
    }

    pub async fn get(
        mut self,
        subject: Subject,
        id: ArtifactId,
    ) -> Result<ArtifactView, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Some(id)).await?;
        self.artifacts
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)
    }

    /// Files an artifact the system produced: no policy applies. False when
    /// the run already filed one.
    pub async fn publish(mut self, artifact: &Artifact) -> Result<bool, CoreError> {
        self.artifacts.insert(artifact).await
    }

    async fn require(&self, subject: Subject, id: Option<ArtifactId>) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(
                subject,
                Action::ArtifactRead,
                ResourceKind::Artifact,
                id.map(|id| id.0),
            ),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;

    use super::*;
    use crate::domain::{
        artifact::ports::MockArtifactRepository,
        authorization::{Decision, ports::MockAuthorizer},
    };

    fn subject() -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id: OrganizationId::new(),
            actor: Some("agent:iris".into()),
            is_admin: false,
        }
    }

    #[tokio::test]
    async fn a_denied_caller_reads_no_artifact() {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "agents may not".into(),
                })
            })
        });
        let mut repository = MockArtifactRepository::new();
        repository.expect_find().never();
        let error = ArtifactService::new(repository, &authorizer)
            .get(subject(), ArtifactId::new())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::Forbidden { .. }));
    }

    #[tokio::test]
    async fn a_missing_artifact_is_not_found() {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        let mut repository = MockArtifactRepository::new();
        repository
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        let error = ArtifactService::new(repository, &authorizer)
            .get(subject(), ArtifactId::new())
            .await
            .unwrap_err();
        assert!(matches!(error, CoreError::NotFound));
    }
}
