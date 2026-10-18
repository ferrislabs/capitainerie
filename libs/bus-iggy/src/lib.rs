use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use capitainerie_common::CoreError;
use capitainerie_core::domain::event::ports::EventBus;
use capitainerie_events::{EventEnvelope, Topic};
use iggy::prelude::*;
use uuid::Uuid;

#[derive(Clone, clap::Args)]
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

impl std::fmt::Debug for IggyArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IggyArgs")
            .field("address", &self.address)
            .field("username", &self.username)
            .field("password", &"***")
            .field("stream", &self.stream)
            .field("partitions", &self.partitions)
            .finish()
    }
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
    #[error("iggy subscription failed: {0}")]
    Subscribe(String),
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
    joined: Arc<Mutex<HashSet<(String, String)>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartFrom {
    Earliest,
    Latest,
}

pub struct Subscription {
    client: Arc<IggyClient>,
    stream: Identifier,
    topic: Identifier,
    group: Identifier,
    consumer: Consumer,
    partitions: u32,
    ephemeral: bool,
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
            joined: Arc::default(),
        })
    }

    async fn ensure_group(
        &self,
        topic: &Identifier,
        group: &Identifier,
        name: &str,
    ) -> Result<bool, IggyError> {
        let created = match self
            .client
            .create_consumer_group(&self.stream, topic, name)
            .await
        {
            Ok(_) => true,
            Err(IggyError::ConsumerGroupNameAlreadyExists(..)) => false,
            Err(error) => return Err(error),
        };
        self.client
            .join_consumer_group(&self.stream, topic, group)
            .await?;
        Ok(created)
    }

    fn joined_key(topic: Topic, group: &str) -> (String, String) {
        (topic.as_str().to_owned(), group.to_owned())
    }

    fn forget(&self, key: &(String, String)) {
        if let Ok(mut joined) = self.joined.lock() {
            joined.remove(key);
        }
    }

    pub async fn subscribe(
        &self,
        topic: Topic,
        group: &str,
        start: StartFrom,
    ) -> Result<Subscription, BusError> {
        self.open(topic, group, start, false).await
    }

    /// Fan-out subscription under a unique `<prefix>-<uuid v7>` group that only
    /// sees messages published from now on. Call [`Subscription::close`] on
    /// shutdown to delete the group; a crash leaves it behind, and
    /// [`IggyEventBus::prune_groups`] removes such leftovers.
    pub async fn subscribe_ephemeral(
        &self,
        topic: Topic,
        prefix: &str,
    ) -> Result<Subscription, BusError> {
        let group = format!("{prefix}-{}", Uuid::now_v7());
        self.open(topic, &group, StartFrom::Latest, true).await
    }

    /// Deletes the empty groups named `<prefix>-<uuid v7>` on `topic` whose
    /// uuid is older than `older_than` (left behind by crashed processes).
    /// Returns how many were deleted.
    pub async fn prune_groups(
        &self,
        topic: Topic,
        prefix: &str,
        older_than: Duration,
    ) -> Result<usize, BusError> {
        let topic_id = identifier(topic.as_str()).map_err(BusError::Subscribe)?;
        let groups = self
            .client
            .get_consumer_groups(&self.stream, &topic_id)
            .await
            .map_err(|e| BusError::Subscribe(e.to_string()))?;
        let cutoff = SystemTime::now()
            .checked_sub(older_than)
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .unwrap_or_default();
        let mut deleted = 0;
        for group in groups.iter().filter(|g| g.members_count == 0) {
            let Some(suffix) = group
                .name
                .strip_prefix(prefix)
                .and_then(|s| s.strip_prefix('-'))
            else {
                continue;
            };
            let Some(created) = Uuid::parse_str(suffix)
                .ok()
                .and_then(|u| u.get_timestamp())
                .map(|t| {
                    let (secs, nanos) = t.to_unix();
                    Duration::new(secs, nanos)
                })
            else {
                continue;
            };
            if created > cutoff {
                continue;
            }
            let group_id = identifier(&group.name).map_err(BusError::Subscribe)?;
            self.client
                .delete_consumer_group(&self.stream, &topic_id, &group_id)
                .await
                .map_err(|e| BusError::Subscribe(e.to_string()))?;
            deleted += 1;
        }
        Ok(deleted)
    }

    async fn open(
        &self,
        topic: Topic,
        group: &str,
        start: StartFrom,
        ephemeral: bool,
    ) -> Result<Subscription, BusError> {
        let fail = |e: String| BusError::Subscribe(e);
        let topic_id = identifier(topic.as_str()).map_err(fail)?;
        let group_id = identifier(group).map_err(fail)?;
        let created = self
            .ensure_group(&topic_id, &group_id, group)
            .await
            .map_err(|e| BusError::Subscribe(e.to_string()))?;
        let consumer = Consumer::group(group_id.clone());
        if created && start == StartFrom::Latest {
            let details = self
                .client
                .get_topic(&self.stream, &topic_id)
                .await
                .map_err(|e| BusError::Subscribe(e.to_string()))?
                .ok_or_else(|| {
                    BusError::Subscribe(format!("topic {} not found", topic.as_str()))
                })?;
            for partition in details.partitions.iter().filter(|p| p.messages_count > 0) {
                self.client
                    .store_consumer_offset(
                        &consumer,
                        &self.stream,
                        &topic_id,
                        Some(partition.id),
                        partition.current_offset,
                    )
                    .await
                    .map_err(|e| BusError::Subscribe(e.to_string()))?;
            }
        }
        Ok(Subscription {
            client: Arc::clone(&self.client),
            stream: self.stream.clone(),
            topic: topic_id,
            group: group_id,
            consumer,
            partitions: self.partitions,
            ephemeral,
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
        let key = Self::joined_key(topic, group);
        let cached = self.joined.lock().is_ok_and(|j| j.contains(&key));
        if !cached {
            self.ensure_group(&topic_id, &group_id, group)
                .await
                .map_err(|e| BusError::Poll(e.to_string()))?;
            if let Ok(mut joined) = self.joined.lock() {
                joined.insert(key.clone());
            }
        }

        let consumer = Consumer::group(group_id);
        let mut envelopes = Vec::new();
        for _ in 0..self.partitions {
            let remaining = count.saturating_sub(envelopes.len() as u32);
            if remaining == 0 {
                break;
            }
            let polled = match self
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
            {
                Ok(polled) => polled,
                Err(error) => {
                    self.forget(&key);
                    return Err(BusError::Poll(error.to_string()));
                }
            };
            for message in &polled.messages {
                envelopes.push(decode(message)?);
            }
        }
        Ok(envelopes)
    }
}

impl Subscription {
    /// Polls every partition in turn and commits each partition's offset once
    /// its messages are decoded, so delivery is at-least-once. Undecodable
    /// messages are logged and skipped.
    pub async fn next_batch(&mut self, count: u32) -> Result<Vec<EventEnvelope>, BusError> {
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
                    &self.topic,
                    None,
                    &self.consumer,
                    &PollingStrategy::next(),
                    remaining,
                    false,
                )
                .await
                .map_err(|e| BusError::Poll(e.to_string()))?;
            let Some(last) = polled.messages.last() else {
                continue;
            };
            for message in &polled.messages {
                match decode(message) {
                    Ok(envelope) => envelopes.push(envelope),
                    Err(error) => tracing::warn!(%error, "skipping an undecodable event"),
                }
            }
            self.client
                .store_consumer_offset(
                    &self.consumer,
                    &self.stream,
                    &self.topic,
                    Some(polled.partition_id),
                    last.header.offset,
                )
                .await
                .map_err(|e| BusError::Poll(e.to_string()))?;
        }
        Ok(envelopes)
    }

    /// Leaves the group, and deletes it when the subscription is ephemeral.
    pub async fn close(self) -> Result<(), BusError> {
        let result = if self.ephemeral {
            self.client
                .delete_consumer_group(&self.stream, &self.topic, &self.group)
                .await
        } else {
            self.client
                .leave_consumer_group(&self.stream, &self.topic, &self.group)
                .await
        };
        result.map_err(|e| BusError::Subscribe(e.to_string()))
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
