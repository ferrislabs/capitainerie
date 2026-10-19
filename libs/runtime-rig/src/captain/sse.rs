#[derive(Debug, Default)]
pub(crate) struct SseDecoder {
    pending: Vec<u8>,
    data: Vec<String>,
}

impl SseDecoder {
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches(['\n', '\r']);
            if line.is_empty() {
                if !self.data.is_empty() {
                    events.push(self.data.join("\n"));
                    self.data.clear();
                }
            } else if let Some(value) = line.strip_prefix("data:") {
                self.data
                    .push(value.strip_prefix(' ').unwrap_or(value).to_owned());
            }
        }
        events
    }

    pub(crate) fn finish(&mut self) -> Option<String> {
        if self.data.is_empty() {
            return None;
        }
        let event = self.data.join("\n");
        self.data.clear();
        Some(event)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn an_event_ends_at_the_blank_line_whatever_the_chunking() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(b"event: x\ndata: {\"a\"").is_empty());
        let events = decoder.push(b":1}\n\ndata: second\n\n");
        assert_eq!(events, ["{\"a\":1}", "second"]);
    }

    #[test]
    fn crlf_comments_and_multi_line_data_are_handled() {
        let mut decoder = SseDecoder::default();
        let events = decoder.push(b": keep-alive\r\ndata: one\r\ndata: two\r\n\r\n");
        assert_eq!(events, ["one\ntwo"]);
    }

    #[test]
    fn a_multibyte_character_split_across_chunks_survives() {
        let mut decoder = SseDecoder::default();
        let bytes = "data: é\n\n".as_bytes();
        assert!(decoder.push(&bytes[..7]).is_empty());
        assert_eq!(decoder.push(&bytes[7..]), ["é"]);
    }

    #[test]
    fn a_last_event_without_its_blank_line_is_flushed() {
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(b"data: tail\n").is_empty());
        assert_eq!(decoder.finish().as_deref(), Some("tail"));
        assert_eq!(decoder.finish(), None);
    }
}
