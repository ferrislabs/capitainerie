use capitainerie_runtime_rig::{RigDocumentReaders, RigModelCatalog, RigTranscribers};

/// The single place that names the catalog adapter the API serves `/models` with.
pub type ApiCatalog = RigModelCatalog;

pub fn build() -> anyhow::Result<ApiCatalog> {
    Ok(RigModelCatalog::new()?)
}

pub type ApiTranscribers = RigTranscribers;

pub fn build_transcribers() -> anyhow::Result<ApiTranscribers> {
    Ok(RigTranscribers::new()?)
}

pub type ApiDocumentReaders = RigDocumentReaders;

pub fn build_document_readers() -> anyhow::Result<ApiDocumentReaders> {
    Ok(RigDocumentReaders::new()?)
}
