use capitainerie_bus_iggy::{IggyArgs, IggyEventBus};
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
