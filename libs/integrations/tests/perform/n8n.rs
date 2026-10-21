use capitainerie_core::domain::integration::{
    ConnectorFailure,
    action::{ActionRequest, ExecutionStatus},
};
use serde_json::{Value, json};

use crate::support::{Fake, Reply, SECRET, array, id, n8n, object, perform, serve};

async fn instance(status: u16, body: Value) -> Fake {
    serve(move |_| Reply::json(status, &body)).await
}

fn list_workflows() -> ActionRequest {
    ActionRequest::N8nListWorkflows {
        name: None,
        tags: None,
        active: None,
        project_id: None,
        limit: 20,
        cursor: None,
    }
}

fn update(publish: bool) -> ActionRequest {
    ActionRequest::N8nUpdateWorkflow {
        workflow_id: id("wf1"),
        name: Some("Sync".into()),
        nodes: array(json!([{"name": "Start", "type": "n8n-nodes-base.start"}])),
        connections: object(json!({})),
        settings: Some(object(json!({"executionOrder": "v1"}))),
        publish,
    }
}

#[tokio::test]
async fn a_workflow_list_sends_the_key_and_returns_the_page() {
    let page = json!({"data": [{"id": "wf1", "name": "Sync"}], "nextCursor": null});
    let fake = instance(200, page.clone()).await;
    assert_eq!(perform(&n8n(&fake.base), &list_workflows()).await, Ok(page));
    let call = fake.only();
    assert_eq!(call.method, "GET");
    assert_eq!(call.path, "/api/v1/workflows?limit=20");
    assert_eq!(call.header("x-n8n-api-key"), Some(SECRET));
    assert_eq!(call.header("accept"), Some("application/json"));
    assert_eq!(call.body, "");
}

#[tokio::test]
async fn the_workflow_filters_are_url_encoded_and_only_sent_when_set() {
    let fake = instance(200, json!({"data": []})).await;
    let request = ActionRequest::N8nListWorkflows {
        name: Some("a b&c".into()),
        tags: Some("x,y".into()),
        active: Some(true),
        project_id: Some(id("proj-1")),
        limit: 100,
        cursor: Some("abc=".into()),
    };
    perform(&n8n(&fake.base), &request).await.unwrap();
    assert_eq!(
        fake.only().path,
        "/api/v1/workflows?name=a+b%26c&tags=x%2Cy&active=true&projectId=proj-1&limit=100&cursor=abc%3D"
    );
}

#[tokio::test]
async fn reads_use_their_own_paths_and_queries() {
    let cases = [
        (
            ActionRequest::N8nGetWorkflow {
                workflow_id: id("wf1"),
            },
            "/api/v1/workflows/wf1",
        ),
        (
            ActionRequest::N8nListWorkflowVersions {
                workflow_id: id("wf1"),
            },
            "/api/v1/workflows/wf1/history",
        ),
        (
            ActionRequest::N8nGetWorkflowTags {
                workflow_id: id("wf1"),
            },
            "/api/v1/workflows/wf1/tags",
        ),
        (
            ActionRequest::N8nListExecutions {
                workflow_id: Some(id("wf1")),
                status: Some(ExecutionStatus::Error),
                limit: 5,
                cursor: Some("next".into()),
            },
            "/api/v1/executions?workflowId=wf1&status=error&limit=5&cursor=next",
        ),
        (
            ActionRequest::N8nListExecutions {
                workflow_id: None,
                status: None,
                limit: 20,
                cursor: None,
            },
            "/api/v1/executions?limit=20",
        ),
        (
            ActionRequest::N8nGetExecution {
                execution_id: id("e1"),
                include_data: true,
            },
            "/api/v1/executions/e1?includeData=true",
        ),
        (
            ActionRequest::N8nGetExecution {
                execution_id: id("e1"),
                include_data: false,
            },
            "/api/v1/executions/e1?includeData=false",
        ),
        (
            ActionRequest::N8nListTags {
                limit: 20,
                cursor: None,
            },
            "/api/v1/tags?limit=20",
        ),
    ];
    for (request, path) in cases {
        let answer = json!({"id": "x", "data": []});
        let fake = instance(200, answer.clone()).await;
        assert_eq!(perform(&n8n(&fake.base), &request).await, Ok(answer));
        let call = fake.only();
        assert_eq!((call.method.as_str(), call.path.as_str()), ("GET", path));
    }
}

