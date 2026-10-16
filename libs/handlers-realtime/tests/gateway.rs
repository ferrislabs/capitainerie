#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use capitainerie_handlers_realtime::{
    Authenticate, Denied, Gateway, Hub, Notice, ServerMessage, Session, Timing, router,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use uuid::Uuid;

#[derive(Clone)]
struct FakeAuth;

/// `<organization uuid>|<subject>[|<ttl in ms>]`, anything else is refused.
impl Authenticate for FakeAuth {
    async fn authenticate(&self, token: &str) -> Result<Session, Denied> {
        let mut parts = token.split('|');
        let organization_id = parts
            .next()
            .and_then(|part| Uuid::parse_str(part).ok())
            .ok_or(Denied("unauthorized"))?;
        let subject = parts.next().ok_or(Denied("unauthorized"))?.to_owned();
        let expires_at = parts
            .next()
            .and_then(|ms| ms.parse::<i64>().ok())
            .map(|ms| Utc::now() + chrono::Duration::milliseconds(ms));
        Ok(Session {
            subject,
            organization_id,
            expires_at,
        })
    }
}

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

const WAIT: Duration = Duration::from_secs(5);

fn timing() -> Timing {
    Timing {
        identify_timeout: Duration::from_millis(300),
        heartbeat: Duration::from_secs(30),
        send_timeout: Duration::from_secs(5),
    }
}

async fn start(hub: &Hub) -> String {
    let gateway = Gateway::new(FakeAuth, hub.clone()).with_timing(timing());
    let app = router::<_, ()>(gateway);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("ws://{addr}/api/v1/realtime")
}

async fn connect(url: &str) -> Client {
    connect_async(url).await.unwrap().0
}

async fn identify(client: &mut Client, token: &str) {
    let frame = json!({"op": "identify", "token": token}).to_string();
    client.send(Message::Text(frame.into())).await.unwrap();
}

async fn next_json(client: &mut Client) -> Value {
    loop {
        match timeout(WAIT, client.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
        {
            Message::Text(text) => return serde_json::from_str(&text).unwrap(),
            Message::Close(frame) => panic!("closed: {frame:?}"),
            _ => {}
        }
    }
}

async fn next_close(client: &mut Client) -> (u16, String) {
    loop {
        match timeout(WAIT, client.next()).await.unwrap() {
            Some(Ok(Message::Close(Some(frame)))) => {
                return (u16::from(frame.code), frame.reason.to_string());
            }
            Some(Ok(_)) => {}
            other => panic!("expected a close frame, got {other:?}"),
        }
    }
}

fn event(organization_id: Uuid, kind: &str, run_id: Uuid) -> Notice {
    Notice {
        organization_id,
        message: ServerMessage::Event {
            kind: kind.into(),
            run_id: Some(run_id),
            card_id: None,
            occurred_at: Utc::now(),
        },
    }
}

fn healthy_hub(capacity: usize) -> Hub {
    let hub = Hub::new(capacity);
    hub.set_degraded(false);
    hub
}

#[tokio::test]
async fn no_identify_closes_with_4001() {
    let hub = healthy_hub(8);
    let mut client = connect(&start(&hub).await).await;
    let (code, reason) = next_close(&mut client).await;
    assert_eq!(code, 4001);
    assert_eq!(reason, "identify timeout");
}

#[tokio::test]
async fn a_first_frame_other_than_identify_closes_with_4001() {
    let hub = healthy_hub(8);
    let mut client = connect(&start(&hub).await).await;
    client
        .send(Message::Text(json!({"op": "hello"}).to_string().into()))
        .await
        .unwrap();
    assert_eq!(next_close(&mut client).await.0, 4001);
}

#[tokio::test]
async fn a_refused_token_closes_with_4001() {
    let hub = healthy_hub(8);
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, "garbage").await;
    let (code, reason) = next_close(&mut client).await;
    assert_eq!(code, 4001);
    assert_eq!(reason, "unauthorized");
}

