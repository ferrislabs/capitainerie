use serde_json::{Map, Value};

use super::{action::ActionRequest, journal::Denial};

pub const MASK: &str = "[masqué]";
pub const MODEL_RESULT_MAX_CHARS: usize = 24_000;

const SECRET_KEYS: [&str; 12] = [
    "password",
    "passwd",
    "secret",
    "client_secret",
    "token",
    "access_token",
    "refresh_token",
    "api_key",
    "apikey",
    "x_api_key",
    "private_key",
    "authorization",
];

pub fn is_secret_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace('-', "_");
    SECRET_KEYS.contains(&normalized.as_str())
}

pub fn redact(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(redact_object(map)),
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        other => other.clone(),
    }
}

fn redact_object(map: &Map<String, Value>) -> Map<String, Value> {
    let named_secret = map
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(is_secret_key);
    map.iter()
        .map(|(key, value)| {
            let hidden =
                !value.is_null() && (is_secret_key(key) || (named_secret && key == "value"));
            let shown = if hidden {
                Value::String(MASK.to_owned())
            } else {
                redact(value)
            };
            (key.clone(), shown)
        })
        .collect()
}

pub fn for_model(value: &Value) -> String {
    truncate(&redact(value).to_string(), MODEL_RESULT_MAX_CHARS)
}

pub fn truncate(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max_chars).collect();
    format!(
        "{kept}\n[truncated: {} characters omitted]",
        total - max_chars
    )
}

pub fn contains_mask(value: &Value) -> bool {
    match value {
        Value::String(text) => text == MASK,
        Value::Array(items) => items.iter().any(contains_mask),
        Value::Object(map) => map.values().any(contains_mask),
        _ => false,
    }
}

pub fn restore_masked(request: &mut ActionRequest, before: Option<&Value>) -> Result<(), Denial> {
    let restored = match request {
        ActionRequest::OdooWrite { values, .. } => restore_odoo_values(values, before),
        ActionRequest::N8nUpdateWorkflow {
            nodes,
            connections,
            settings,
            ..
        } => {
            let nodes_ok = restore_n8n_nodes(nodes, before);
            let connections_ok = !contains_mask(&Value::Object(connections.clone()));
            let settings_ok = match settings {
                Some(map) => {
                    let mut value = Value::Object(std::mem::take(map));
                    let ok = restore_in_place(
                        &mut value,
                        before.and_then(|workflow| workflow.get("settings")),
                    );
                    if let Value::Object(restored) = value {
                        *map = restored;
                    }
                    ok
                }
                None => true,
            };
            nodes_ok && connections_ok && settings_ok
        }
        ActionRequest::OdooCreate { values, .. } => !contains_mask(&Value::Object(values.clone())),
        ActionRequest::OdooCallMethod { args, kwargs, .. } => {
            !args.iter().any(contains_mask) && !contains_mask(&Value::Object(kwargs.clone()))
        }
        ActionRequest::N8nCreateWorkflow {
            nodes,
            connections,
            settings,
            ..
        } => {
            !nodes.iter().any(contains_mask)
                && !contains_mask(&Value::Object(connections.clone()))
                && !contains_mask(&Value::Object(settings.clone()))
        }
        _ => true,
    };
    if restored {
        Ok(())
    } else {
        Err(Denial::MaskedValueWithoutOriginal)
    }
}

fn restore_in_place(target: &mut Value, original: Option<&Value>) -> bool {
    match target {
        Value::String(text) if text == MASK => match original {
            Some(value) if !contains_mask(value) => {
                *target = value.clone();
                true
            }
            _ => false,
        },
        Value::Object(map) => map.iter_mut().all(|(key, value)| {
            restore_in_place(value, original.and_then(|original| original.get(key)))
        }),
        Value::Array(items) => items.iter_mut().enumerate().all(|(index, value)| {
            restore_in_place(value, original.and_then(|original| original.get(index)))
        }),
        _ => true,
    }
}

fn restore_odoo_values(values: &mut Map<String, Value>, before: Option<&Value>) -> bool {
    let records = before.and_then(Value::as_array);
    values.iter_mut().all(|(key, value)| {
        if !contains_mask(value) {
            return true;
        }
        let Some(records) = records else {
            return false;
        };
        let mut originals = records.iter().map(|record| record.get(key));
        let Some(Some(first)) = originals.next() else {
            return false;
        };
        if !originals.all(|other| other == Some(first)) {
            return false;
        }
        let original = odoo_writable(first);
        restore_in_place(value, Some(&original))
    })
}

fn odoo_writable(read: &Value) -> Value {
    match read.as_array().map(Vec::as_slice) {
        Some([Value::Number(id), Value::String(_)]) => Value::Number(id.clone()),
        _ => read.clone(),
    }
}

fn restore_n8n_nodes(nodes: &mut [Value], before: Option<&Value>) -> bool {
    let originals = before
        .and_then(|workflow| workflow.get("nodes"))
        .and_then(Value::as_array);
    nodes.iter_mut().all(|node| {
        if !contains_mask(node) {
            return true;
        }
        let original = originals.and_then(|originals| {
            originals
                .iter()
                .find(|candidate| same_node(candidate, node, "id"))
                .or_else(|| {
                    originals
                        .iter()
                        .find(|candidate| same_node(candidate, node, "name"))
                })
        });
        restore_in_place(node, original)
    })
}

