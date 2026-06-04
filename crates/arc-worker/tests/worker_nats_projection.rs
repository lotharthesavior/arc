use arc_app::domain::user::projector::{UserProjector, USERS_VIEW};
use arc_core::audit::AuditMetadata;
use arc_core::event::Event;
use arc_core::event_store::InMemoryEventStore;
use arc_core::projection::ProjectionEngine;
use arc_core::read_model_store::{InMemoryReadModelStore, ReadModelStore};
use arc_es_nats::NatsEventBus;
use arc_worker::{connect_consumer, process_jetstream_message, WorkerConfig};
use futures_util::StreamExt;
use serde_json::json;
use std::error::Error;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
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
        eprintln!("skipping worker NATS integration test: nats-server binary not found");
        return Ok(None);
    }

    let port = free_port()?;
    let store_dir = std::env::temp_dir().join(format!("arc-worker-nats-test-{}", Uuid::new_v4()));
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

fn user_registered(id: &str) -> Event {
    Event::new(
        "User",
        id,
        1,
        "UserRegistered",
        json!({
            "id": id,
            "name": "Grace",
            "email": "grace@example.test",
            "password_hash": "$argon2$test"
        }),
    )
    .with_audit(AuditMetadata::test_default())
}

#[tokio::test]
async fn durable_worker_projects_and_acks_event() -> TestResult {
    let Some(server) = start_nats().await? else {
        return Ok(());
    };
    let stream = format!("EVENTS_{}", Uuid::new_v4().simple());
    let durable = format!("arc_worker_{}", Uuid::new_v4().simple());
    let event = user_registered("worker-user-1");

    let bus = NatsEventBus::new(&server.url, &stream).await?;
    arc_core::event_bus::EventBus::publish(&bus, vec![event.clone()]).await?;

    let config = WorkerConfig {
        database_url: "unused-for-in-memory-test".to_string(),
        nats_url: server.url.clone(),
        nats_stream: stream,
        durable_name: durable,
    };
    let consumer = connect_consumer(&config).await?;

    let read_model_store = Arc::new(InMemoryReadModelStore::new());
    let mut engine = ProjectionEngine::new(Box::new(InMemoryEventStore::new()));
    engine.register_projector(
        Box::new(UserProjector::new()),
        read_model_store.clone(),
        USERS_VIEW,
    );
    let engine = Arc::new(engine);

    let mut batch = consumer
        .fetch()
        .max_messages(1)
        .expires(Duration::from_secs(2))
        .messages()
        .await?;
    let delivery = timeout(Duration::from_secs(3), batch.next())
        .await?
        .ok_or("expected worker delivery")??;

    process_jetstream_message(delivery, engine).await?;

    let row = read_model_store
        .get(USERS_VIEW, &event.aggregate_id)
        .await?
        .ok_or("expected projected user row")?;
    assert_eq!(row["name"], "Grace");
    assert_eq!(row["email"], "grace@example.test");
    assert_eq!(row["version"], 1);

    let mut empty_batch = consumer
        .fetch()
        .max_messages(1)
        .expires(Duration::from_millis(200))
        .messages()
        .await?;
    let maybe_delivery = timeout(Duration::from_secs(1), empty_batch.next()).await?;
    assert!(
        maybe_delivery.is_none(),
        "acked event should not be redelivered"
    );

    Ok(())
}
