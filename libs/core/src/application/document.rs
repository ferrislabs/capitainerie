use capitainerie_common::CoreError;
use capitainerie_macros::transactional;

use super::CapitainerieUseCase;
use crate::{
    domain::{
        authorization::Subject,
        document::{
            DocumentReading, PdfDocument, ReadError,
            ports::{DocumentReader, DocumentReaderFactory},
        },
        provider::{ModelRef, ModelTarget, ProviderAccess, ProviderKind, service::ProviderService},
    },
    infrastructure::provider::postgres::PgProviderRepository,
};

#[derive(Debug, Clone)]
pub struct PreparedDocumentReading {
    model: ModelRef,
    access: ProviderAccess,
}

impl CapitainerieUseCase {
    pub async fn prepare_document_reading(
        &self,
        subject: Subject,
    ) -> Result<PreparedDocumentReading, ReadError> {
        let target = self
            .reader_target(subject)
            .await?
            .ok_or(ReadError::NotConfigured)?;
        match target {
            ModelTarget {
                model,
                access: Some(access),
            } if access.kind == ProviderKind::Mistral => {
                Ok(PreparedDocumentReading { model, access })
            }
            _ => Err(ReadError::NotConfigured),
        }
    }

    pub async fn read_document<F: DocumentReaderFactory>(
        &self,
        prepared: PreparedDocumentReading,
        document: PdfDocument,
        factory: &F,
    ) -> Result<DocumentReading, ReadError> {
        let PreparedDocumentReading { model, access } = prepared;
        let bytes = document.bytes.len();
        let read =
            factory
                .bind(&access)
                .read(&model, document)
                .await
                .map_err(|error| match error {
                    CoreError::Internal(message) | CoreError::Invalid(message) => {
                        ReadError::Provider(message)
                    }
                    other => ReadError::Core(other),
                })?;
        tracing::info!(
            bytes,
            pages = read.pages,
            provider = model.provider.as_str(),
            model = model.model.as_str(),
            "read a pdf"
        );
        Ok(DocumentReading {
            text: read.text,
            pages: read.pages,
            model,
        })
    }

    #[transactional(model_assignment, authz)]
    async fn reader_target(&self, subject: Subject) -> Result<Option<ModelTarget>, CoreError> {
        let provider_repository = PgProviderRepository::new(&tx, self.secrets.as_deref());
        ProviderService::new(provider_repository, model_assignment_repository, authz)
            .reader_target(subject)
            .await
    }
}
