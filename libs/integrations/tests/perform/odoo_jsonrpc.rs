use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

use capitainerie_core::domain::integration::{
    ConnectorFailure, IntegrationKind, action::ActionRequest,
};
use serde_json::{Value, json};

use crate::support::{
    Fake, Reply, SECRET, array, connection, connector, jsonrpc, method, model, object, perform,
    perform_with, serve,
};

fn rpc(result: Value) -> Reply {
    Reply::json(200, &json!({"jsonrpc": "2.0", "id": 1, "result": result}))
}

fn rpc_error(name: &str, message: &str) -> Reply {
    Reply::json(
        200,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": 200,
                "message": "Odoo Server Error",
                "data": {"name": name, "message": message}
            }
        }),
    )
}

async fn odoo(result: Value) -> Fake {
    serve(
        move |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("authenticate") => rpc(json!(7)),
            Some("execute_kw") => rpc(result.clone()),
            _ => Reply::status(400),
        },
    )
    .await
}

fn execute(uid: i64, model: &str, method: &str, args: Value, kwargs: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "call",
        "params": {
            "service": "object",
            "method": "execute_kw",
            "args": ["prod", uid, SECRET, model, method, args, kwargs]
        },
        "id": 1
    })
}

fn search_read() -> ActionRequest {
    ActionRequest::OdooSearchRead {
        model: model("res.partner"),
        domain: array(json!([["is_company", "=", true]])),
        fields: vec!["name".into()],
        limit: 10,
        offset: 5,
        order: Some("name asc".into()),
    }
}

#[tokio::test]
async fn a_search_read_authenticates_then_calls_execute_kw() {
    let records = json!([{"id": 1, "name": "Acme"}]);
    let fake = odoo(records.clone()).await;
    assert_eq!(
        perform(&jsonrpc(&fake.base), &search_read()).await,
        Ok(records)
    );
    let calls = fake.requests();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].method, "POST");
    assert_eq!(calls[0].path, "/jsonrpc");
    assert_eq!(
        calls[0].json(),
        json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": {
                "service": "common",
                "method": "authenticate",
                "args": ["prod", "admin@acme.test", SECRET, {}]
            },
            "id": 1
        })
    );
    assert_eq!(
        calls[1].json(),
        execute(
            7,
            "res.partner",
            "search_read",
            json!([[["is_company", "=", true]]]),
            json!({"fields": ["name"], "limit": 10, "offset": 5, "order": "name asc"})
        )
    );
}

#[tokio::test]
async fn a_search_read_without_order_sends_no_order() {
    let fake = odoo(json!([])).await;
    let request = ActionRequest::OdooSearchRead {
        model: model("res.partner"),
        domain: Vec::new(),
        fields: vec!["name".into()],
        limit: 50,
        offset: 0,
        order: None,
    };
    perform(&jsonrpc(&fake.base), &request).await.unwrap();
    assert_eq!(
        fake.rpc_calls("execute_kw")[0].json(),
        execute(
            7,
            "res.partner",
            "search_read",
            json!([[]]),
            json!({"fields": ["name"], "limit": 50, "offset": 0})
        )
    );
}

#[tokio::test]
async fn two_calls_authenticate_once() {
    let fake = odoo(json!(3)).await;
    let connector = connector();
    let connection = jsonrpc(&fake.base);
    let request = ActionRequest::OdooCount {
        model: model("res.partner"),
        domain: Vec::new(),
    };
    for _ in 0..2 {
        assert_eq!(
            perform_with(&connector, &connection, &request).await,
            Ok(json!(3))
        );
    }
    assert_eq!(fake.rpc_calls("authenticate").len(), 1);
    assert_eq!(fake.rpc_calls("execute_kw").len(), 2);
}

#[tokio::test]
async fn each_instance_authenticates_for_itself() {
    let fake = odoo(json!(3)).await;
    let connector = connector();
    let request = ActionRequest::OdooCount {
        model: model("res.partner"),
        domain: Vec::new(),
    };
    for _ in 0..2 {
        perform_with(&connector, &jsonrpc(&fake.base), &request)
            .await
            .unwrap();
    }
    assert_eq!(fake.rpc_calls("authenticate").len(), 2);
}

