use capitainerie_common::CoreError;
use rig_reqwest::reqwest;

pub(crate) const MAX_PENDING_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;
pub(crate) const MAX_TOOL_JSON_BYTES: usize = 256 * 1024;
pub(crate) const MAX_REPLY_BYTES: usize = 512 * 1024;

pub(crate) fn overflow(what: &str) -> CoreError {
    too_large(what)
}

fn too_large(what: &str) -> CoreError {
    CoreError::Internal(format!("the language model {what} is too large"))
}

#[derive(Debug, Default)]
pub(crate) struct SseDecoder {
    pending: Vec<u8>,
    data: Vec<String>,
    data_bytes: usize,
}

impl SseDecoder {
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>, CoreError> {
        self.pending.extend_from_slice(chunk);
        if self.pending.len() > MAX_PENDING_BYTES {
            return Err(too_large("stream event"));
        }
        let mut events = Vec::new();
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches(['\n', '\r']);
            if line.is_empty() {
                if !self.data.is_empty() {
                    events.push(self.data.join("\n"));
                    self.data.clear();
                    self.data_bytes = 0;
                }
            } else if let Some(value) = line.strip_prefix("data:") {
                self.data_bytes += value.len();
                if self.data_bytes > MAX_PENDING_BYTES {
                    return Err(too_large("stream event"));
                }
                self.data
                    .push(value.strip_prefix(' ').unwrap_or(value).to_owned());
            }
        }
        Ok(events)
    }

    pub(crate) fn finish(&mut self) -> Option<String> {
        if self.data.is_empty() {
            return None;
        }
        let event = self.data.join("\n");
        self.data.clear();
        self.data_bytes = 0;
        Some(event)
    }
}

pub(crate) async fn read_capped_text(response: &mut reqwest::Response, max: usize) -> String {
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        let room = max.saturating_sub(body.len());
        body.extend_from_slice(&chunk[..chunk.len().min(room)]);
        if body.len() >= max {
            break;
        }
    }
    String::from_utf8_lossy(&body).into_owned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn an_event_ends_at_the_blank_line_whatever_the_chunking() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(b"event: x\ndata: {\"a\"").unwrap().is_empty());
        let events = decoder.push(b":1}\n\ndata: second\n\n").unwrap();
        assert_eq!(events, ["{\"a\":1}", "second"]);
    }

    #[test]
    fn crlf_comments_and_multi_line_data_are_handled() {
        let mut decoder = SseDecoder::default();
        let events = decoder
            .push(b": keep-alive\r\ndata: one\r\ndata: two\r\n\r\n")
            .unwrap();
        assert_eq!(events, ["one\ntwo"]);
    }

    #[test]
    fn a_multibyte_character_split_across_chunks_survives() {
        let mut decoder = SseDecoder::default();
        let bytes = "data: é\n\n".as_bytes();
        assert!(decoder.push(&bytes[..7]).unwrap().is_empty());
        assert_eq!(decoder.push(&bytes[7..]).unwrap(), ["é"]);
    }

    #[test]
    fn a_last_event_without_its_blank_line_is_flushed() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(b"data: tail\n").unwrap().is_empty());
        assert_eq!(decoder.finish().as_deref(), Some("tail"));
        assert_eq!(decoder.finish(), None);
    }

    #[test]
    fn an_event_that_never_ends_fails_past_one_mebibyte() {
        let mut decoder = SseDecoder::default();
        let block = vec![b'x'; 256 * 1024];
        for _ in 0..4 {
            assert!(decoder.push(&block).unwrap().is_empty());
        }

        let error = decoder.push(b"y").unwrap_err();

        assert!(error.to_string().contains("too large"));
    }

    #[test]
    fn many_data_lines_of_one_event_count_together() {
        let mut decoder = SseDecoder::default();
        let line = format!("data: {}\n", "x".repeat(100 * 1024));
        let failed = (0..12).any(|_| decoder.push(line.as_bytes()).is_err());
        assert!(failed);
    }

    #[tokio::test]
    async fn an_error_body_is_read_up_to_its_cap_only() {
        let app = axum::Router::new().route(
            "/",
            axum::routing::get(|| async { "e".repeat(MAX_ERROR_BODY_BYTES * 3) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await });
        let mut response = reqwest::get(url).await.unwrap();

        let text = read_capped_text(&mut response, MAX_ERROR_BODY_BYTES).await;

        assert_eq!(text.len(), MAX_ERROR_BODY_BYTES);
    }
}