#[tokio::test]
async fn an_update_puts_the_full_workflow_and_keeps_it_unpublished_by_default() {
    let fake = instance(200, json!({"id": "wf1", "active": false})).await;
    perform(&n8n(&fake.base), &update(false)).await.unwrap();
    let call = fake.only();
    assert_eq!(call.method, "PUT");
    assert_eq!(call.path, "/api/v1/workflows/wf1?publishIfActive=false");
    assert_eq!(
        call.json(),
        json!({
            "name": "Sync",
            "nodes": [{"name": "Start", "type": "n8n-nodes-base.start"}],
            "connections": {},
            "settings": {"executionOrder": "v1"}
        })
    );

    let fake = instance(200, json!({})).await;
    perform(&n8n(&fake.base), &update(true)).await.unwrap();
    assert_eq!(
        fake.only().path,
        "/api/v1/workflows/wf1?publishIfActive=true"
    );
}

#[tokio::test]
async fn an_update_without_the_name_or_the_settings_is_refused_before_any_request() {
    let fake = instance(200, json!({})).await;
    for (name, settings) in [
        (None, Some(object(json!({})))),
        (Some("Sync".to_owned()), None),
        (None, None),
    ] {
        let request = ActionRequest::N8nUpdateWorkflow {
            workflow_id: id("wf1"),
            name,
            nodes: Vec::new(),
            connections: object(json!({})),
            settings,
            publish: false,
        };
        assert_eq!(
            perform(&n8n(&fake.base), &request).await,
            Err(ConnectorFailure::Protocol(
                "the update needs the full workflow: name and settings".into()
            ))
        );
    }
    assert_eq!(fake.hits(), 0);
}

#[tokio::test]
async fn a_create_posts_the_workflow() {
    let fake = instance(200, json!({"id": "wf9"})).await;
    let request = ActionRequest::N8nCreateWorkflow {
        name: "New".into(),
        nodes: array(json!([{"name": "Start"}])),
        connections: object(json!({"Start": {"main": []}})),
        settings: object(json!({})),
    };
    assert_eq!(
        perform(&n8n(&fake.base), &request).await,
        Ok(json!({"id": "wf9"}))
    );
    let call = fake.only();
    assert_eq!(
        (call.method.as_str(), call.path.as_str()),
        ("POST", "/api/v1/workflows")
    );
    assert_eq!(
        call.json(),
        json!({
            "name": "New",
            "nodes": [{"name": "Start"}],
            "connections": {"Start": {"main": []}},
            "settings": {}
        })
    );
}

#[tokio::test]
async fn publishing_posts_to_publish() {
    let fake = instance(200, json!({"id": "wf1"})).await;
    let request = ActionRequest::N8nPublishWorkflow {
        workflow_id: id("wf1"),
    };
    perform(&n8n(&fake.base), &request).await.unwrap();
    assert_eq!(fake.paths(), ["POST /api/v1/workflows/wf1/publish"]);
}

#[tokio::test]
async fn publishing_falls_back_to_activate_on_a_404() {
    let fake = serve(|recorded| {
        if recorded.path.ends_with("/activate") {
            Reply::json(200, &json!({"id": "wf1", "active": true}))
        } else {
            Reply::status(404)
        }
    })
    .await;
    let request = ActionRequest::N8nPublishWorkflow {
        workflow_id: id("wf1"),
    };
    assert_eq!(
        perform(&n8n(&fake.base), &request).await,
        Ok(json!({"id": "wf1", "active": true}))
    );
    assert_eq!(
        fake.paths(),
        [
            "POST /api/v1/workflows/wf1/publish",
            "POST /api/v1/workflows/wf1/activate"
        ]
    );
}

#[tokio::test]
async fn unpublishing_falls_back_to_deactivate_on_a_404() {
    let fake = serve(|recorded| {
        if recorded.path.ends_with("/deactivate") {
            Reply::json(200, &json!({"id": "wf1", "active": false}))
        } else {
            Reply::status(404)
        }
    })
    .await;
    let request = ActionRequest::N8nUnpublishWorkflow {
        workflow_id: id("wf1"),
    };
    assert_eq!(
        perform(&n8n(&fake.base), &request).await,
        Ok(json!({"id": "wf1", "active": false}))
    );
    assert_eq!(
        fake.paths(),
        [
            "POST /api/v1/workflows/wf1/unpublish",
            "POST /api/v1/workflows/wf1/deactivate"
        ]
    );
}