#[tokio::test]
async fn an_access_denied_invalidates_the_session_and_retries_once() {
    let logins = AtomicI64::new(7);
    let uses = AtomicUsize::new(0);
    let fake = serve(move |recorded| {
        let body = recorded.json();
        match body["params"]["method"].as_str() {
            Some("authenticate") => rpc(json!(logins.fetch_add(1, Ordering::SeqCst))),
            Some("execute_kw") if body["params"]["args"][1] == 8 => rpc(json!(4)),
            Some("execute_kw") if uses.fetch_add(1, Ordering::SeqCst) == 0 => rpc(json!(3)),
            Some("execute_kw") => rpc_error("odoo.exceptions.AccessDenied", "Access Denied"),
            _ => Reply::status(400),
        }
    })
    .await;
    let connector = connector();
    let connection = jsonrpc(&fake.base);
    let request = ActionRequest::OdooCount {
        model: model("res.partner"),
        domain: Vec::new(),
    };
    assert_eq!(
        perform_with(&connector, &connection, &request).await,
        Ok(json!(3))
    );
    assert_eq!(
        perform_with(&connector, &connection, &request).await,
        Ok(json!(4))
    );
    assert_eq!(fake.rpc_calls("authenticate").len(), 2);
    let uids: Vec<Value> = fake
        .rpc_calls("execute_kw")
        .iter()
        .map(|call| call.json()["params"]["args"][1].clone())
        .collect();
    assert_eq!(uids, [json!(7), json!(7), json!(8)]);
}

#[tokio::test]
async fn an_access_denied_right_after_a_login_is_not_retried() {
    let fake = serve(
        |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("authenticate") => rpc(json!(7)),
            _ => rpc_error("odoo.exceptions.AccessDenied", "Access Denied"),
        },
    )
    .await;
    assert_eq!(
        perform(&jsonrpc(&fake.base), &search_read()).await,
        Err(ConnectorFailure::Unauthorized)
    );
    assert_eq!(fake.rpc_calls("authenticate").len(), 1);
    assert_eq!(fake.rpc_calls("execute_kw").len(), 1);
}

#[tokio::test]
async fn an_access_denied_that_persists_on_a_cached_session_is_retried_once() {
    let uses = AtomicUsize::new(0);
    let fake = serve(
        move |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("authenticate") => rpc(json!(7)),
            _ if uses.fetch_add(1, Ordering::SeqCst) == 0 => rpc(json!(3)),
            _ => rpc_error("odoo.exceptions.AccessDenied", "Access Denied"),
        },
    )
    .await;
    let connector = connector();
    let connection = jsonrpc(&fake.base);
    assert_eq!(
        perform_with(&connector, &connection, &search_read()).await,
        Ok(json!(3))
    );
    assert_eq!(
        perform_with(&connector, &connection, &search_read()).await,
        Err(ConnectorFailure::Unauthorized)
    );
    assert_eq!(fake.rpc_calls("authenticate").len(), 2);
    assert_eq!(fake.rpc_calls("execute_kw").len(), 3);
    assert_eq!(
        perform_with(&connector, &connection, &search_read()).await,
        Err(ConnectorFailure::Unauthorized)
    );
    assert_eq!(fake.rpc_calls("authenticate").len(), 3);
    assert_eq!(fake.rpc_calls("execute_kw").len(), 4);
}

#[tokio::test]
async fn a_refused_login_is_unauthorized_without_any_call() {
    for login in [json!(false), json!(0)] {
        let fake = serve(move |_| rpc(login.clone())).await;
        assert_eq!(
            perform(&jsonrpc(&fake.base), &search_read()).await,
            Err(ConnectorFailure::Unauthorized)
        );
        assert_eq!(fake.hits(), 1);
    }
}

