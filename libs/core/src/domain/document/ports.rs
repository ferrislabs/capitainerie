use capitainerie_common::CoreError;

use super::{DocumentText, PdfDocument};
use crate::domain::provider::{ModelRef, ProviderAccess};

pub trait DocumentReader: Send + Sync {
    fn read(
        &self,
        model: &ModelRef,
        document: PdfDocument,
    ) -> impl Future<Output = Result<DocumentText, CoreError>> + Send;
}

pub trait DocumentReaderFactory: Send + Sync {
    type Bound: DocumentReader;

    fn bind(&self, access: &ProviderAccess) -> Self::Bound;
}
