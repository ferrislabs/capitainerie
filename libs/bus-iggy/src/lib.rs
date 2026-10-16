use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use capitainerie_common::CoreError;
use capitainerie_core::domain::event::ports::EventBus;
use capitainerie_events::{EventEnvelope, Topic};
use iggy::prelude::*;
use uuid::Uuid;

#[derive(Debug, Clone, clap::Args)]
pub struct IggyArgs {
    #[arg(
        long = "iggy-address",
        env = "IGGY_ADDRESS",
        default_value = "localhost:4190"
    )]
    pub address: String,
    #[arg(long = "iggy-username", env = "IGGY_USERNAME", default_value = "iggy")]
    pub username: String,
    #[arg(long = "iggy-password", env = "IGGY_PASSWORD", default_value = "iggy")]
    pub password: String,
    #[arg(
        long = "iggy-stream",
        env = "IGGY_STREAM",
        default_value = "capitainerie"
    )]
    pub stream: String,
    #[arg(long = "iggy-partitions", env = "IGGY_PARTITIONS", default_value_t = 3)]
    pub partitions: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("iggy connection failed: {0}")]
    Connect(String),
    #[error("iggy topology setup failed: {0}")]
    Topology(String),
    #[error("iggy publish failed: {0}")]
    Publish(String),
    #[error("iggy poll failed: {0}")]
    Poll(String),
    #[error("iggy message decoding failed: {0}")]
    Decode(String),
}

impl From<BusError> for CoreError {
    fn from(error: BusError) -> Self {
        CoreError::Internal(error.to_string())
    }
}

#[derive(Clone)]
pub struct IggyEventBus {
    client: Arc<IggyClient>,
    stream: Identifier,
    partitions: u32,
}

impl IggyEventBus {
    pub async fn connect(args: &IggyArgs) -> Result<Self, BusError> {
        let client = IggyClient::builder()
            .with_tcp()
            .with_server_address(args.address.clone())
            .build()
            .map_err(|e| BusError::Connect(e.to_string()))?;
        client
            .connect()
            .await
            .map_err(|e| BusError::Connect(e.to_string()))?;
        client
            .login_user(&args.username, &args.password)
            .await
            .map_err(|e| BusError::Connect(e.to_string()))?;

        let stream = identifier(&args.stream).map_err(BusError::Topology)?;
        tolerate_existing(client.create_stream(&args.stream).await.map(drop))?;
        for topic in Topic::ALL {
            let options = TopicCreateOptions {
                partitions_count: Some(args.partitions),
                ..Default::default()
            };
            tolerate_existing(
                client
                    .create_topic(&stream, topic.as_str(), &options)
                    .await
                    .map(drop),
            )?;
        }

        Ok(Self {
            client: Arc::new(client),
            stream,
            partitions: args.partitions.max(1),
        })
    }

    pub async fn poll(
        &self,
        topic: Topic,
        group: &str,
        count: u32,
    ) -> Result<Vec<EventEnvelope>, BusError> {
        let topic_id = identifier(topic.as_str()).map_err(BusError::Poll)?;
        let group_id = identifier(group).map_err(BusError::Poll)?;
        tolerate_existing(
            self.client
                .create_consumer_group(&self.stream, &topic_id, group)
                .await
                .map(drop),
        )
        .map_err(|e| BusError::Poll(e.to_string()))?;
        tolerate_existing(
            self.client
                .join_consumer_group(&self.stream, &topic_id, &group_id)
                .await,
        )
        .map_err(|e| BusError::Poll(e.to_string()))?;

        let consumer = Consumer::group(group_id);
        let mut envelopes = Vec::new();
        for _ in 0..self.partitions {
            let remaining = count.saturating_sub(envelopes.len() as u32);
            if remaining == 0 {
                break;
            }
            let polled = self
                .client
                .poll_messages(
                    &self.stream,
                    &topic_id,
                    None,
                    &consumer,
                    &PollingStrategy::next(),
                    remaining,
                    true,
                )
                .await
                .map_err(|e| BusError::Poll(e.to_string()))?;
            for message in &polled.messages {
                envelopes.push(decode(message)?);
            }
        }
        Ok(envelopes)
    }
}