#[tokio::test]
async fn identify_answers_ready() {
    let hub = healthy_hub(8);
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, &format!("{}|alice", Uuid::now_v7())).await;
    assert_eq!(next_json(&mut client).await, json!({"op": "ready"}));
}

#[tokio::test]
async fn events_of_another_organization_are_not_delivered() {
    let hub = healthy_hub(8);
    let mine = Uuid::now_v7();
    let run = Uuid::now_v7();
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, &format!("{mine}|alice")).await;
    assert_eq!(next_json(&mut client).await["op"], "ready");

    hub.publish(event(Uuid::now_v7(), "run.queued", Uuid::now_v7()));
    hub.publish(event(mine, "run.started", run));

    let received = next_json(&mut client).await;
    assert_eq!(received["op"], "event");
    assert_eq!(received["kind"], "run.started");
    assert_eq!(received["run_id"], run.to_string());
}

#[tokio::test]
async fn a_lagging_socket_gets_a_resync_and_stays_connected() {
    let hub = healthy_hub(2);
    let mine = Uuid::now_v7();
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, &format!("{mine}|alice")).await;
    assert_eq!(next_json(&mut client).await["op"], "ready");
    tokio::time::sleep(Duration::from_millis(100)).await;

    for _ in 0..50 {
        hub.publish(event(mine, "run.step_finished", Uuid::now_v7()));
    }

    let mut saw_resync = false;
    for _ in 0..60 {
        if next_json(&mut client).await["op"] == "resync" {
            saw_resync = true;
            break;
        }
    }
    assert!(saw_resync);
    hub.publish(event(mine, "run.finished", Uuid::now_v7()));
    let mut saw_after = false;
    for _ in 0..60 {
        if next_json(&mut client).await["kind"] == "run.finished" {
            saw_after = true;
            break;
        }
    }
    assert!(saw_after);
}

#[tokio::test]
async fn a_degraded_hub_is_announced_after_ready_and_recovery_resyncs() {
    let hub = Hub::new(8);
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, &format!("{}|alice", Uuid::now_v7())).await;
    assert_eq!(next_json(&mut client).await["op"], "ready");
    assert_eq!(next_json(&mut client).await["op"], "degraded");

    hub.set_degraded(false);
    assert_eq!(next_json(&mut client).await["op"], "ready");
    assert_eq!(next_json(&mut client).await["op"], "resync");

    hub.set_degraded(true);
    assert_eq!(next_json(&mut client).await["op"], "degraded");
}

#[tokio::test]
async fn reidentify_with_the_same_subject_renews_the_session() {
    let hub = healthy_hub(8);
    let mine = Uuid::now_v7();
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, &format!("{mine}|alice|300")).await;
    assert_eq!(next_json(&mut client).await["op"], "ready");
    identify(&mut client, &format!("{mine}|alice")).await;
    assert_eq!(next_json(&mut client).await["op"], "ready");

    tokio::time::sleep(Duration::from_millis(500)).await;
    hub.publish(event(mine, "run.finished", Uuid::now_v7()));
    assert_eq!(next_json(&mut client).await["kind"], "run.finished");
}

#[tokio::test]
async fn reidentify_with_another_subject_closes_with_4003() {
    let hub = healthy_hub(8);
    let mine = Uuid::now_v7();
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, &format!("{mine}|alice")).await;
    assert_eq!(next_json(&mut client).await["op"], "ready");
    identify(&mut client, &format!("{mine}|bob")).await;
    assert_eq!(next_close(&mut client).await.0, 4003);
}

#[tokio::test]
async fn an_expired_token_closes_with_4001() {
    let hub = healthy_hub(8);
    let mut client = connect(&start(&hub).await).await;
    identify(&mut client, &format!("{}|alice|200", Uuid::now_v7())).await;
    assert_eq!(next_json(&mut client).await["op"], "ready");
    let (code, reason) = next_close(&mut client).await;
    assert_eq!(code, 4001);
    assert_eq!(reason, "token expired");
}