#[tokio::test]
async fn a_404_on_both_routes_is_a_remote_404() {
    let fake = serve(|_| Reply::json(404, &json!({"message": "Workflow not found"}))).await;
    let request = ActionRequest::N8nPublishWorkflow {
        workflow_id: id("wf1"),
    };
    assert_eq!(
        perform(&n8n(&fake.base), &request).await,
        Err(ConnectorFailure::Remote {
            status: 404,
            message: "Workflow not found".into()
        })
    );
    assert_eq!(fake.hits(), 2);
}

#[tokio::test]
async fn another_failure_on_publish_does_not_fall_back() {
    let fake = serve(|_| Reply::status(500)).await;
    let request = ActionRequest::N8nUnpublishWorkflow {
        workflow_id: id("wf1"),
    };
    assert!(matches!(
        perform(&n8n(&fake.base), &request).await,
        Err(ConnectorFailure::Remote { status: 500, .. })
    ));
    assert_eq!(fake.hits(), 1);
}

#[tokio::test]
async fn archiving_needs_n8n_2() {
    let fake = instance(200, json!({"id": "wf1", "isArchived": true})).await;
    let request = ActionRequest::N8nArchiveWorkflow {
        workflow_id: id("wf1"),
    };
    perform(&n8n(&fake.base), &request).await.unwrap();
    assert_eq!(fake.paths(), ["POST /api/v1/workflows/wf1/archive"]);

    let request = ActionRequest::N8nUnarchiveWorkflow {
        workflow_id: id("wf1"),
    };
    perform(&n8n(&fake.base), &request).await.unwrap();
    assert_eq!(fake.paths()[1], "POST /api/v1/workflows/wf1/unarchive");

    let missing = serve(|_| Reply::status(404)).await;
    let archive = ActionRequest::N8nArchiveWorkflow {
        workflow_id: id("wf1"),
    };
    assert_eq!(
        perform(&n8n(&missing.base), &archive).await,
        Err(ConnectorFailure::NotSupported(
            "archiving needs n8n 2.x".into()
        ))
    );
    let unarchive = ActionRequest::N8nUnarchiveWorkflow {
        workflow_id: id("wf1"),
    };
    assert_eq!(
        perform(&n8n(&missing.base), &unarchive).await,
        Err(ConnectorFailure::NotSupported(
            "archiving needs n8n 2.x".into()
        ))
    );
}

#[tokio::test]
async fn the_workflow_versions_need_n8n_2() {
    let fake = serve(|_| Reply::status(404)).await;
    let request = ActionRequest::N8nListWorkflowVersions {
        workflow_id: id("wf1"),
    };
    assert_eq!(
        perform(&n8n(&fake.base), &request).await,
        Err(ConnectorFailure::NotSupported(
            "workflow versions need n8n 2.x".into()
        ))
    );
}

#[tokio::test]
async fn the_credentials_list_keeps_only_the_metadata() {
    let fake = instance(
        200,
        json!({
            "data": [
                {
                    "id": "c1",
                    "name": "Odoo prod",
                    "type": "odooApi",
                    "createdAt": "2026-01-01T00:00:00.000Z",
                    "updatedAt": "2026-02-01T00:00:00.000Z",
                    "isManaged": false,
                    "data": {"apiKey": "hunter2"},
                    "apiKey": "hunter2"
                },
                {"id": "c2", "name": "Slack", "type": "slackApi", "password": "hunter2"}
            ],
            "nextCursor": "c3",
            "debug": {"token": "hunter2"}
        }),
    )
    .await;
    let request = ActionRequest::N8nListCredentials {
        limit: 20,
        cursor: None,
    };
    let answer = perform(&n8n(&fake.base), &request).await.unwrap();
    assert_eq!(
        answer,
        json!({
            "data": [
                {
                    "id": "c1",
                    "name": "Odoo prod",
                    "type": "odooApi",
                    "createdAt": "2026-01-01T00:00:00.000Z",
                    "updatedAt": "2026-02-01T00:00:00.000Z"
                },
                {"id": "c2", "name": "Slack", "type": "slackApi"}
            ],
            "nextCursor": "c3"
        })
    );
    assert!(!answer.to_string().contains("hunter2"));
    assert_eq!(fake.only().path, "/api/v1/credentials?limit=20");
}

