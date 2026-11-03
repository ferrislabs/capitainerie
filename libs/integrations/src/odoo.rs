use std::future::Future;

use capitainerie_core::domain::integration::{ConnectorFailure, action::ActionRequest};
use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde_json::{Map, Value, json};

use crate::attachment;

const FIELD_ATTRIBUTES: [&str; 7] = [
    "string",
    "type",
    "required",
    "readonly",
    "relation",
    "selection",
    "help",
];
const MODEL_FIELDS: [&str; 2] = ["model", "name"];

pub(crate) const NOT_FOR_THIS_INSTANCE: &str = "this tool does not apply to this instance";

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Arg<'a> {
    Values(&'a [Value]),
    Strings(&'a [String]),
    Labels(&'a [&'static str]),
    Record(&'a Map<String, Value>),
    Batch([&'a Map<String, Value>; 1]),
    Text(&'a str),
    Count(u32),
    Owned(Value),
}

pub(crate) struct OdooCall<'a> {
    pub(crate) model: &'a str,
    pub(crate) method: &'a str,
    pub(crate) ids: Option<&'a [i64]>,
    pub(crate) leading: Vec<(&'static str, Arg<'a>)>,
    pub(crate) keyword: Vec<(&'static str, Arg<'a>)>,
    pub(crate) args: &'a [Value],
    pub(crate) kwargs: Option<&'a Map<String, Value>>,
}

impl<'a> OdooCall<'a> {
    pub(crate) fn new(model: &'a str, method: &'a str) -> Self {
        Self {
            model,
            method,
            ids: None,
            leading: Vec::new(),
            keyword: Vec::new(),
            args: &[],
            kwargs: None,
        }
    }

    pub(crate) fn ids(mut self, ids: &'a [i64]) -> Self {
        self.ids = Some(ids);
        self
    }

    pub(crate) fn lead(mut self, name: &'static str, arg: Arg<'a>) -> Self {
        self.leading.push((name, arg));
        self
    }

    pub(crate) fn keyword(mut self, name: &'static str, arg: Arg<'a>) -> Self {
        self.keyword.push((name, arg));
        self
    }
}

pub(crate) trait Transport {
    fn execute(
        &self,
        call: &OdooCall<'_>,
    ) -> impl Future<Output = Result<Value, ConnectorFailure>> + Send;
}

pub(crate) async fn perform<T: Transport + Sync>(
    transport: &T,
    request: &ActionRequest,
) -> Result<Value, ConnectorFailure> {
    if let ActionRequest::OdooReadAttachment { attachment_id } = request {
        return attachment::read(transport, *attachment_id).await;
    }
    let call = translate(request)
        .ok_or_else(|| ConnectorFailure::NotSupported(NOT_FOR_THIS_INSTANCE.into()))?;
    transport.execute(&call).await
}

fn models_domain(query: Option<&str>) -> Value {
    match query.filter(|query| !query.is_empty()) {
        Some(query) => json!([["model", "ilike", query]]),
        None => json!([]),
    }
}

fn translate(request: &ActionRequest) -> Option<OdooCall<'_>> {
    Some(match request {
        ActionRequest::OdooListModels { query, limit } => OdooCall::new("ir.model", "search_read")
            .lead("domain", Arg::Owned(models_domain(query.as_deref())))
            .keyword("fields", Arg::Labels(&MODEL_FIELDS))
            .keyword("limit", Arg::Count(*limit)),
        ActionRequest::OdooDescribeModel { model } => OdooCall::new(model.as_str(), "fields_get")
            .keyword("attributes", Arg::Labels(&FIELD_ATTRIBUTES)),
        ActionRequest::OdooSearchRead {
            model,
            domain,
            fields,
            limit,
            offset,
            order,
        } => {
            let call = OdooCall::new(model.as_str(), "search_read")
                .lead("domain", Arg::Values(domain))
                .keyword("fields", Arg::Strings(fields))
                .keyword("limit", Arg::Count(*limit))
                .keyword("offset", Arg::Count(*offset));
            match order {
                Some(order) => call.keyword("order", Arg::Text(order)),
                None => call,
            }
        }
        ActionRequest::OdooRead { model, ids, fields } => OdooCall::new(model.as_str(), "read")
            .ids(ids)
            .keyword("fields", Arg::Strings(fields)),
        ActionRequest::OdooCount { model, domain } => {
            OdooCall::new(model.as_str(), "search_count").lead("domain", Arg::Values(domain))
        }
        ActionRequest::OdooCreate { model, values } => {
            OdooCall::new(model.as_str(), "create").lead("vals_list", Arg::Batch([values]))
        }
        ActionRequest::OdooWrite { model, ids, values } => OdooCall::new(model.as_str(), "write")
            .ids(ids)
            .lead("vals", Arg::Record(values)),
        ActionRequest::OdooDelete { model, ids } => {
            OdooCall::new(model.as_str(), "unlink").ids(ids)
        }
        ActionRequest::OdooCallMethod {
            model,
            method,
            ids,
            args,
            kwargs,
        } => {
            let mut call = OdooCall::new(model.as_str(), method.as_str());
            call.ids = (!ids.is_empty()).then_some(ids.as_slice());
            call.args = args;
            call.kwargs = Some(kwargs);
            call
        }
        _ => return None,
    })
}

pub(crate) struct Json2Body<'b, 'a>(pub(crate) &'b OdooCall<'a>);

impl Serialize for Json2Body<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let call = self.0;
        let mut map = serializer.serialize_map(None)?;
        if let Some(ids) = call.ids {
            map.serialize_entry("ids", ids)?;
        }
        for (name, arg) in call.leading.iter().chain(&call.keyword) {
            map.serialize_entry(name, arg)?;
        }
        if let Some(kwargs) = call.kwargs {
            for (name, value) in kwargs {
                if call.ids.is_none() || name != "ids" {
                    map.serialize_entry(name, value)?;
                }
            }
        }
        map.end()
    }
}

pub(crate) struct RpcArgs<'b, 'a>(pub(crate) &'b OdooCall<'a>);

impl Serialize for RpcArgs<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let call = self.0;
        let mut seq = serializer.serialize_seq(None)?;
        if let Some(ids) = call.ids {
            seq.serialize_element(ids)?;
        }
        for (_, arg) in &call.leading {
            match arg {
                Arg::Batch([record]) => seq.serialize_element(record)?,
                other => seq.serialize_element(other)?,
            }
        }
        for value in call.args {
            seq.serialize_element(value)?;
        }
        seq.end()
    }
}

pub(crate) struct RpcKwargs<'b, 'a>(pub(crate) &'b OdooCall<'a>);

impl Serialize for RpcKwargs<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let call = self.0;
        let mut map = serializer.serialize_map(None)?;
        for (name, arg) in &call.keyword {
            map.serialize_entry(name, arg)?;
        }
        if let Some(kwargs) = call.kwargs {
            for (name, value) in kwargs {
                map.serialize_entry(name, value)?;
            }
        }
        map.end()
    }
}
