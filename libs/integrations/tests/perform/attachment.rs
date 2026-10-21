use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use capitainerie_core::domain::integration::{ConnectorFailure, action::ActionRequest};
use serde_json::{Value, json};

use crate::support::{Fake, Reply, json2, jsonrpc, perform, serve};

fn encoded(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

fn request() -> ActionRequest {
    ActionRequest::OdooReadAttachment { attachment_id: 5 }
}

fn meta(mimetype: &str, file_size: usize) -> Value {
    json!({
        "id": 5,
        "name": "notes.txt",
        "mimetype": mimetype,
        "file_size": file_size,
        "res_model": "res.partner",
        "res_id": 3,
        "type": "binary"
    })
}

fn summary(mimetype: &str, file_size: usize) -> Value {
    json!({
        "id": 5,
        "name": "notes.txt",
        "mimetype": mimetype,
        "file_size": file_size,
        "res_model": "res.partner",
        "res_id": 3
    })
}

fn with(mut base: Value, extra: Value) -> Value {
    if let (Some(base), Value::Object(extra)) = (base.as_object_mut(), extra) {
        base.extend(extra);
    }
    base
}

async fn odoo(meta: Value, datas: Value) -> Fake {
    serve(move |recorded| {
        if recorded.path != "/json/2/ir.attachment/read" {
            return Reply::status(404);
        }
        if recorded.json()["fields"] == json!(["datas"]) {
            Reply::json(200, &json!([{"id": 5, "datas": datas}]))
        } else {
            Reply::json(200, &json!([meta]))
        }
    })
    .await
}

async fn read(meta: Value, datas: Value) -> (Result<Value, ConnectorFailure>, Fake) {
    let fake = odoo(meta, datas).await;
    let outcome = perform(&json2(&fake.base), &request()).await;
    (outcome, fake)
}

fn pdf(text: &str) -> Vec<u8> {
    let content = if text.is_empty() {
        String::new()
    } else {
        format!("BT /F1 24 Tf 72 700 Td ({text}) Tj ET")
    };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{body}\nendobj\n", index + 1).bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .bytes(),
    );
    out
}

#[tokio::test]
async fn a_text_attachment_is_returned_as_text_and_never_as_datas() {
    let text = "hello world\n";
    let datas = encoded(text.as_bytes());
    let (outcome, fake) = read(meta("text/plain", text.len()), json!(datas)).await;
    let answer = outcome.unwrap();
    assert_eq!(
        answer,
        with(
            summary("text/plain", text.len()),
            json!({"text": text, "truncated": false})
        )
    );
    assert!(answer.get("datas").is_none());
    assert!(!answer.to_string().contains(&datas));
    let calls = fake.requests();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].json(),
        json!({
            "ids": [5],
            "fields": ["name", "mimetype", "file_size", "res_model", "res_id", "type"]
        })
    );
    assert_eq!(calls[1].json(), json!({"ids": [5], "fields": ["datas"]}));
}

#[tokio::test]
async fn a_long_text_is_cut_at_100000_characters() {
    let long = "é".repeat(100_001);
    let (outcome, _) = read(
        meta("text/plain", long.len()),
        json!(encoded(long.as_bytes())),
    )
    .await;
    let answer = outcome.unwrap();
    assert_eq!(answer["truncated"], true);
    assert_eq!(answer["text"].as_str().unwrap().chars().count(), 100_000);

    let exact = "é".repeat(100_000);
    let (outcome, _) = read(
        meta("text/plain", exact.len()),
        json!(encoded(exact.as_bytes())),
    )
    .await;
    let answer = outcome.unwrap();
    assert_eq!(answer["truncated"], false);
    assert_eq!(answer["text"].as_str().unwrap().chars().count(), 100_000);
}

#[tokio::test]
async fn every_textual_mimetype_is_rendered_as_text() {
    for mimetype in [
        "text/plain",
        "text/csv",
        "text/html",
        "application/json",
        "application/xml",
        "application/ld+json",
        "application/atom+xml",
        "application/json; charset=utf-8",
        "TEXT/Plain",
    ] {
        let (outcome, _) = read(meta(mimetype, 3), json!(encoded(b"a,b"))).await;
        let answer = outcome.unwrap();
        assert_eq!(answer["text"], "a,b", "{mimetype}");
        assert_eq!(answer["truncated"], false, "{mimetype}");
    }
}

#[tokio::test]
async fn invalid_utf8_is_replaced_not_refused() {
    let (outcome, _) = read(meta("text/plain", 3), json!(encoded(&[b'a', 0xff, b'b']))).await;
    assert_eq!(outcome.unwrap()["text"], "a\u{fffd}b");
}

#[tokio::test]
async fn other_types_return_the_metadata_and_a_note_without_downloading() {
    for mimetype in ["image/png", "application/zip", "application/octet-stream"] {
        let (outcome, fake) = read(meta(mimetype, 900), json!(encoded(b"\x89PNG"))).await;
        let answer = outcome.unwrap();
        assert_eq!(
            answer,
            with(
                summary(mimetype, 900),
                json!({"note": "this type of file has no text rendering: only its metadata is returned"})
            )
        );
        assert_eq!(fake.hits(), 1, "{mimetype}");
    }

    let unknown = with(meta("image/png", 900), json!({"mimetype": false}));
    let (outcome, fake) = read(unknown, json!(null)).await;
    let answer = outcome.unwrap();
    assert_eq!(answer["mimetype"], false);
    assert!(answer["note"].is_string());
    assert!(answer.get("text").is_none());
    assert_eq!(fake.hits(), 1);
}

