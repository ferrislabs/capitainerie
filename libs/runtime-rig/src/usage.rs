use capitainerie_core::domain::agent::Usage;
use rig_core::completion::Usage as RigUsage;

pub(crate) fn usage_of(model: &str, usage: &RigUsage) -> Usage {
    Usage {
        model: model.to_owned(),
        input_tokens: usage.input_tokens.unwrap_or(0),
        output_tokens: usage.output_tokens.unwrap_or(0),
    }
}

pub(crate) fn truncate_chars(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((index, _)) => text[..index].to_owned(),
        None => text.to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn missing_counters_count_as_zero() {
        let usage = usage_of("m", &RigUsage::default());
        assert_eq!(usage.model, "m");
        assert_eq!((usage.input_tokens, usage.output_tokens), (0, 0));
    }

    #[test]
    fn reported_counters_are_carried_over() {
        let rig: RigUsage =
            serde_json::from_str(r#"{"input_tokens":12,"output_tokens":5}"#).unwrap();
        let usage = usage_of("m", &rig);
        assert_eq!((usage.input_tokens, usage.output_tokens), (12, 5));
    }

    #[test]
    fn truncation_keeps_short_text() {
        assert_eq!(truncate_chars("abc", 3), "abc");
        assert_eq!(truncate_chars("", 3), "");
    }

    #[test]
    fn truncation_cuts_on_a_char_boundary() {
        assert_eq!(truncate_chars("éàüîô", 3), "éàü");
        assert_eq!(truncate_chars("日本語テキスト", 2), "日本");
    }
}
