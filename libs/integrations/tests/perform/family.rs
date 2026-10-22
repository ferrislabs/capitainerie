use capitainerie_core::domain::integration::{
    ConnectorFailure, InstanceConnection, action::ActionRequest,
};
use serde_json::json;

use crate::support::{Fake, Reply, id, json2, jsonrpc, model, n8n, perform, serve};

const MESSAGE: &str = "this tool does not apply to this instance";

fn odoo_requests() -> Vec<ActionRequest> {
    vec![
        ActionRequest::OdooCount {
            model: model("res.partner"),
            domain: Vec::new(),
        },
        ActionRequest::OdooReadAttachment { attachment_id: 5 },
    ]
}

fn n8n_requests() -> Vec<ActionRequest> {
    vec![
        ActionRequest::N8nGetWorkflow {
            workflow_id: id("wf1"),
        },
        ActionRequest::N8nDeleteTag { tag_id: id("t1") },
    ]
}

async fn refuses(fake: &Fake, connection: &InstanceConnection, requests: Vec<ActionRequest>) {
    for request in requests {
        assert_eq!(
            perform(connection, &request).await,
            Err(ConnectorFailure::NotSupported(MESSAGE.into())),
            "{request:?}"
        );
    }
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn an_n8n_tool_on_an_odoo_instance_sends_nothing() {
    let fake = serve(|_| Reply::json(200, &json!({}))).await;
    refuses(&fake, &json2(&fake.base), n8n_requests()).await;
    refuses(&fake, &jsonrpc(&fake.base), n8n_requests()).await;
}

#[tokio::test]
async fn an_odoo_tool_on_an_n8n_instance_sends_nothing() {
    let fake = serve(|_| Reply::json(200, &json!({}))).await;
    refuses(&fake, &n8n(&fake.base), odoo_requests()).await;
}
