use serde_json::{Map, Value};

use crate::domain::mcp::ToolInfo;

pub const TOOL_NAME_MAX_CHARS: usize = 64;
pub const ARGUMENTS_MAX_CHARS: usize = 200;
pub const SEPARATOR: &str = "__";
pub const MASK: &str = "[masqué]";

const READ_PREFIXES: [&str; 4] = ["list_", "get_", "search_", "export_"];
const SECRET_FRAGMENTS: [&str; 6] = [
    "token",
    "secret",
    "password",
    "passwd",
    "authorization",
    "api_key",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ExposedTool {
    pub name: String,
    pub connector: usize,
    pub original: String,
    pub description: String,
    pub schema: Value,
    pub write: bool,
}

pub fn is_read_only(tool: &ToolInfo) -> bool {
    if tool.destructive_hint == Some(true) {
        return false;
    }
    match tool.read_only_hint {
        Some(hint) => hint,
        None => READ_PREFIXES
            .iter()
            .any(|prefix| tool.name.starts_with(prefix)),
    }
}

fn sanitized(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn exposed_name(slug: &str, tool: &str, taken: &[String]) -> String {
    let mut base = format!("{}{SEPARATOR}{}", sanitized(slug), sanitized(tool));
    base.truncate(TOOL_NAME_MAX_CHARS);
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| {
            let suffix = format!("_{n}");
            let mut candidate = base.clone();
            candidate.truncate(TOOL_NAME_MAX_CHARS - suffix.len());
            candidate.push_str(&suffix);
            candidate
        })
        .find(|candidate| !taken.contains(candidate))
        .unwrap_or(base)
}

pub fn expose(
    connector: usize,
    slug: &str,
    tools: &[ToolInfo],
    taken: &mut Vec<String>,
) -> Vec<ExposedTool> {
    let mut exposed = Vec::with_capacity(tools.len());
    for tool in tools {
        let name = exposed_name(slug, &tool.name, taken);
        taken.push(name.clone());
        exposed.push(ExposedTool {
            name,
            connector,
            original: tool.name.clone(),
            description: tool.description.clone().unwrap_or_default(),
            schema: if tool.input_schema.is_object() {
                tool.input_schema.clone()
            } else {
                serde_json::json!({"type": "object"})
            },
            write: !is_read_only(tool),
        });
    }
    exposed
}

pub fn masked(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, inner)| {
                    let lowered = key.to_lowercase();
                    if SECRET_FRAGMENTS
                        .iter()
                        .any(|fragment| lowered.contains(fragment))
                    {
                        (key.clone(), Value::String(MASK.to_owned()))
                    } else {
                        (key.clone(), masked(inner))
                    }
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(masked).collect()),
        other => other.clone(),
    }
}

pub fn redacted_arguments(arguments: &Value) -> String {
    let text = masked(arguments).to_string();
    match text.char_indices().nth(ARGUMENTS_MAX_CHARS) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn info(name: &str, read_only: Option<bool>, destructive: Option<bool>) -> ToolInfo {
        ToolInfo {
            name: name.into(),
            description: None,
            input_schema: json!({"type": "object"}),
            read_only_hint: read_only,
            destructive_hint: destructive,
        }
    }

    fn valid(name: &str) -> bool {
        (1..=64).contains(&name.len())
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }

    #[test]
    fn a_read_only_hint_makes_a_tool_a_read() {
        assert!(is_read_only(&info("anything", Some(true), None)));
    }

    #[test]
    fn without_a_hint_the_name_prefix_decides() {
        for name in ["list_orders", "get_order", "search_clients", "export_csv"] {
            assert!(is_read_only(&info(name, None, None)), "{name}");
        }
        for name in [
            "create_order",
            "update_client",
            "delete_x",
            "listing",
            "run",
        ] {
            assert!(!is_read_only(&info(name, None, None)), "{name}");
        }
    }

    #[test]
    fn an_explicit_write_or_destructive_hint_beats_the_name() {
        assert!(!is_read_only(&info("get_order", Some(false), None)));
        assert!(!is_read_only(&info("get_order", None, Some(true))));
        assert!(!is_read_only(&info("get_order", Some(true), Some(true))));
    }

    #[test]
    fn names_are_prefixed_by_the_connector_and_valid() {
        assert_eq!(
            exposed_name("mestier", "list_orders", &[]),
            "mestier__list_orders"
        );
        let odd = exposed_name("mestier", "orders.list v2/é", &[]);
        assert_eq!(odd, "mestier__orders_list_v2__");
        assert!(valid(&odd));
    }

    #[test]
    fn long_names_are_cut_to_64_and_stay_unique() {
        let long = "a".repeat(100);
        let first = exposed_name("mestier", &long, &[]);
        assert_eq!(first.len(), 64);
        let second = exposed_name("mestier", &long, std::slice::from_ref(&first));
        assert_ne!(first, second);
        assert!(valid(&second));
        let third = exposed_name("mestier", &long, &[first, second.clone()]);
        assert!(valid(&third) && third != second);
    }

    #[test]
    fn exposing_keeps_the_original_name_and_classifies() {
        let mut taken = Vec::new();
        let tools = [
            info("list_orders", None, None),
            info("create_order", None, None),
        ];
        let exposed = expose(0, "mestier", &tools, &mut taken);
        assert_eq!(exposed[0].name, "mestier__list_orders");
        assert_eq!(exposed[0].original, "list_orders");
        assert!(!exposed[0].write);
        assert!(exposed[1].write);
        assert_eq!(taken.len(), 2);
    }

    #[test]
    fn arguments_are_masked_and_cut_to_200_characters() {
        let text = redacted_arguments(
            &json!({"q": "x", "api_token": "s3cr3t", "nested": {"Password": "p"}}),
        );
        assert!(!text.contains("s3cr3t") && !text.contains("\"p\""));
        assert!(text.contains(MASK));
        let long = redacted_arguments(&json!({"q": "é".repeat(500)}));
        assert_eq!(long.chars().count(), ARGUMENTS_MAX_CHARS + 1);
        assert!(long.ends_with('…'));
    }
}
