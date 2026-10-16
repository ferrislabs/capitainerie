use capitainerie_common::CoreError;
use rmcp::{
    ErrorData,
    model::{CallToolResult, ContentBlock},
};
use serde::Serialize;

pub fn tool_failure(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

pub fn tool_success<T: Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    let text = serde_json::to_string(value).map_err(|error| {
        ErrorData::internal_error(format!("serializing the result: {error}"), None)
    })?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

pub fn failure_message(error: &CoreError) -> Option<String> {
    match error {
        CoreError::Invalid(message) => Some(format!("invalid input: {message}")),
        CoreError::NotFound => {
            Some("not found: no such card in this organization. Check the card_id.".to_owned())
        }
        CoreError::Conflict(message) => Some(format!(
            "conflict: {message}. Another change happened first: read the card again and retry with its current version."
        )),
        CoreError::Forbidden { reason } => Some(match reason {
            Some(reason) => format!("forbidden: {reason}"),
            None => "forbidden: you are not allowed to do this".to_owned(),
        }),
        CoreError::Database(_) | CoreError::Internal(_) => None,
    }
}

pub fn into_tool_result<T: Serialize>(
    outcome: Result<T, CoreError>,
) -> Result<CallToolResult, ErrorData> {
    match outcome {
        Ok(value) => tool_success(&value),
        Err(error) => match failure_message(&error) {
            Some(message) => Ok(tool_failure(message)),
            None => {
                tracing::error!(%error, "tool call failed");
                Err(ErrorData::internal_error("internal error", None))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(result: &CallToolResult) -> String {
        serde_json::to_value(result)
            .map(|value| {
                value["content"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned()
            })
            .unwrap_or_default()
    }

    #[test]
    fn domain_errors_become_error_results_the_model_can_read() {
        let forbidden = into_tool_result::<()>(Err(CoreError::Forbidden {
            reason: Some("only a human can take a card out of proposed".into()),
        }))
        .unwrap();
        assert_eq!(forbidden.is_error, Some(true));
        assert!(text(&forbidden).contains("forbidden: only a human"));

        let conflict = into_tool_result::<()>(Err(CoreError::Conflict("stale".into()))).unwrap();
        assert_eq!(conflict.is_error, Some(true));
        assert!(text(&conflict).contains("retry with its current version"));

        let invalid = into_tool_result::<()>(Err(CoreError::Invalid("bad".into()))).unwrap();
        assert!(text(&invalid).contains("invalid input: bad"));

        let missing = into_tool_result::<()>(Err(CoreError::NotFound)).unwrap();
        assert!(text(&missing).contains("not found"));
    }

    #[test]
    fn infrastructure_errors_are_protocol_errors_without_detail() {
        for error in [
            CoreError::Database("password=hunter2".into()),
            CoreError::Internal("secret".into()),
        ] {
            let error = into_tool_result::<()>(Err(error)).unwrap_err();
            assert_eq!(error.message, "internal error");
        }
    }

    #[test]
    fn success_is_json_text() {
        let result = into_tool_result(Ok(serde_json::json!({"a": 1}))).unwrap();
        assert_eq!(result.is_error, Some(false));
        assert_eq!(text(&result), r#"{"a":1}"#);
    }
}
