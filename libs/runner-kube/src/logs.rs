use capitainerie_core::domain::execution::{TaskLine, TaskMarker};
use futures_util::{AsyncBufRead, AsyncBufReadExt};
use serde_json::Value;

const MARKER_TYPE: &str = "capitainerie";
pub const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    Text(String),
    Oversize,
}

/// Reads one line without ever holding more than `max` bytes of it: a longer
/// line is skipped up to its newline and reported as `Oversize`.
pub async fn read_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max: usize,
) -> std::io::Result<Option<Line>> {
    let mut line = Vec::new();
    let mut oversize = false;
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok((!line.is_empty() || oversize).then(|| finish(line, oversize)));
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let end = newline.unwrap_or(available.len());
        if !oversize {
            if line.len() + end > max {
                oversize = true;
                line = Vec::new();
            } else {
                line.extend_from_slice(&available[..end]);
            }
        }
        reader.consume_unpin(newline.map_or(end, |at| at + 1));
        if newline.is_some() {
            return Ok(Some(finish(line, oversize)));
        }
    }
}

fn finish(line: Vec<u8>, oversize: bool) -> Line {
    if oversize {
        return Line::Oversize;
    }
    let text = String::from_utf8_lossy(&line);
    Line::Text(text.trim_end_matches('\r').to_owned())
}

pub fn parse_line(line: &str) -> Option<TaskLine> {
    let value: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => {
            tracing::debug!(len = line.len(), "dropping a non-JSON task log line");
            return None;
        }
    };
    if !value.is_object() {
        tracing::debug!("dropping a JSON task log line that is not an object");
        return None;
    }
    if value.get("type").and_then(Value::as_str) == Some(MARKER_TYPE) {
        return match serde_json::from_value::<TaskMarker>(value) {
            Ok(marker) => Some(TaskLine::Capitainerie(marker)),
            Err(error) => {
                tracing::debug!(%error, "dropping an unknown capitainerie marker");
                None
            }
        };
    }
    Some(TaskLine::Agent(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn lines_of(input: &[u8], max: usize) -> Vec<Line> {
        let mut reader = futures_util::io::Cursor::new(input.to_vec());
        let mut lines = Vec::new();
        while let Some(line) = read_line(&mut reader, max).await.unwrap() {
            lines.push(line);
        }
        lines
    }

    #[tokio::test]
    async fn lines_are_split_on_newlines_and_the_last_one_may_lack_one() {
        assert_eq!(
            lines_of(b"a\r\nbc\n\nd", 16).await,
            [
                Line::Text("a".into()),
                Line::Text("bc".into()),
                Line::Text(String::new()),
                Line::Text("d".into()),
            ]
        );
        assert!(lines_of(b"", 16).await.is_empty());
    }

    #[tokio::test]
    async fn an_oversized_line_is_dropped_and_the_next_one_survives() {
        let mut input = vec![b'x'; 40];
        input.extend_from_slice(b"\nok\n");
        input.extend(vec![b'y'; 40]);
        assert_eq!(
            lines_of(&input, 10).await,
            [Line::Oversize, Line::Text("ok".into()), Line::Oversize]
        );
        assert_eq!(
            lines_of(b"0123456789\n", 10).await,
            [Line::Text("0123456789".into())]
        );
    }

    #[test]
    fn markers_are_parsed() {
        assert_eq!(
            parse_line(r#"{"type":"capitainerie","event":"cloned"}"#),
            Some(TaskLine::Capitainerie(TaskMarker::Cloned))
        );
        assert_eq!(
            parse_line(r#"{"type":"capitainerie","event":"pushed","sha":"abc123"}"#),
            Some(TaskLine::Capitainerie(TaskMarker::Pushed {
                sha: "abc123".into()
            }))
        );
        assert_eq!(
            parse_line(r#"{"type":"capitainerie","event":"no_changes"}"#),
            Some(TaskLine::Capitainerie(TaskMarker::NoChanges))
        );
    }

    #[test]
    fn other_json_objects_are_agent_events() {
        let line = r#"{"type":"assistant","message":{"content":[]}}"#;
        let expected: Value = serde_json::from_str(line).unwrap();
        assert_eq!(parse_line(line), Some(TaskLine::Agent(expected)));
    }

    #[test]
    fn noise_is_dropped() {
        assert_eq!(parse_line("Cloning into 'repo'..."), None);
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("42"), None);
        assert_eq!(parse_line(r#"["a"]"#), None);
        assert_eq!(
            parse_line(r#"{"type":"capitainerie","event":"nope"}"#),
            None
        );
    }
}
