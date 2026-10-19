pub mod ports;

use base64::{Engine, engine::general_purpose::STANDARD};
use bytes::Bytes;
use capitainerie_common::CoreError;

use crate::domain::provider::ModelRef;

pub const MAX_PDF_BYTES: usize = 25 * 1024 * 1024;

const PDF_MAGIC: &[u8] = b"%PDF-";
const DATA_URI_PREFIX: &str = "data:application/pdf;base64,";

#[derive(Clone)]
pub struct PdfDocument {
    pub file_name: String,
    pub bytes: Bytes,
}

impl std::fmt::Debug for PdfDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfDocument")
            .field("file_name", &self.file_name)
            .field("len", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentText {
    pub text: String,
    pub pages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentReading {
    pub text: String,
    pub pages: usize,
    pub model: ModelRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the file is not a PDF")]
pub struct NotAPdf;

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("no document reader is assigned for this organization")]
    NotConfigured,
    #[error("{0}")]
    Provider(String),
    #[error(transparent)]
    Core(#[from] CoreError),
}

impl PdfDocument {
    pub fn declared_pdf(file_name: &str, content_type: Option<&str>) -> bool {
        let by_extension = file_name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("pdf"));
        let by_type = content_type.is_some_and(|content_type| {
            content_type
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .eq_ignore_ascii_case("application/pdf")
        });
        by_extension || by_type
    }

    pub fn accept(
        file_name: String,
        content_type: Option<&str>,
        bytes: Bytes,
    ) -> Result<Self, NotAPdf> {
        if Self::declared_pdf(&file_name, content_type) && bytes.starts_with(PDF_MAGIC) {
            Ok(Self { file_name, bytes })
        } else {
            Err(NotAPdf)
        }
    }

    pub fn data_uri(&self) -> String {
        let mut uri = String::with_capacity(DATA_URI_PREFIX.len() + self.bytes.len() * 4 / 3 + 4);
        uri.push_str(DATA_URI_PREFIX);
        STANDARD.encode_string(&self.bytes, &mut uri);
        uri
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted(name: &str, content_type: Option<&str>, bytes: &'static [u8]) -> bool {
        PdfDocument::accept(name.into(), content_type, Bytes::from_static(bytes)).is_ok()
    }

    #[test]
    fn debug_shows_the_name_and_the_length_but_never_the_bytes() {
        let document = PdfDocument {
            file_name: "a.pdf".into(),
            bytes: Bytes::from_static(b"%PDF-SECRET-CONTENT"),
        };
        let shown = format!("{document:?}");
        assert!(shown.contains("a.pdf") && shown.contains("19"));
        assert!(!shown.contains("SECRET") && !shown.contains("37"));
    }

    #[test]
    fn the_extension_or_the_content_type_declares_a_pdf() {
        assert!(PdfDocument::declared_pdf("a.pdf", None));
        assert!(PdfDocument::declared_pdf("Rapport.PDF", Some("text/plain")));
        assert!(PdfDocument::declared_pdf(
            "scan",
            Some("Application/PDF; charset=binary")
        ));
        assert!(!PdfDocument::declared_pdf("a.txt", Some("text/plain")));
        assert!(!PdfDocument::declared_pdf("pdf", None));
        assert!(!PdfDocument::declared_pdf("a.pdf.exe", None));
    }

    #[test]
    fn the_bytes_must_start_with_the_pdf_magic_number() {
        assert!(accepted("a.pdf", None, b"%PDF-1.7 rest"));
        assert!(accepted("scan", Some("application/pdf"), b"%PDF-1.4"));
        assert!(!accepted(
            "fake.pdf",
            Some("application/pdf"),
            b"plain text"
        ));
        assert!(!accepted("a.pdf", None, b"%PDF"));
        assert!(!accepted("a.pdf", None, b""));
        assert!(!accepted("a.txt", Some("text/plain"), b"%PDF-1.7"));
    }

    #[test]
    fn the_data_uri_carries_the_pdf_in_base64() {
        let document = PdfDocument {
            file_name: "a.pdf".into(),
            bytes: Bytes::from_static(b"%PDF-1"),
        };
        assert_eq!(document.data_uri(), "data:application/pdf;base64,JVBERi0x");
    }
}
