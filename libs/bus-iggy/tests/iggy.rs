use std::time::Duration;

use capitainerie_bus_iggy::{IggyArgs, IggyEventBus, StartFrom};
use capitainerie_core::domain::event::ports::EventBus;
use capitainerie_events::{EventEnvelope, Topic};
use chrono::Utc;
use iggy::prelude::*;
use uuid::Uuid;

fn address() -> Option<String> {
    match std::env::var("IGGY_ADDRESS") {
        Ok(address) => Some(address),
        Err(_) if std::env::var("REQUIRE_IGGY").as_deref() == Ok("1") => {
            panic!("IGGY_ADDRESS is required when REQUIRE_IGGY=1")
        }
        Err(_) => None,
    }
}

fn args(address: String) -> IggyArgs {
    IggyArgs {
        address,
        username: "iggy".into(),
        password: "iggy".into(),
        stream: format!("test-{}", Uuid::now_v7().simple()),
        partitions: 3,
    }
}

fn envelope(key: Uuid, n: u32) -> EventEnvelope {
    EventEnvelope {
        id: Uuid::now_v7(),
        kind: "run.queued".into(),
        topic: Topic::Runs,
        organization_id: Uuid::now_v7(),
        partition_key: key,
        occurred_at: Utc::now(),
        traceparent: Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".into()),
        payload: serde_json::json!({ "n": n }),
    }
}

async fn drop_stream(args: &IggyArgs) {
    let Ok(client) = IggyClient::builder()
        .with_tcp()
        .with_server_address(args.address.clone())
        .build()
    else {
        return;
    };
    if client.connect().await.is_err() {
        return;
    }
    if client
        .login_user(&args.username, &args.password)
        .await
        .is_err()
    {
        return;
    }
    if let Ok(id) = Identifier::from_str_value(&args.stream) {
        let _ = client.delete_stream(&id).await;
    }
}

#[tokio::test]
async fn connect_is_idempotent() {
    let Some(address) = address() else {
        eprintln!("skipped: IGGY_ADDRESS is unset");
        return;
    };
    let args = args(address);
    IggyEventBus::connect(&args).await.unwrap();
    IggyEventBus::connect(&args).await.unwrap();
    drop_stream(&args).await;
}

#[tokio::test]
async fn published_events_are_polled_in_order() {
    let Some(address) = address() else {
        eprintln!("skipped: IGGY_ADDRESS is unset");
        return;
    };
    let args = args(address);
    let bus = IggyEventBus::connect(&args).await.unwrap();
    let key = Uuid::now_v7();
    let events: Vec<_> = (0..3).map(|n| envelope(key, n)).collect();

    bus.publish(&events).await.unwrap();
    let polled = bus.poll(Topic::Runs, "test-group", 10).await.unwrap();
    drop_stream(&args).await;

    assert_eq!(polled, events);
}

#[tokio::test]
async fn publishing_nothing_is_ok() {
    let Some(address) = address() else {
        eprintln!("skipped: IGGY_ADDRESS is unset");
        return;
    };
    let args = args(address);
    let bus = IggyEventBus::connect(&args).await.unwrap();
    bus.publish(&[]).await.unwrap();
    drop_stream(&args).await;
}

async fn drain(subscription: &mut capitainerie_bus_iggy::Subscription) -> Vec<EventEnvelope> {
    let mut all = Vec::new();
    for _ in 0..3 {
        match subscription.next_batch(10).await {
            Ok(batch) => all.extend(batch),
            Err(error) => panic!("poll failed: {error}"),
        }
    }
    all
}

#[tokio::test]
async fn latest_skips_the_backlog_then_sees_new_messages() {
    let Some(address) = address() else {
        eprintln!("skipped: IGGY_ADDRESS is unset");
        return;
    };
    let args = args(address);
    let bus = IggyEventBus::connect(&args).await.unwrap();
    let key = Uuid::now_v7();
    let old: Vec<_> = (0..3).map(|n| envelope(key, n)).collect();
    bus.publish(&old).await.unwrap();

    let mut subscription = bus
        .subscribe(Topic::Runs, "latest-group", StartFrom::Latest)
        .await
        .unwrap();
    let skipped = drain(&mut subscription).await;
    let fresh = envelope(key, 9);
    bus.publish(std::slice::from_ref(&fresh)).await.unwrap();
    let seen = drain(&mut subscription).await;
    drop_stream(&args).await;

    assert!(skipped.is_empty());
    assert_eq!(seen, vec![fresh]);
}

