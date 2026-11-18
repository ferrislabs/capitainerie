use aws_lc_rs::digest;
use base64::{Engine, engine::general_purpose::STANDARD};

/// The most an audit report may weigh once decoded; the entrypoint cuts it
/// to this size before it sends it.
pub const REPORT_MAX_BYTES: usize = 1024 * 1024;
/// Base64 of `REPORT_MAX_BYTES`, plus room for padding.
const ENCODED_MAX_BYTES: usize = REPORT_MAX_BYTES.div_ceil(3) * 4 + 4;

/// What the task's output said about its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportOutcome {
    /// The task wrote no report.
    Missing,
    /// The report arrived but cannot be trusted; the reason says why.
    Invalid(String),
    Ready {
        text: String,
        truncated: bool,
    },
}

/// Rebuilds the report from the chunks of the task's output and checks it
/// against the digest the entrypoint computed.
#[derive(Debug, Clone, Default)]
pub struct ReportAssembler {
    encoded: String,
    overflow: bool,
    ended: Option<ReportEnd>,
}

#[derive(Debug, Clone)]
struct ReportEnd {
    sha256: String,
    bytes: u64,
    truncated: bool,
}

impl ReportAssembler {
    pub fn chunk(&mut self, data: &str) {
        if self.ended.is_some() || self.overflow {
            return;
        }
        if self.encoded.len() + data.len() > ENCODED_MAX_BYTES {
            self.overflow = true;
            self.encoded = String::new();
            return;
        }
        self.encoded.push_str(data.trim());
    }

    pub fn end(&mut self, sha256: &str, bytes: u64, truncated: bool) {
        if self.ended.is_none() {
            self.ended = Some(ReportEnd {
                sha256: sha256.trim().to_ascii_lowercase(),
                bytes,
                truncated,
            });
        }
    }

    pub fn outcome(&self) -> ReportOutcome {
        let Some(end) = &self.ended else {
            return if self.encoded.is_empty() && !self.overflow {
                ReportOutcome::Missing
            } else {
                ReportOutcome::Invalid("le rapport est incomplet".into())
            };
        };
        if self.overflow {
            return ReportOutcome::Invalid("le rapport dépasse 1 Mio".into());
        }
        let Ok(decoded) = STANDARD.decode(self.encoded.as_bytes()) else {
            return ReportOutcome::Invalid("le rapport n'est pas du base64 valide".into());
        };
        if decoded.len() > REPORT_MAX_BYTES {
            return ReportOutcome::Invalid("le rapport dépasse 1 Mio".into());
        }
        if u64::try_from(decoded.len()).ok() != Some(end.bytes) {
            return ReportOutcome::Invalid("la taille du rapport ne correspond pas".into());
        }
        if hex(digest::digest(&digest::SHA256, &decoded).as_ref()) != end.sha256 {
            return ReportOutcome::Invalid("l'empreinte du rapport ne correspond pas".into());
        }
        let text = String::from_utf8_lossy(&decoded).into_owned();
        if text.trim().is_empty() {
            return ReportOutcome::Missing;
        }
        ReportOutcome::Ready {
            text,
            truncated: end.truncated,
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(bytes: &[u8]) -> String {
        hex(digest::digest(&digest::SHA256, bytes).as_ref())
    }

    fn sent(text: &[u8], chunk: usize) -> ReportAssembler {
        let encoded = STANDARD.encode(text);
        let mut assembler = ReportAssembler::default();
        for piece in encoded.as_bytes().chunks(chunk) {
            assembler.chunk(std::str::from_utf8(piece).unwrap());
        }
        assembler.end(&sha(text), text.len() as u64, false);
        assembler
    }

    #[test]
    fn chunks_rebuild_the_report_whatever_their_size() {
        let text = "# Audit\n\nTout va bien, sauf `unsafe` dans lib.rs. é".repeat(50);
        for chunk in [1, 3, 7, 4096] {
            assert_eq!(
                sent(text.as_bytes(), chunk).outcome(),
                ReportOutcome::Ready {
                    text: text.clone(),
                    truncated: false
                },
                "{chunk}"
            );
        }
    }

    #[test]
    fn a_digest_mismatch_or_a_wrong_size_is_refused() {
        let mut tampered = ReportAssembler::default();
        tampered.chunk(&STANDARD.encode("rapport modifié"));
        tampered.end(&sha(b"rapport"), 16, false);
        assert!(
            matches!(tampered.outcome(), ReportOutcome::Invalid(reason) if reason.contains("empreinte"))
        );

        let mut short = ReportAssembler::default();
        short.chunk(&STANDARD.encode("rapport"));
        short.end(&sha(b"rapport"), 99, false);
        assert!(
            matches!(short.outcome(), ReportOutcome::Invalid(reason) if reason.contains("taille"))
        );
    }

    #[test]
    fn the_digest_is_compared_without_case() {
        let mut assembler = ReportAssembler::default();
        assembler.chunk(&STANDARD.encode("ok"));
        assembler.end(&sha(b"ok").to_uppercase(), 2, false);
        assert!(matches!(assembler.outcome(), ReportOutcome::Ready { .. }));
    }

    #[test]
    fn a_truncated_report_says_so() {
        let mut assembler = ReportAssembler::default();
        assembler.chunk(&STANDARD.encode("début…"));
        assembler.end(&sha("début…".as_bytes()), "début…".len() as u64, true);
        assert_eq!(
            assembler.outcome(),
            ReportOutcome::Ready {
                text: "début…".into(),
                truncated: true
            }
        );
    }

    #[test]
    fn no_chunk_and_no_end_is_a_missing_report() {
        assert_eq!(ReportAssembler::default().outcome(), ReportOutcome::Missing);
        let mut blank = ReportAssembler::default();
        blank.chunk(&STANDARD.encode("  \n"));
        blank.end(&sha(b"  \n"), 3, false);
        assert_eq!(blank.outcome(), ReportOutcome::Missing);
    }

    #[test]
    fn chunks_without_an_end_are_incomplete() {
        let mut assembler = ReportAssembler::default();
        assembler.chunk(&STANDARD.encode("moitié"));
        assert!(
            matches!(assembler.outcome(), ReportOutcome::Invalid(reason) if reason.contains("incomplet"))
        );
    }

    #[test]
    fn more_than_the_cap_is_refused_without_being_kept() {
        let mut assembler = ReportAssembler::default();
        let piece = "A".repeat(64 * 1024);
        for _ in 0..(ENCODED_MAX_BYTES / piece.len() + 2) {
            assembler.chunk(&piece);
        }
        assert!(assembler.encoded.is_empty());
        assembler.end("00", 0, false);
        assert!(
            matches!(assembler.outcome(), ReportOutcome::Invalid(reason) if reason.contains("1 Mio"))
        );
    }

    #[test]
    fn invalid_base64_is_refused_and_chunks_after_the_end_are_ignored() {
        let mut assembler = ReportAssembler::default();
        assembler.chunk("@@@@");
        assembler.end(&sha(b""), 0, false);
        assert!(matches!(assembler.outcome(), ReportOutcome::Invalid(_)));

        let mut late = ReportAssembler::default();
        late.chunk(&STANDARD.encode("ok"));
        late.end(&sha(b"ok"), 2, false);
        late.chunk(&STANDARD.encode("injected"));
        assert!(matches!(late.outcome(), ReportOutcome::Ready { text, .. } if text == "ok"));
    }
}
