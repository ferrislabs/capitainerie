use std::{sync::LazyLock, time::Duration};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use capitainerie_core::domain::integration::ConnectorFailure;
use capitainerie_core::domain::integration::action::{ATTACHMENT_MAX_BYTES, ATTACHMENT_MAX_CHARS};
use serde_json::{Map, Value, json};
use tokio::sync::Semaphore;

use crate::odoo::{Arg, OdooCall, Transport};

const MODEL: &str = "ir.attachment";
const SUMMARY_FIELDS: [&str; 6] = [
    "name",
    "mimetype",
    "file_size",
    "res_model",
    "res_id",
    "type",
];
const KEPT_FIELDS: [&str; 5] = ["name", "mimetype", "file_size", "res_model", "res_id"];
const CONTENT_FIELD: [&str; 1] = ["datas"];
const PDF_TIMEOUT: Duration = Duration::from_secs(20);
const PDF_WAIT: Duration = Duration::from_secs(5);
const PDF_EXTRACTIONS: usize = 2;

static PDF_SLOTS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(PDF_EXTRACTIONS));

const NOTE_LINK: &str = "this attachment is a link, not a stored file";
const NOTE_OTHER_TYPE: &str =
    "this type of file has no text rendering: only its metadata is returned";
const NOTE_TOO_LARGE: &str = "the attachment exceeds 7 MiB";
const NOTE_EMPTY: &str = "the attachment has no content";
const NOTE_PDF_UNREADABLE: &str = "the PDF text could not be extracted";
const NOTE_PDF_SCANNED: &str = "the PDF has no text layer";
const NOTE_PDF_BUSY: &str = "too many PDFs are being read right now: try again in a moment";

#[derive(Clone, Copy)]
enum Rendering {
    Text,
    Pdf,
}

enum Rendered {
    Text(String),
    Note(&'static str),
}

fn rendering(mimetype: &str) -> Option<Rendering> {
    let essence = mimetype
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if essence == "application/pdf" {
        Some(Rendering::Pdf)
    } else if essence.starts_with("text/")
        || essence == "application/json"
        || essence == "application/xml"
        || essence.ends_with("+json")
        || essence.ends_with("+xml")
    {
        Some(Rendering::Text)
    } else {
        None
    }
}

fn first_record(answer: Value) -> Result<Option<Map<String, Value>>, ConnectorFailure> {
    match answer {
        Value::Array(items) => match items.into_iter().next() {
            Some(Value::Object(record)) => Ok(Some(record)),
            Some(_) => Err(unexpected()),
            None => Ok(None),
        },
        _ => Err(unexpected()),
    }
}

fn unexpected() -> ConnectorFailure {
    ConnectorFailure::Protocol("the server answered something unexpected for the attachment".into())
}

fn with_note(mut summary: Map<String, Value>, note: &str) -> Value {
    summary.insert("note".into(), Value::from(note));
    Value::Object(summary)
}

fn clip(text: &str) -> (&str, bool) {
    match text.char_indices().nth(ATTACHMENT_MAX_CHARS) {
        Some((end, _)) => (&text[..end], true),
        None => (text, false),
    }
}

fn decode(encoded: &str) -> Result<Vec<u8>, ConnectorFailure> {
    let compact: Vec<u8> = encoded
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    STANDARD.decode(compact).map_err(|_| {
        ConnectorFailure::Protocol("the attachment content is not valid base64".into())
    })
}

async fn pdf_text(bytes: Vec<u8>) -> Rendered {
    let Ok(Ok(slot)) = tokio::time::timeout(PDF_WAIT, PDF_SLOTS.acquire()).await else {
        return Rendered::Note(NOTE_PDF_BUSY);
    };
    let extraction = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        pdf_extract::extract_text_from_mem(&bytes)
    });
    match tokio::time::timeout(PDF_TIMEOUT, extraction).await {
        Ok(Ok(Ok(text))) if text.trim().is_empty() => Rendered::Note(NOTE_PDF_SCANNED),
        Ok(Ok(Ok(text))) => Rendered::Text(text),
        _ => Rendered::Note(NOTE_PDF_UNREADABLE),
    }
}

pub(crate) async fn read<T: Transport + Sync>(
    transport: &T,
    attachment_id: i64,
) -> Result<Value, ConnectorFailure> {
    let ids = std::slice::from_ref(&attachment_id);
    let found = transport
        .execute(
            &OdooCall::new(MODEL, "read")
                .ids(ids)
                .keyword("fields", Arg::Labels(&SUMMARY_FIELDS)),
        )
        .await?;
    let Some(mut record) = first_record(found)? else {
        return Err(ConnectorFailure::Remote {
            status: 404,
            message: "attachment not found".into(),
        });
    };
    let mut summary = Map::new();
    summary.insert("id".into(), Value::from(attachment_id));
    for field in KEPT_FIELDS {
        summary.insert(field.into(), record.remove(field).unwrap_or(Value::Null));
    }
    if record.get("type").and_then(Value::as_str) == Some("url") {
        return Ok(with_note(summary, NOTE_LINK));
    }
    let Some(rendering) = summary
        .get("mimetype")
        .and_then(Value::as_str)
        .and_then(rendering)
    else {
        return Ok(with_note(summary, NOTE_OTHER_TYPE));
    };
    let too_large = summary
        .get("file_size")
        .and_then(Value::as_u64)
        .is_some_and(|size| size > ATTACHMENT_MAX_BYTES as u64);
    if too_large {
        return Ok(with_note(summary, NOTE_TOO_LARGE));
    }
    let content = transport
        .execute(
            &OdooCall::new(MODEL, "read")
                .ids(ids)
                .keyword("fields", Arg::Labels(&CONTENT_FIELD)),
        )
        .await?;
    let Some(mut record) = first_record(content)? else {
        return Err(ConnectorFailure::Remote {
            status: 404,
            message: "attachment not found".into(),
        });
    };
    let Some(Value::String(encoded)) = record.remove("datas") else {
        return Ok(with_note(summary, NOTE_EMPTY));
    };
    let bytes = decode(&encoded)?;
    let rendered = match rendering {
        Rendering::Text => Rendered::Text(String::from_utf8_lossy(&bytes).into_owned()),
        Rendering::Pdf => pdf_text(bytes).await,
    };
    Ok(match rendered {
        Rendered::Note(note) => with_note(summary, note),
        Rendered::Text(text) => {
            let (kept, truncated) = clip(&text);
            summary.insert("text".into(), json!(kept));
            summary.insert("truncated".into(), json!(truncated));
            Value::Object(summary)
        }
    })
}