fn same_node(candidate: &Value, node: &Value, key: &str) -> bool {
    match (candidate.get(key), node.get(key)) {
        (Some(Value::String(left)), Some(Value::String(right))) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::domain::integration::action::{N8nId, OdooModel};

    #[test]
    fn secret_keys_are_matched_exactly_after_normalizing_case_and_dashes() {
        for key in [
            "password",
            "API_KEY",
            "x-api-key",
            "Authorization",
            "client_secret",
        ] {
            assert!(is_secret_key(key), "{key}");
        }
        for key in [
            "maxTokens",
            "token_count",
            "credentials",
            "secretary",
            "name",
        ] {
            assert!(!is_secret_key(key), "{key}");
        }
    }

    #[test]
    fn redaction_masks_secret_keys_and_named_header_values_at_any_depth() {
        let value = json!({
            "name": "Relance",
            "nodes": [{
                "parameters": {
                    "maxTokens": 400,
                    "apiKey": "sk-live",
                    "headerParameters": {"parameters": [
                        {"name": "Authorization", "value": "Bearer abc"},
                        {"name": "Accept", "value": "application/json"}
                    ]}
                },
                "credentials": {"httpBasicAuth": {"id": "7", "name": "Prod"}}
            }],
            "password": null
        });
        let shown = redact(&value);
        let parameters = &shown["nodes"][0]["parameters"];
        assert_eq!(parameters["apiKey"], MASK);
        assert_eq!(parameters["maxTokens"], 400);
        assert_eq!(
            parameters["headerParameters"]["parameters"][0]["value"],
            MASK
        );
        assert_eq!(
            parameters["headerParameters"]["parameters"][1]["value"],
            "application/json"
        );
        assert_eq!(shown["nodes"][0]["credentials"]["httpBasicAuth"]["id"], "7");
        assert_eq!(shown["password"], Value::Null);
        assert!(!shown.to_string().contains("sk-live"));
        assert!(!shown.to_string().contains("Bearer abc"));
    }

    #[test]
    fn a_long_result_is_truncated_with_a_marker() {
        let text = truncate(&"a".repeat(30), 10);
        assert!(text.starts_with("aaaaaaaaaa\n[truncated: 20 characters omitted]"));
        assert_eq!(truncate("short", 10), "short");
    }

    fn model(name: &str) -> OdooModel {
        OdooModel::try_from(name).unwrap()
    }

    #[test]
    fn a_masked_odoo_value_is_put_back_from_the_copy_before() {
        let mut request = ActionRequest::OdooWrite {
            model: model("res.partner"),
            ids: vec![1, 2],
            values: json!({"api_key": MASK, "name": "Acme"})
                .as_object()
                .unwrap()
                .clone(),
        };
        let before = json!([{"id": 1, "api_key": "k-1"}, {"id": 2, "api_key": "k-1"}]);
        restore_masked(&mut request, Some(&before)).unwrap();
        let ActionRequest::OdooWrite { values, .. } = request else {
            unreachable!()
        };
        assert_eq!(values["api_key"], "k-1");
    }

    #[test]
    fn a_masked_odoo_value_with_differing_or_missing_originals_is_refused() {
        let values = json!({"api_key": MASK}).as_object().unwrap().clone();
        let mut differing = ActionRequest::OdooWrite {
            model: model("res.partner"),
            ids: vec![1, 2],
            values: values.clone(),
        };
        let before = json!([{"id": 1, "api_key": "a"}, {"id": 2, "api_key": "b"}]);
        assert_eq!(
            restore_masked(&mut differing, Some(&before)),
            Err(Denial::MaskedValueWithoutOriginal)
        );
        let mut created = ActionRequest::OdooCreate {
            model: model("res.partner"),
            values,
        };
        assert_eq!(
            restore_masked(&mut created, None),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_many2one_read_as_a_pair_is_written_back_as_its_id() {
        let mut request = ActionRequest::OdooWrite {
            model: model("res.partner"),
            ids: vec![1],
            values: json!({"user_id": MASK}).as_object().unwrap().clone(),
        };
        let before = json!([{"id": 1, "user_id": [7, "Marie"]}]);
        restore_masked(&mut request, Some(&before)).unwrap();
        let ActionRequest::OdooWrite { values, .. } = request else {
            unreachable!()
        };
        assert_eq!(values["user_id"], 7);
    }

    #[test]
    fn a_masked_n8n_header_is_put_back_by_node_id_even_when_nodes_move() {
        let before = json!({"nodes": [
            {"id": "a", "name": "Start", "parameters": {}},
            {"id": "b", "name": "Call", "parameters": {"headerParameters": {"parameters": [
                {"name": "Authorization", "value": "Bearer real"}
            ]}}}
        ]});
        let mut request = ActionRequest::N8nUpdateWorkflow {
            workflow_id: N8nId::try_from("w1").unwrap(),
            name: None,
            nodes: vec![
                json!({"id": "new", "name": "Log", "parameters": {}}),
                json!({"id": "b", "name": "Call", "parameters": {"headerParameters": {"parameters": [
                    {"name": "Authorization", "value": MASK}
                ]}}}),
            ],
            connections: Map::new(),
            settings: None,
            publish: false,
        };
        restore_masked(&mut request, Some(&before)).unwrap();
        let ActionRequest::N8nUpdateWorkflow { nodes, .. } = request else {
            unreachable!()
        };
        assert_eq!(
            nodes[1]["parameters"]["headerParameters"]["parameters"][0]["value"],
            "Bearer real"
        );
    }

    #[test]
    fn a_masked_value_in_a_new_n8n_node_is_refused() {
        let before = json!({"nodes": []});
        let mut request = ActionRequest::N8nUpdateWorkflow {
            workflow_id: N8nId::try_from("w1").unwrap(),
            name: None,
            nodes: vec![json!({"id": "x", "name": "X", "parameters": {"apiKey": MASK}})],
            connections: Map::new(),
            settings: None,
            publish: false,
        };
        assert_eq!(
            restore_masked(&mut request, Some(&before)),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }
}