#[tokio::test]
async fn a_link_attachment_is_not_downloaded() {
    let link = with(meta("text/html", 0), json!({"type": "url", "name": "Spec"}));
    let (outcome, fake) = read(link, json!(null)).await;
    let answer = outcome.unwrap();
    assert_eq!(
        answer["note"],
        "this attachment is a link, not a stored file"
    );
    assert!(answer.get("text").is_none());
    assert_eq!(fake.hits(), 1);
}

#[tokio::test]
async fn an_attachment_over_7_mib_is_not_downloaded() {
    let size = 7 * 1024 * 1024 + 1;
    let (outcome, fake) = read(meta("text/plain", size), json!(encoded(b"never read"))).await;
    assert_eq!(
        outcome.unwrap(),
        with(
            summary("text/plain", size),
            json!({"note": "the attachment exceeds 7 MiB"})
        )
    );
    assert_eq!(fake.hits(), 1);

    let size = 7 * 1024 * 1024;
    let (outcome, fake) = read(meta("application/pdf", size), json!(encoded(&pdf("ok")))).await;
    assert!(outcome.unwrap().get("text").is_some());
    assert_eq!(fake.hits(), 2);
}

#[tokio::test]
async fn a_pdf_is_read_through_its_text_layer() {
    let file = pdf("Hello attachment");
    let datas = encoded(&file);
    let (outcome, _) = read(meta("application/pdf", file.len()), json!(datas)).await;
    let answer = outcome.unwrap();
    assert!(
        answer["text"]
            .as_str()
            .unwrap()
            .contains("Hello attachment"),
        "{answer}"
    );
    assert_eq!(answer["truncated"], false);
    assert!(answer.get("datas").is_none());
    assert!(!answer.to_string().contains(&datas));
}

#[tokio::test]
async fn a_pdf_without_a_text_layer_gets_a_note() {
    let file = pdf("");
    let (outcome, _) = read(meta("application/pdf", file.len()), json!(encoded(&file))).await;
    let answer = outcome.unwrap();
    assert_eq!(answer["note"], "the PDF has no text layer");
    assert!(answer.get("text").is_none());
}

#[tokio::test]
async fn a_broken_pdf_gets_a_note_instead_of_failing() {
    let garbage = b"%PDF-1.4 this is not a pdf at all";
    let (outcome, _) = read(
        meta("application/pdf", garbage.len()),
        json!(encoded(garbage)),
    )
    .await;
    let answer = outcome.unwrap();
    assert_eq!(answer["note"], "the PDF text could not be extracted");
    assert!(answer.get("text").is_none());
}

#[tokio::test]
async fn a_missing_attachment_is_a_remote_404() {
    let fake = serve(|_| Reply::json(200, &json!([]))).await;
    assert_eq!(
        perform(&json2(&fake.base), &request()).await,
        Err(ConnectorFailure::Remote {
            status: 404,
            message: "attachment not found".into()
        })
    );
    assert_eq!(fake.hits(), 1);
}

#[tokio::test]
async fn an_attachment_without_content_gets_a_note() {
    let (outcome, _) = read(meta("text/plain", 0), json!(false)).await;
    let answer = outcome.unwrap();
    assert_eq!(answer["note"], "the attachment has no content");
    assert!(answer.get("text").is_none());
}

#[tokio::test]
async fn content_that_is_not_base64_is_a_protocol_failure() {
    let (outcome, _) = read(meta("text/plain", 5), json!("not base64 !!")).await;
    assert!(matches!(outcome, Err(ConnectorFailure::Protocol(_))));
}

#[tokio::test]
async fn the_attachment_is_read_the_same_way_through_json_rpc() {
    let text = "from rpc";
    let datas = encoded(text.as_bytes());
    let fake = serve(move |recorded| {
        let body = recorded.json();
        match body["params"]["method"].as_str() {
            Some("authenticate") => Reply::json(200, &json!({"result": 7})),
            Some("execute_kw") if body["params"]["args"][6]["fields"] == json!(["datas"]) => {
                Reply::json(200, &json!({"result": [{"id": 5, "datas": datas}]}))
            }
            Some("execute_kw") => Reply::json(200, &json!({"result": [meta("text/plain", 8)]})),
            _ => Reply::status(400),
        }
    })
    .await;
    assert_eq!(
        perform(&jsonrpc(&fake.base), &request()).await,
        Ok(with(
            summary("text/plain", 8),
            json!({"text": text, "truncated": false})
        ))
    );
    let reads = fake.rpc_calls("execute_kw");
    assert_eq!(reads.len(), 2);
    assert_eq!(
        reads[0].json()["params"]["args"],
        json!([
            "prod",
            7,
            crate::support::SECRET,
            "ir.attachment",
            "read",
            [[5]],
            {"fields": ["name", "mimetype", "file_size", "res_model", "res_id", "type"]}
        ])
    );
    assert_eq!(fake.rpc_calls("authenticate").len(), 1);
}