impl EventBus for IggyEventBus {
    #[tracing::instrument(skip_all, fields(count = events.len()), err)]
    async fn publish(&self, events: &[EventEnvelope]) -> Result<(), CoreError> {
        if events.is_empty() {
            return Ok(());
        }
        let mut batches: Vec<Batch> = Vec::new();
        for event in events {
            let message = encode(event)?;
            match batches
                .iter_mut()
                .find(|b| b.topic == event.topic && b.key == event.partition_key)
            {
                Some(batch) => batch.messages.push(message),
                None => batches.push(Batch {
                    topic: event.topic,
                    key: event.partition_key,
                    messages: vec![message],
                }),
            }
        }
        for mut batch in batches {
            let topic_id = identifier(batch.topic.as_str()).map_err(BusError::Publish)?;
            let partitioning = Partitioning::messages_key(batch.key.as_bytes())
                .map_err(|e| BusError::Publish(e.to_string()))?;
            self.client
                .send_messages(&self.stream, &topic_id, &partitioning, &mut batch.messages)
                .await
                .map_err(|e| BusError::Publish(e.to_string()))?;
        }
        Ok(())
    }
}

struct Batch {
    topic: Topic,
    key: Uuid,
    messages: Vec<IggyMessage>,
}

fn identifier(name: &str) -> Result<Identifier, String> {
    Identifier::from_str(name).map_err(|e| e.to_string())
}

fn tolerate_existing(result: Result<(), IggyError>) -> Result<(), BusError> {
    match result {
        Err(
            IggyError::StreamNameAlreadyExists(_)
            | IggyError::TopicNameAlreadyExists(..)
            | IggyError::ConsumerGroupNameAlreadyExists(..),
        )
        | Ok(()) => Ok(()),
        Err(error) => Err(BusError::Topology(error.to_string())),
    }
}

fn encode(envelope: &EventEnvelope) -> Result<IggyMessage, BusError> {
    let payload = serde_json::to_vec(envelope).map_err(|e| BusError::Publish(e.to_string()))?;
    let headers = BTreeMap::from([(
        HeaderKey::from_str("kind").map_err(|e| BusError::Publish(e.to_string()))?,
        HeaderValue::from_str(&envelope.kind).map_err(|e| BusError::Publish(e.to_string()))?,
    )]);
    IggyMessage::builder()
        .id(envelope.id.as_u128())
        .payload(payload.into())
        .user_headers(headers)
        .build()
        .map_err(|e| BusError::Publish(e.to_string()))
}

fn decode(message: &IggyMessage) -> Result<EventEnvelope, BusError> {
    serde_json::from_slice(&message.payload).map_err(|e| BusError::Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn envelope() -> EventEnvelope {
        EventEnvelope {
            id: Uuid::now_v7(),
            kind: "run.queued".into(),
            topic: Topic::Runs,
            organization_id: Uuid::now_v7(),
            partition_key: Uuid::now_v7(),
            occurred_at: Utc::now(),
            traceparent: None,
            payload: serde_json::json!({"a": 1}),
        }
    }

    #[test]
    fn encode_carries_id_and_round_trips() {
        let envelope = envelope();
        let message = encode(&envelope).unwrap();
        assert_eq!(message.header.id, envelope.id.as_u128());
        assert!(message.user_headers.is_some());
        assert_eq!(decode(&message).unwrap(), envelope);
    }

    #[test]
    fn bus_error_maps_to_internal() {
        let error: CoreError = BusError::Publish("boom".into()).into();
        assert!(matches!(error, CoreError::Internal(m) if m.contains("boom")));
    }
}
