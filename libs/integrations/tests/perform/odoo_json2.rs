use capitainerie_core::domain::integration::{
    ConnectorFailure, IntegrationKind, action::ActionRequest,
};
use serde_json::{Value, json};

use crate::support::{
    Fake, Reply, SECRET, array, connection, json2, method, model, object, perform, serve,
};

async fn odoo(result: Value) -> Fake {
    serve(move |_| Reply::json(200, &result)).await
}

fn search_read() -> ActionRequest {
    ActionRequest::OdooSearchRead {
        model: model("res.partner"),
        domain: array(json!([["is_company", "=", true]])),
        fields: vec!["name".into(), "email".into()],
        limit: 10,
        offset: 5,
        order: Some("name asc".into()),
    }
}

#[tokio::test]
async fn a_search_read_posts_named_arguments_and_returns_the_records() {
    let records = json!([{"id": 1, "name": "Acme"}, {"id": 2, "name": "Globex"}]);
    let fake = odoo(records.clone()).await;
    assert_eq!(
        perform(&json2(&fake.base), &search_read()).await,
        Ok(records)
    );
    let call = fake.only();
    assert_eq!(call.method, "POST");
    assert_eq!(call.path, "/json/2/res.partner/search_read");
    assert_eq!(
        call.header("authorization"),
        Some(&*format!("bearer {SECRET}"))
    );
    assert_eq!(call.header("x-odoo-database"), Some("prod"));
    assert_eq!(call.header("content-type"), Some("application/json"));
    assert_eq!(
        call.json(),
        json!({
            "domain": [["is_company", "=", true]],
            "fields": ["name", "email"],
            "limit": 10,
            "offset": 5,
            "order": "name asc"
        })
    );
}

#[tokio::test]
async fn a_search_read_without_order_sends_no_order() {
    let fake = odoo(json!([])).await;
    let request = ActionRequest::OdooSearchRead {
        model: model("res.partner"),
        domain: Vec::new(),
        fields: Vec::new(),
        limit: 50,
        offset: 0,
        order: None,
    };
    perform(&json2(&fake.base), &request).await.unwrap();
    assert_eq!(
        fake.only().json(),
        json!({"domain": [], "fields": [], "limit": 50, "offset": 0})
    );
}

#[tokio::test]
async fn no_database_header_is_sent_when_the_instance_has_no_database() {
    let fake = odoo(json!([])).await;
    let connection = connection(IntegrationKind::OdooJson2, &fake.base, None, None);
    perform(&connection, &search_read()).await.unwrap();
    assert_eq!(fake.only().header("x-odoo-database"), None);
}

#[tokio::test]
async fn listing_the_models_searches_ir_model_by_name() {
    let fake = odoo(json!([{"id": 1, "model": "res.partner", "name": "Contact"}])).await;
    let request = ActionRequest::OdooListModels {
        query: Some("res.".into()),
        limit: 20,
    };
    perform(&json2(&fake.base), &request).await.unwrap();
    let call = fake.only();
    assert_eq!(call.path, "/json/2/ir.model/search_read");
    assert_eq!(
        call.json(),
        json!({
            "domain": [["model", "ilike", "res."]],
            "fields": ["model", "name"],
            "limit": 20
        })
    );

    let fake = odoo(json!([])).await;
    let request = ActionRequest::OdooListModels {
        query: None,
        limit: 200,
    };
    perform(&json2(&fake.base), &request).await.unwrap();
    assert_eq!(
        fake.only().json(),
        json!({"domain": [], "fields": ["model", "name"], "limit": 200})
    );
}

#[tokio::test]
async fn describing_a_model_asks_for_the_field_attributes() {
    let description = json!({"name": {"string": "Name", "type": "char", "required": true}});
    let fake = odoo(description.clone()).await;
    let request = ActionRequest::OdooDescribeModel {
        model: model("res.partner"),
    };
    assert_eq!(perform(&json2(&fake.base), &request).await, Ok(description));
    let call = fake.only();
    assert_eq!(call.path, "/json/2/res.partner/fields_get");
    assert_eq!(
        call.json(),
        json!({"attributes": ["string", "type", "required", "readonly", "relation", "selection", "help"]})
    );
}

#[tokio::test]
async fn read_and_count_use_their_own_argument_names() {
    let fake = odoo(json!([{"id": 4}])).await;
    let request = ActionRequest::OdooRead {
        model: model("res.partner"),
        ids: vec![4, 9],
        fields: vec!["name".into()],
    };
    perform(&json2(&fake.base), &request).await.unwrap();
    let call = fake.only();
    assert_eq!(call.path, "/json/2/res.partner/read");
    assert_eq!(call.json(), json!({"ids": [4, 9], "fields": ["name"]}));

    let fake = odoo(json!(12)).await;
    let request = ActionRequest::OdooCount {
        model: model("sale.order"),
        domain: array(json!([["state", "=", "sale"]])),
    };
    assert_eq!(perform(&json2(&fake.base), &request).await, Ok(json!(12)));
    let call = fake.only();
    assert_eq!(call.path, "/json/2/sale.order/search_count");
    assert_eq!(call.json(), json!({"domain": [["state", "=", "sale"]]}));
}