#[tokio::test]
async fn a_credentials_answer_of_an_unexpected_shape_is_not_forwarded() {
    for body in [
        json!([{"id": "c1", "data": "secret"}]),
        json!({"items": []}),
    ] {
        let fake = instance(200, body).await;
        let request = ActionRequest::N8nListCredentials {
            limit: 20,
            cursor: None,
        };
        assert!(matches!(
            perform(&n8n(&fake.base), &request).await,
            Err(ConnectorFailure::Protocol(_))
        ));
    }
}

#[tokio::test]
async fn the_tags_of_a_workflow_are_replaced_by_ids() {
    let fake = instance(200, json!([{"id": "t1"}, {"id": "t2"}])).await;
    let request = ActionRequest::N8nSetWorkflowTags {
        workflow_id: id("wf1"),
        tag_ids: vec![id("t1"), id("t2")],
    };
    perform(&n8n(&fake.base), &request).await.unwrap();
    let call = fake.only();
    assert_eq!(
        (call.method.as_str(), call.path.as_str()),
        ("PUT", "/api/v1/workflows/wf1/tags")
    );
    assert_eq!(call.json(), json!([{"id": "t1"}, {"id": "t2"}]));
}

#[tokio::test]
async fn a_tag_is_created_then_deleted() {
    let fake = instance(201, json!({"id": "t9", "name": "urgent"})).await;
    let request = ActionRequest::N8nCreateTag {
        name: "urgent".into(),
    };
    assert_eq!(
        perform(&n8n(&fake.base), &request).await,
        Ok(json!({"id": "t9", "name": "urgent"}))
    );
    let call = fake.only();
    assert_eq!(
        (call.method.as_str(), call.path.as_str()),
        ("POST", "/api/v1/tags")
    );
    assert_eq!(call.json(), json!({"name": "urgent"}));

    let fake = serve(|_| Reply::status(204)).await;
    let request = ActionRequest::N8nDeleteTag { tag_id: id("t9") };
    assert_eq!(perform(&n8n(&fake.base), &request).await, Ok(Value::Null));
    assert_eq!(fake.paths(), ["DELETE /api/v1/tags/t9"]);
}

#[tokio::test]
async fn an_execution_is_retried_or_stopped() {
    let fake = instance(200, json!({"id": "e2"})).await;
    let request = ActionRequest::N8nRetryExecution {
        execution_id: id("e1"),
    };
    perform(&n8n(&fake.base), &request).await.unwrap();
    let call = fake.only();
    assert_eq!(
        (call.method.as_str(), call.path.as_str()),
        ("POST", "/api/v1/executions/e1/retry")
    );
    assert_eq!(call.json(), json!({}));

    let fake = instance(200, json!({"status": "canceled"})).await;
    let request = ActionRequest::N8nStopExecution {
        execution_id: id("e1"),
    };
    perform(&n8n(&fake.base), &request).await.unwrap();
    assert_eq!(fake.paths(), ["POST /api/v1/executions/e1/stop"]);
}

#[tokio::test]
async fn refused_credentials_are_unauthorized() {
    for status in [401, 403] {
        let fake = serve(move |_| Reply::status(status)).await;
        assert_eq!(
            perform(&n8n(&fake.base), &list_workflows()).await,
            Err(ConnectorFailure::Unauthorized)
        );
    }
}

#[tokio::test]
async fn another_status_is_a_remote_failure_without_the_secret() {
    let fake = serve(|_| {
        Reply::json(
            400,
            &json!({"message": format!("request/body must have {SECRET}")}),
        )
    })
    .await;
    assert_eq!(
        perform(&n8n(&fake.base), &list_workflows()).await,
        Err(ConnectorFailure::Remote {
            status: 400,
            message: "request/body must have ***".into()
        })
    );
}

#[tokio::test]
async fn an_empty_answer_is_null_and_a_non_json_answer_is_a_protocol_failure() {
    let fake = serve(|_| Reply::status(200)).await;
    assert_eq!(
        perform(&n8n(&fake.base), &list_workflows()).await,
        Ok(Value::Null)
    );
    let fake = serve(|_| Reply::text(200, "<html>nope</html>")).await;
    assert!(matches!(
        perform(&n8n(&fake.base), &list_workflows()).await,
        Err(ConnectorFailure::Protocol(_))
    ));
}
