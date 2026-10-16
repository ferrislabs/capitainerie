use capitainerie_events::EventEnvelope;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ClientMessage {
    Identify { token: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ServerMessage {
    Ready,
    Degraded,
    Resync,
    Event {
        kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        run_id: Option<Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        card_id: Option<Uuid>,
        occurred_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub organization_id: Uuid,
    pub message: ServerMessage,
}

impl Notice {
    pub fn from_envelope(envelope: &EventEnvelope) -> Self {
        Self {
            organization_id: envelope.organization_id,
            message: ServerMessage::Event {
                kind: envelope.kind.clone(),
                run_id: id_in(&envelope.payload, "run_id"),
                card_id: id_in(&envelope.payload, "card_id"),
                occurred_at: envelope.occurred_at,
            },
        }
    }

    pub fn concerns(&self, organization_id: Uuid) -> bool {
        self.organization_id == organization_id
    }
}

fn id_in(payload: &serde_json::Value, field: &str) -> Option<Uuid> {
    payload
        .get(field)
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

#[cfg(test)]
mod tests {
    use capitainerie_events::Topic;
    use serde_json::json;

    use super::*;

    fn envelope(kind: &str, topic: Topic, payload: serde_json::Value) -> EventEnvelope {
        EventEnvelope {
            id: Uuid::now_v7(),
            kind: kind.into(),
            topic,
            organization_id: Uuid::now_v7(),
            partition_key: Uuid::now_v7(),
            occurred_at: Utc::now(),
            traceparent: None,
            payload,
        }
    }

    #[test]
    fn identify_deserializes() {
        let message: ClientMessage =
            serde_json::from_str(r#"{"op":"identify","token":"abc"}"#).unwrap();
        assert_eq!(
            message,
            ClientMessage::Identify {
                token: "abc".into()
            }
        );
    }

    #[test]
    fn unknown_client_op_is_rejected() {
        assert!(serde_json::from_str::<ClientMessage>(r#"{"op":"nope"}"#).is_err());
        assert!(serde_json::from_str::<ClientMessage>("not json").is_err());
    }

    #[test]
    fn control_messages_serialize_to_their_op() {
        for (message, op) in [
            (ServerMessage::Ready, "ready"),
            (ServerMessage::Degraded, "degraded"),
            (ServerMessage::Resync, "resync"),
        ] {
            assert_eq!(serde_json::to_value(&message).unwrap(), json!({ "op": op }));
        }
    }

    #[test]
    fn event_carries_ids_and_nothing_else_of_the_payload() {
        let run_id = Uuid::now_v7();
        let card_id = Uuid::now_v7();
        let envelope = envelope(
            "card.moved",
            Topic::Cards,
            json!({"run_id": run_id, "card_id": card_id, "title": "secret", "to": "done"}),
        );
        let notice = Notice::from_envelope(&envelope);
        let value = serde_json::to_value(&notice.message).unwrap();

        assert_eq!(value["op"], "event");
        assert_eq!(value["kind"], "card.moved");
        assert_eq!(value["run_id"], run_id.to_string());
        assert_eq!(value["card_id"], card_id.to_string());
        let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["card_id", "kind", "occurred_at", "op", "run_id"]);
    }

    #[test]
    fn missing_ids_are_omitted() {
        let run_id = Uuid::now_v7();
        let envelope = envelope("run.started", Topic::Runs, json!({"run_id": run_id}));
        let value = serde_json::to_value(Notice::from_envelope(&envelope).message).unwrap();
        assert!(value.get("card_id").is_none());
        assert_eq!(value["run_id"], run_id.to_string());
    }

    #[test]
    fn malformed_ids_are_dropped() {
        let envelope = envelope("run.started", Topic::Runs, json!({"run_id": "nope"}));
        let value = serde_json::to_value(Notice::from_envelope(&envelope).message).unwrap();
        assert!(value.get("run_id").is_none());
    }

    #[test]
    fn a_notice_concerns_only_its_organization() {
        let envelope = envelope("run.queued", Topic::Runs, json!({}));
        let notice = Notice::from_envelope(&envelope);
        assert!(notice.concerns(envelope.organization_id));
        assert!(!notice.concerns(Uuid::now_v7()));
    }
}