#[tokio::test]
async fn a_create_sends_the_values_as_a_one_item_vals_list() {
    let fake = odoo(json!([42])).await;
    let request = ActionRequest::OdooCreate {
        model: model("res.partner"),
        values: object(json!({"name": "Acme", "is_company": true})),
    };
    assert_eq!(perform(&json2(&fake.base), &request).await, Ok(json!([42])));
    let call = fake.only();
    assert_eq!(call.path, "/json/2/res.partner/create");
    assert_eq!(
        call.json(),
        json!({"vals_list": [{"name": "Acme", "is_company": true}]})
    );
}

#[tokio::test]
async fn a_write_sends_the_ids_and_the_values() {
    let fake = odoo(json!(true)).await;
    let request = ActionRequest::OdooWrite {
        model: model("res.partner"),
        ids: vec![1, 2],
        values: object(json!({"phone": "+32 2 555 01 00"})),
    };
    assert_eq!(perform(&json2(&fake.base), &request).await, Ok(json!(true)));
    let call = fake.only();
    assert_eq!(call.path, "/json/2/res.partner/write");
    assert_eq!(
        call.json(),
        json!({"ids": [1, 2], "vals": {"phone": "+32 2 555 01 00"}})
    );
}

#[tokio::test]
async fn a_delete_unlinks_the_ids() {
    let fake = odoo(json!(true)).await;
    let request = ActionRequest::OdooDelete {
        model: model("res.partner"),
        ids: vec![3],
    };
    perform(&json2(&fake.base), &request).await.unwrap();
    let call = fake.only();
    assert_eq!(call.path, "/json/2/res.partner/unlink");
    assert_eq!(call.json(), json!({"ids": [3]}));
}

#[tokio::test]
async fn a_method_call_sends_the_ids_and_the_keyword_arguments() {
    let fake = odoo(json!(true)).await;
    let request = ActionRequest::OdooCallMethod {
        model: model("sale.order"),
        method: method("action_confirm"),
        ids: vec![7],
        args: Vec::new(),
        kwargs: object(json!({"context": {"lang": "fr_FR"}})),
    };
    assert_eq!(perform(&json2(&fake.base), &request).await, Ok(json!(true)));
    let call = fake.only();
    assert_eq!(call.path, "/json/2/sale.order/action_confirm");
    assert_eq!(
        call.json(),
        json!({"ids": [7], "context": {"lang": "fr_FR"}})
    );

    let fake = odoo(json!(null)).await;
    let request = ActionRequest::OdooCallMethod {
        model: model("res.partner"),
        method: method("find_or_create"),
        ids: Vec::new(),
        args: Vec::new(),
        kwargs: object(json!({"email": "a@b.test"})),
    };
    assert_eq!(perform(&json2(&fake.base), &request).await, Ok(Value::Null));
    assert_eq!(fake.only().json(), json!({"email": "a@b.test"}));
}

#[tokio::test]
async fn positional_arguments_are_refused_before_any_request() {
    let fake = odoo(json!(true)).await;
    let request = ActionRequest::OdooCallMethod {
        model: model("sale.order"),
        method: method("action_confirm"),
        ids: vec![7],
        args: vec![json!(1)],
        kwargs: Default::default(),
    };
    assert_eq!(
        perform(&json2(&fake.base), &request).await,
        Err(ConnectorFailure::NotSupported(
            "positional arguments are not supported by JSON-2: use kwargs".into()
        ))
    );
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn refused_credentials_are_unauthorized() {
    for status in [401, 403] {
        let fake = serve(move |_| Reply::status(status)).await;
        assert_eq!(
            perform(&json2(&fake.base), &search_read()).await,
            Err(ConnectorFailure::Unauthorized)
        );
    }
}

#[tokio::test]
async fn an_unknown_model_or_method_is_a_remote_404() {
    let fake = serve(|_| {
        Reply::json(
            404,
            &json!({"name": "builtins.KeyError", "message": "Unknown model res.nothing"}),
        )
    })
    .await;
    assert_eq!(
        perform(&json2(&fake.base), &search_read()).await,
        Err(ConnectorFailure::Remote {
            status: 404,
            message: "Unknown model res.nothing".into()
        })
    );
}

#[tokio::test]
async fn an_odoo_error_carries_its_message_without_the_secret() {
    for status in [422, 500] {
        let fake = serve(move |_| {
            Reply::json(
                status,
                &json!({
                    "name": "odoo.exceptions.UserError",
                    "message": format!("bad key {SECRET} given"),
                    "debug": "Traceback"
                }),
            )
        })
        .await;
        assert_eq!(
            perform(&json2(&fake.base), &search_read()).await,
            Err(ConnectorFailure::Remote {
                status,
                message: "bad key *** given".into()
            })
        );
    }
}

#[tokio::test]
async fn an_answer_that_is_not_json_is_a_protocol_failure() {
    let fake = serve(|_| Reply::text(200, "<html>nope</html>")).await;
    assert!(matches!(
        perform(&json2(&fake.base), &search_read()).await,
        Err(ConnectorFailure::Protocol(_))
    ));
}
