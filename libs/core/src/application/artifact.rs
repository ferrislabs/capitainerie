use capitainerie_common::{ArtifactId, CoreError};
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::domain::{
    artifact::{
        Artifact, ArtifactFilter, ArtifactView,
        service::{ArtifactPage, ArtifactService},
    },
    authorization::Subject,
};

impl CapitainerieUseCase {
    #[transactional(artifact, authz)]
    pub async fn list_artifacts(
        &self,
        subject: Subject,
        filter: ArtifactFilter,
    ) -> Result<ArtifactPage, CoreError> {
        ArtifactService::new(artifact_repository, authz)
            .list(subject, filter)
            .await
    }

    #[transactional(artifact, authz)]
    pub async fn get_artifact(
        &self,
        subject: Subject,
        id: ArtifactId,
    ) -> Result<ArtifactView, CoreError> {
        ArtifactService::new(artifact_repository, authz)
            .get(subject, id)
            .await
    }

    /// Files an artifact the system produced; false when its run already filed one.
    #[transactional(artifact, authz)]
    pub async fn publish_artifact(&self, artifact: Artifact) -> Result<bool, CoreError> {
        ArtifactService::new(artifact_repository, authz)
            .publish(&artifact)
            .await
    }
}
