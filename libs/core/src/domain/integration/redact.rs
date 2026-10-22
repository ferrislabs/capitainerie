use serde_json::{Map, Value};

use super::{action::ActionRequest, journal::Denial};

pub const MASK: &str = "[masqué]";
pub const MODEL_RESULT_MAX_CHARS: usize = 24_000;

const SECRET_KEYS: [&str; 16] = [
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
    "oauth_access_token",
    "totp_secret",
    "new_password",
    "password_crypt",
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
                    let ok = restore_checked(
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

fn is_mask(value: &Value) -> bool {
    matches!(value, Value::String(text) if text == MASK)
}

fn restore_checked(target: &mut Value, original: Option<&Value>) -> bool {
    let expected = redact(target);
    restore_in_place(target, original) && redact(target) == expected
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
        Value::Object(map) => restore_object(map, original),
        Value::Array(items) => items.iter_mut().enumerate().all(|(index, value)| {
            !is_mask(value)
                && restore_in_place(value, original.and_then(|original| original.get(index)))
        }),
        _ => true,
    }
}

fn restore_object(map: &mut Map<String, Value>, original: Option<&Value>) -> bool {
    let keeps_secret_name = map.get("name").and_then(Value::as_str).is_some_and(|name| {
        is_secret_key(name)
            && original
                .and_then(|original| original.get("name"))
                .and_then(Value::as_str)
                == Some(name)
    });
    map.iter_mut().all(|(key, value)| {
        let stays_masked = is_secret_key(key) || (key == "value" && keeps_secret_name);
        if is_mask(value) && !stays_masked {
            return false;
        }
        restore_in_place(
            value,
            original.and_then(|original| original.get(key.as_str())),
        )
    })
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
        match original {
            Some(original) => {
                same_destination(original, node) && restore_checked(node, Some(original))
            }
            None => false,
        }
    })
}

const DESTINATIONS: [&str; 5] = [
    "/parameters/url",
    "/parameters/baseUrl",
    "/parameters/host",
    "/parameters/options/proxy",
    "/credentials",
];