#[tokio::test]
async fn a_database_and_a_login_are_required_before_any_request() {
    let fake = odoo(json!([])).await;
    for (database, login) in [(None, Some("admin")), (Some("prod"), None), (None, None)] {
        let connection = connection(IntegrationKind::OdooJsonRpc, &fake.base, database, login);
        assert_eq!(
            perform(&connection, &search_read()).await,
            Err(ConnectorFailure::Protocol(
                "database and login are required for JSON-RPC".into()
            ))
        );
    }
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn listing_and_describing_models_use_keyword_arguments() {
    let fake = odoo(json!([])).await;
    let request = ActionRequest::OdooListModels {
        query: Some("res.".into()),
        limit: 20,
    };
    perform(&jsonrpc(&fake.base), &request).await.unwrap();
    assert_eq!(
        fake.rpc_calls("execute_kw")[0].json(),
        execute(
            7,
            "ir.model",
            "search_read",
            json!([[["model", "ilike", "res."]]]),
            json!({"fields": ["model", "name"], "limit": 20})
        )
    );

    let fake = odoo(json!({})).await;
    let request = ActionRequest::OdooDescribeModel {
        model: model("res.partner"),
    };
    perform(&jsonrpc(&fake.base), &request).await.unwrap();
    assert_eq!(
        fake.rpc_calls("execute_kw")[0].json(),
        execute(
            7,
            "res.partner",
            "fields_get",
            json!([]),
            json!({"attributes": ["string", "type", "required", "readonly", "relation", "selection", "help"]})
        )
    );
}

#[tokio::test]
async fn read_count_create_write_and_unlink_use_positional_arguments() {
    let cases = [
        (
            ActionRequest::OdooRead {
                model: model("res.partner"),
                ids: vec![4, 9],
                fields: vec!["name".into()],
            },
            execute(
                7,
                "res.partner",
                "read",
                json!([[4, 9]]),
                json!({"fields": ["name"]}),
            ),
        ),
        (
            ActionRequest::OdooCount {
                model: model("sale.order"),
                domain: array(json!([["state", "=", "sale"]])),
            },
            execute(
                7,
                "sale.order",
                "search_count",
                json!([[["state", "=", "sale"]]]),
                json!({}),
            ),
        ),
        (
            ActionRequest::OdooCreate {
                model: model("res.partner"),
                values: object(json!({"name": "Acme"})),
            },
            execute(
                7,
                "res.partner",
                "create",
                json!([{"name": "Acme"}]),
                json!({}),
            ),
        ),
        (
            ActionRequest::OdooWrite {
                model: model("res.partner"),
                ids: vec![1, 2],
                values: object(json!({"phone": "1"})),
            },
            execute(
                7,
                "res.partner",
                "write",
                json!([[1, 2], {"phone": "1"}]),
                json!({}),
            ),
        ),
        (
            ActionRequest::OdooDelete {
                model: model("res.partner"),
                ids: vec![3],
            },
            execute(7, "res.partner", "unlink", json!([[3]]), json!({})),
        ),
    ];
    for (request, expected) in cases {
        let fake = odoo(json!(true)).await;
        assert_eq!(
            perform(&jsonrpc(&fake.base), &request).await,
            Ok(json!(true))
        );
        assert_eq!(fake.rpc_calls("execute_kw")[0].json(), expected);
    }
}

#[tokio::test]
async fn a_method_call_puts_the_ids_first_when_there_are_some() {
    let fake = odoo(json!(true)).await;
    let request = ActionRequest::OdooCallMethod {
        model: model("sale.order"),
        method: method("action_confirm"),
        ids: vec![7],
        args: vec![json!("now")],
        kwargs: object(json!({"context": {"lang": "fr_FR"}})),
    };
    perform(&jsonrpc(&fake.base), &request).await.unwrap();
    assert_eq!(
        fake.rpc_calls("execute_kw")[0].json(),
        execute(
            7,
            "sale.order",
            "action_confirm",
            json!([[7], "now"]),
            json!({"context": {"lang": "fr_FR"}})
        )
    );

    let fake = odoo(json!(null)).await;
    let request = ActionRequest::OdooCallMethod {
        model: model("res.partner"),
        method: method("find_or_create"),
        ids: Vec::new(),
        args: vec![json!("a@b.test")],
        kwargs: Default::default(),
    };
    assert_eq!(
        perform(&jsonrpc(&fake.base), &request).await,
        Ok(Value::Null)
    );
    assert_eq!(
        fake.rpc_calls("execute_kw")[0].json(),
        execute(
            7,
            "res.partner",
            "find_or_create",
            json!(["a@b.test"]),
            json!({})
        )
    );
}

#[tokio::test]
async fn an_odoo_error_is_a_remote_failure_without_the_secret() {
    let fake = serve(
        |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("authenticate") => rpc(json!(7)),
            _ => rpc_error("odoo.exceptions.UserError", &format!("bad key {SECRET}")),
        },
    )
    .await;
    assert_eq!(
        perform(&jsonrpc(&fake.base), &search_read()).await,
        Err(ConnectorFailure::Remote {
            status: 200,
            message: "bad key ***".into()
        })
    );
    assert_eq!(fake.rpc_calls("authenticate").len(), 1);
}

#[tokio::test]
async fn an_answer_without_result_is_a_protocol_failure() {
    let fake = serve(
        |recorded| match recorded.json()["params"]["method"].as_str() {
            Some("authenticate") => rpc(json!(7)),
            _ => Reply::json(200, &json!({"hello": "world"})),
        },
    )
    .await;
    assert!(matches!(
        perform(&jsonrpc(&fake.base), &search_read()).await,
        Err(ConnectorFailure::Protocol(_))
    ));
}

#[tokio::test]
async fn an_http_refusal_is_unauthorized() {
    let fake = serve(|_| Reply::status(403)).await;
    assert_eq!(
        perform(&jsonrpc(&fake.base), &search_read()).await,
        Err(ConnectorFailure::Unauthorized)
    );
}
