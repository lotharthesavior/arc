use arc_core::audit::AuditMetadata;
use arc_core::event::Event;
use arc_core::event_bus::EventBus;
use arc_es_nats::NatsEventBus;
use async_nats::jetstream;
use async_nats::jetstream::consumer::{AckPolicy, DeliverPolicy, ReplayPolicy};
use async_nats::jetstream::{consumer, AckKind};
use futures_util::StreamExt;
use serde_json::json;
use std::error::Error;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

struct NatsServer {
    url: String,
    child: Child,
}

impl Drop for NatsServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn start_nats() -> Result<Option<NatsServer>, Box<dyn Error + Send + Sync>> {
    if Command::new("nats-server")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("skipping NATS integration test: nats-server binary not found");
        return Ok(None);
    }

    let port = free_port()?;
    let store_dir = std::env::temp_dir().join(format!("arc-nats-test-{}", Uuid::new_v4()));
    let mut child = Command::new("nats-server")
        .args(["-js", "-p", &port.to_string(), "-sd"])
        .arg(&store_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let url = format!("nats://127.0.0.1:{port}");

    for _ in 0..40 {
        match async_nats::connect(&url).await {
            Ok(client) => {
                drop(client);
                return Ok(Some(NatsServer { url, child }));
            }
            Err(_) => sleep(Duration::from_millis(50)).await,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    Err("nats-server did not accept connections".into())
}

fn free_port() -> Result<u16, Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn stream_name() -> String {
    format!("EVENTS_{}", Uuid::new_v4().simple())
}

fn durable_name() -> String {
    format!("worker_{}", Uuid::new_v4().simple())
}

fn user_registered(id: &str) -> Event {
    Event::new(
        "User",
        id,
        1,
        "UserRegistered",
        json!({
            "id": id,
            "name": "Ada",
            "email": "ada@example.test",
            "password_hash": "$argon2$test"
        }),
    )
    .with_audit(AuditMetadata::test_default())
}

async fn pull_consumer(
    url: &str,
    stream_name: &str,
    durable_name: &str,
) -> Result<consumer::PullConsumer, Box<dyn Error + Send + Sync>> {
    let client = async_nats::connect(url).await?;
    let jetstream = jetstream::new(client);
    let stream = jetstream.get_stream(stream_name).await?;
    Ok(stream
        .get_or_create_consumer(
            durable_name,
            consumer::pull::Config {
                durable_name: Some(durable_name.to_string()),
                filter_subject: "events.>".to_string(),
                ack_policy: AckPolicy::Explicit,
                deliver_policy: DeliverPolicy::All,
                ack_wait: Duration::from_secs(2),
                replay_policy: ReplayPolicy::Instant,
                ..Default::default()
            },
        )
        .await?)
}

async fn next_message(
    consumer: &consumer::PullConsumer,
) -> Result<async_nats::jetstream::Message, Box<dyn Error + Send + Sync>> {
    let mut batch = consumer
        .fetch()
        .max_messages(1)
        .expires(Duration::from_secs(2))
        .messages()
        .await?;

    let delivery = timeout(Duration::from_secs(3), batch.next())
        .await?
        .ok_or("expected a JetStream message")?;

    delivery
}

#[tokio::test]
async fn publish_consume_roundtrip_over_jetstream() -> TestResult {
    let Some(server) = start_nats().await? else {
        return Ok(());
    };
    let stream = stream_name();
    let bus = NatsEventBus::new(&server.url, &stream).await?;
    let event = user_registered("user-roundtrip");

    bus.publish(vec![event.clone()]).await?;

    let consumer = pull_consumer(&server.url, &stream, &durable_name()).await?;
    let message = next_message(&consumer).await?;
    let delivered = serde_json::from_slice::<Event>(&message.payload)?;
    message.ack().await?;

    assert_eq!(delivered.event_id, event.event_id);
    assert_eq!(delivered.aggregate_type, "User");
    assert_eq!(delivered.event_type, "UserRegistered");
    assert_eq!(message.subject.as_str(), "events.user.user_registered");
    Ok(())
}

#[tokio::test]
async fn stream_and_consumer_creation_are_idempotent() -> TestResult {
    let Some(server) = start_nats().await? else {
        return Ok(());
    };
    let stream = stream_name();

    let first = NatsEventBus::new(&server.url, &stream).await?;
    let second = NatsEventBus::new(&server.url, &stream).await?;
    assert_eq!(first.stream(), stream);
    assert_eq!(second.stream(), stream);

    let durable = durable_name();
    let mut first_consumer = pull_consumer(&server.url, &stream, &durable).await?;
    let mut second_consumer = pull_consumer(&server.url, &stream, &durable).await?;

    assert_eq!(first_consumer.info().await?.name, durable);
    assert_eq!(second_consumer.info().await?.name, durable);
    Ok(())
}

#[tokio::test]
async fn nak_causes_redelivery_until_ack() -> TestResult {
    let Some(server) = start_nats().await? else {
        return Ok(());
    };
    let stream = stream_name();
    let bus = NatsEventBus::new(&server.url, &stream).await?;
    let event = user_registered("user-redelivery");
    bus.publish(vec![event.clone()]).await?;

    let consumer = pull_consumer(&server.url, &stream, &durable_name()).await?;
    let first = next_message(&consumer).await?;
    let first_event = serde_json::from_slice::<Event>(&first.payload)?;
    first.ack_with(AckKind::Nak(None)).await?;

    let second = next_message(&consumer).await?;
    let second_event = serde_json::from_slice::<Event>(&second.payload)?;
    second.ack().await?;

    assert_eq!(first_event.event_id, event.event_id);
    assert_eq!(second_event.event_id, event.event_id);
    Ok(())
}