fn same_destination(original: &Value, node: &Value) -> bool {
    original.get("type") == node.get("type")
        && DESTINATIONS
            .iter()
            .all(|pointer| original.pointer(pointer) == node.pointer(pointer))
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

    fn call_node(id: &str, url: &str, kind: &str, header: &str) -> Value {
        json!({
            "id": id,
            "name": "Call",
            "type": kind,
            "parameters": {
                "url": url,
                "headerParameters": {"parameters": [{"name": header, "value": MASK}]}
            }
        })
    }

    fn update_with(node: Value) -> ActionRequest {
        ActionRequest::N8nUpdateWorkflow {
            workflow_id: N8nId::try_from("w1").unwrap(),
            name: None,
            nodes: vec![node],
            connections: Map::new(),
            settings: None,
            publish: false,
        }
    }

    fn call_before() -> Value {
        json!({"nodes": [{
            "id": "b",
            "name": "Call",
            "type": "n8n-nodes-base.httpRequest",
            "parameters": {
                "url": "https://api.acme.test/v1",
                "headerParameters": {"parameters": [
                    {"name": "Authorization", "value": "Bearer real"}
                ]}
            }
        }]})
    }

    #[test]
    fn a_masked_header_keeps_its_value_when_the_node_is_unchanged() {
        let mut request = update_with(call_node(
            "b",
            "https://api.acme.test/v1",
            "n8n-nodes-base.httpRequest",
            "Authorization",
        ));
        restore_masked(&mut request, Some(&call_before())).unwrap();
        let ActionRequest::N8nUpdateWorkflow { nodes, .. } = request else {
            unreachable!()
        };
        assert_eq!(
            nodes[0]["parameters"]["headerParameters"]["parameters"][0]["value"],
            "Bearer real"
        );
        assert_eq!(
            redact(&nodes[0])["parameters"]["headerParameters"]["parameters"][0]["value"],
            MASK
        );
    }

    #[test]
    fn a_masked_header_in_a_node_sent_through_a_new_proxy_is_refused() {
        let mut node = call_node(
            "b",
            "https://api.acme.test/v1",
            "n8n-nodes-base.httpRequest",
            "Authorization",
        );
        node["parameters"]["options"] = json!({"proxy": "http://attacker.test:8080"});
        let mut request = update_with(node);
        assert_eq!(
            restore_masked(&mut request, Some(&call_before())),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_masked_header_renamed_to_a_plain_name_is_refused() {
        let mut request = update_with(call_node(
            "b",
            "https://api.acme.test/v1",
            "n8n-nodes-base.httpRequest",
            "X-Foo",
        ));
        assert_eq!(
            restore_masked(&mut request, Some(&call_before())),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_masked_header_renamed_to_another_secret_name_is_refused() {
        let mut request = update_with(call_node(
            "b",
            "https://api.acme.test/v1",
            "n8n-nodes-base.httpRequest",
            "X-Api-Key",
        ));
        assert_eq!(
            restore_masked(&mut request, Some(&call_before())),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_masked_header_moved_to_another_index_is_refused() {
        let before = json!({"nodes": [{
            "id": "b",
            "name": "Call",
            "parameters": {"headerParameters": {"parameters": [
                {"name": "Accept", "value": "application/json"},
                {"name": "Authorization", "value": "Bearer real"}
            ]}}
        }]});
        let mut request = update_with(json!({
            "id": "b",
            "name": "Call",
            "parameters": {"headerParameters": {"parameters": [
                {"name": "Authorization", "value": MASK},
                {"name": "Accept", "value": "application/json"}
            ]}}
        }));
        assert_eq!(
            restore_masked(&mut request, Some(&before)),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_masked_value_in_a_node_whose_url_changed_is_refused() {
        for url in ["https://evil.test/v1", "https://api.acme.test/v2"] {
            let mut request = update_with(call_node(
                "b",
                url,
                "n8n-nodes-base.httpRequest",
                "Authorization",
            ));
            assert_eq!(
                restore_masked(&mut request, Some(&call_before())),
                Err(Denial::MaskedValueWithoutOriginal),
                "{url}"
            );
        }
    }

    #[test]
    fn a_masked_value_in_a_node_that_gained_or_lost_a_url_is_refused() {
        let mut gained = update_with(json!({
            "id": "b",
            "name": "Call",
            "parameters": {
                "url": "https://evil.test",
                "apiKey": MASK
            }
        }));
        let without_url = json!({"nodes": [{
            "id": "b",
            "name": "Call",
            "parameters": {"apiKey": "sk-live"}
        }]});
        assert_eq!(
            restore_masked(&mut gained, Some(&without_url)),
            Err(Denial::MaskedValueWithoutOriginal)
        );
        let mut lost = update_with(json!({
            "id": "b",
            "name": "Call",
            "parameters": {"apiKey": MASK}
        }));
        let with_url = json!({"nodes": [{
            "id": "b",
            "name": "Call",
            "parameters": {"url": "https://api.acme.test", "apiKey": "sk-live"}
        }]});
        assert_eq!(
            restore_masked(&mut lost, Some(&with_url)),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_masked_value_in_a_node_whose_type_changed_is_refused() {
        let mut request = update_with(call_node(
            "b",
            "https://api.acme.test/v1",
            "n8n-nodes-base.webhook",
            "Authorization",
        ));
        assert_eq!(
            restore_masked(&mut request, Some(&call_before())),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_changed_url_or_type_does_not_matter_in_a_node_without_masked_values() {
        let mut request = update_with(json!({
            "id": "b",
            "name": "Call",
            "type": "n8n-nodes-base.webhook",
            "parameters": {"url": "https://other.test"}
        }));
        restore_masked(&mut request, Some(&call_before())).unwrap();
    }

    #[test]
    fn a_masked_settings_value_follows_the_same_rules() {
        let before = json!({"settings": {"headers": [
            {"name": "Authorization", "value": "Bearer real"}
        ]}});
        let settings = |header: &str| {
            json!({"headers": [{"name": header, "value": MASK}]})
                .as_object()
                .unwrap()
                .clone()
        };
        let request = |header: &str| ActionRequest::N8nUpdateWorkflow {
            workflow_id: N8nId::try_from("w1").unwrap(),
            name: None,
            nodes: vec![],
            connections: Map::new(),
            settings: Some(settings(header)),
            publish: false,
        };
        let mut kept = request("Authorization");
        restore_masked(&mut kept, Some(&before)).unwrap();
        let ActionRequest::N8nUpdateWorkflow { settings, .. } = kept else {
            unreachable!()
        };
        assert_eq!(settings.unwrap()["headers"][0]["value"], "Bearer real");
        let mut renamed = request("X-Foo");
        assert_eq!(
            restore_masked(&mut renamed, Some(&before)),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn a_masked_value_under_a_plain_key_is_refused() {
        let mut request = update_with(json!({
            "id": "b",
            "name": "Call",
            "type": "n8n-nodes-base.httpRequest",
            "parameters": {"url": "https://api.acme.test/v1", "path": MASK}
        }));
        let before = json!({"nodes": [{
            "id": "b",
            "name": "Call",
            "type": "n8n-nodes-base.httpRequest",
            "parameters": {"url": "https://api.acme.test/v1", "path": "/private"}
        }]});
        assert_eq!(
            restore_masked(&mut request, Some(&before)),
            Err(Denial::MaskedValueWithoutOriginal)
        );
    }

    #[test]
    fn odoo_values_are_restored_by_field_whatever_the_field_is_called() {
        let mut request = ActionRequest::OdooWrite {
            model: model("res.partner"),
            ids: vec![1],
            values: json!({"x_note": MASK, "totp_secret": MASK})
                .as_object()
                .unwrap()
                .clone(),
        };
        let before = json!([{"id": 1, "x_note": "memo", "totp_secret": "JBSW"}]);
        restore_masked(&mut request, Some(&before)).unwrap();
        let ActionRequest::OdooWrite { values, .. } = request else {
            unreachable!()
        };
        assert_eq!(values["x_note"], "memo");
        assert_eq!(values["totp_secret"], "JBSW");
    }

    #[test]
    fn the_extra_secret_keys_are_masked() {
        for key in [
            "oauth_access_token",
            "totp_secret",
            "new_password",
            "password_crypt",
            "OAuth-Access-Token",
        ] {
            assert!(is_secret_key(key), "{key}");
            assert_eq!(redact(&json!({key: "s3cret"}))[key], MASK, "{key}");
        }
    }
}