#[tokio::test]
async fn earliest_reads_the_backlog_and_resumes_after_the_commit() {
    let Some(address) = address() else {
        eprintln!("skipped: IGGY_ADDRESS is unset");
        return;
    };
    let args = args(address);
    let bus = IggyEventBus::connect(&args).await.unwrap();
    let key = Uuid::now_v7();
    let backlog: Vec<_> = (0..3).map(|n| envelope(key, n)).collect();
    bus.publish(&backlog).await.unwrap();

    let mut first = bus
        .subscribe(Topic::Runs, "earliest-group", StartFrom::Earliest)
        .await
        .unwrap();
    let read = drain(&mut first).await;
    first.close().await.unwrap();

    let next = envelope(key, 7);
    bus.publish(std::slice::from_ref(&next)).await.unwrap();
    let mut second = bus
        .subscribe(Topic::Runs, "earliest-group", StartFrom::Earliest)
        .await
        .unwrap();
    let resumed = drain(&mut second).await;
    drop_stream(&args).await;

    assert_eq!(read, backlog);
    assert_eq!(resumed, vec![next]);
}

async fn group_names(args: &IggyArgs, topic: Topic) -> Result<Vec<String>, IggyError> {
    let client = IggyClient::builder()
        .with_tcp()
        .with_server_address(args.address.clone())
        .build()?;
    client.connect().await?;
    client.login_user(&args.username, &args.password).await?;
    let stream = Identifier::from_str_value(&args.stream)?;
    let topic = Identifier::from_str_value(topic.as_str())?;
    let groups = client.get_consumer_groups(&stream, &topic).await?;
    Ok(groups.into_iter().map(|g| g.name).collect())
}

#[tokio::test]
async fn ephemeral_group_is_deleted_on_close() {
    let Some(address) = address() else {
        eprintln!("skipped: IGGY_ADDRESS is unset");
        return;
    };
    let args = args(address);
    let bus = IggyEventBus::connect(&args).await.unwrap();
    let mut subscription = bus
        .subscribe_ephemeral(Topic::Runs, "fanout")
        .await
        .unwrap();
    assert!(drain(&mut subscription).await.is_empty());
    let during = group_names(&args, Topic::Runs).await.unwrap_or_default();
    subscription.close().await.unwrap();
    let after = group_names(&args, Topic::Runs).await.unwrap_or_default();
    drop_stream(&args).await;

    assert!(during.iter().any(|n| n.starts_with("fanout-")));
    assert!(!after.iter().any(|n| n.starts_with("fanout-")));
}

#[tokio::test]
async fn prune_removes_only_old_empty_groups() {
    let Some(address) = address() else {
        eprintln!("skipped: IGGY_ADDRESS is unset");
        return;
    };
    let args = args(address);
    let bus = IggyEventBus::connect(&args).await.unwrap();
    let kept = bus
        .subscribe_ephemeral(Topic::Runs, "fanout")
        .await
        .unwrap();
    let stale_name = format!("fanout-{}", Uuid::now_v7());
    let other = bus
        .subscribe(Topic::Runs, "unrelated", StartFrom::Earliest)
        .await
        .unwrap();
    other.close().await.unwrap();
    let stale = bus
        .subscribe(Topic::Runs, &stale_name, StartFrom::Earliest)
        .await
        .unwrap();
    stale.close().await.unwrap();

    let none = bus
        .prune_groups(Topic::Runs, "fanout", Duration::from_secs(3600))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let pruned = bus
        .prune_groups(Topic::Runs, "fanout", Duration::from_millis(5))
        .await
        .unwrap();
    let after = group_names(&args, Topic::Runs).await.unwrap_or_default();
    kept.close().await.ok();
    drop_stream(&args).await;

    assert_eq!(none, 0);
    assert!(pruned >= 1);
    assert!(!after.contains(&stale_name));
    assert!(after.contains(&"unrelated".to_owned()));
}
