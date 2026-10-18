use capitainerie_core::domain::execution::{TaskLine, TaskMarker};
use serde_json::Value;

const MARKER_TYPE: &str = "capitainerie";

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
