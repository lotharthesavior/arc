use arc_app::domain::user::projector::{UserProjector, USERS_VIEW};
use arc_core::event::Event;
use arc_core::projection::ProjectionEngine;
use arc_core::read_model_store::ReadModelStore;
use arc_es_sqlite::{SqliteEventStore, SqliteReadModelStore};
use async_nats::jetstream;
use async_nats::jetstream::consumer::{AckPolicy, DeliverPolicy, ReplayPolicy};
use async_nats::jetstream::{consumer, AckKind};
use futures_util::StreamExt;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

const EVENTS_SUBJECT: &str = "events.>";

pub struct WorkerConfig {
    pub database_url: String,
    pub nats_url: String,
    pub nats_stream: String,
    pub durable_name: String,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "database/arc_dev.db".into()),
            nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into()),
            nats_stream: env::var("NATS_STREAM").unwrap_or_else(|_| "EVENTS".into()),
            durable_name: env::var("NATS_CONSUMER").unwrap_or_else(|_| "arc-worker".into()),
        }
    }
}

pub async fn build_projection_engine(
    database_url: &str,
) -> Result<Arc<ProjectionEngine>, Box<dyn Error + Send + Sync>> {
    let event_store = SqliteEventStore::new(database_url).await?;
    let read_model_store: Arc<dyn ReadModelStore> =
        Arc::new(SqliteReadModelStore::new(database_url).await?);

    let mut engine = ProjectionEngine::new(Box::new(event_store));
    engine.register_projector(Box::new(UserProjector::new()), read_model_store, USERS_VIEW);

    Ok(Arc::new(engine))
}

pub async fn connect_consumer(
    config: &WorkerConfig,
) -> Result<consumer::PullConsumer, Box<dyn Error + Send + Sync>> {
    let client = async_nats::connect(&config.nats_url).await?;
    let jetstream = jetstream::new(client);

    let stream = jetstream
        .get_or_create_stream(jetstream::stream::Config {
            name: config.nats_stream.clone(),
            subjects: vec![EVENTS_SUBJECT.to_string()],
            ..Default::default()
        })
        .await?;

    let consumer = stream
        .get_or_create_consumer(
            &config.durable_name,
            consumer::pull::Config {
                durable_name: Some(config.durable_name.clone()),
                filter_subject: EVENTS_SUBJECT.to_string(),
                ack_policy: AckPolicy::Explicit,
                deliver_policy: DeliverPolicy::All,
                ack_wait: Duration::from_secs(30),
                max_ack_pending: 1,
                replay_policy: ReplayPolicy::Instant,
                ..Default::default()
            },
        )
        .await?;

    info!(
        stream = config.nats_stream,
        durable = config.durable_name,
        "JetStream durable consumer ready"
    );

    Ok(consumer)
}

pub async fn process_jetstream_message(
    message: async_nats::jetstream::Message,
    projection_engine: Arc<ProjectionEngine>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let event = match serde_json::from_slice::<Event>(&message.payload) {
        Ok(event) => event,
        Err(error) => {
            error!(error = ?error, "Failed to deserialize Event from JetStream message");
            message.ack_with(AckKind::Nak(None)).await?;
            return Ok(());
        }
    };

    info!(
        event_type = event.event_type,
        event_id = %event.event_id,
        aggregate_id = event.aggregate_id,
        "Received event"
    );

    match projection_engine.process(&event).await {
        Ok(()) => {
            message.ack().await?;
            info!(
                event_type = event.event_type,
                event_id = %event.event_id,
                "Projected and acked event"
            );
        }
        Err(error) => {
            error!(
                error = ?error,
                event_type = event.event_type,
                event_id = %event.event_id,
                "Projection failed; nacking event for redelivery"
            );
            message.ack_with(AckKind::Nak(None)).await?;
        }
    }

    Ok(())
}

pub async fn run_consumer_loop(
    consumer: consumer::PullConsumer,
    projection_engine: Arc<ProjectionEngine>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut messages = consumer.messages().await?;

    while let Some(delivery) = messages.next().await {
        let message = match delivery {
            Ok(message) => message,
            Err(error) => {
                warn!(error = ?error, "JetStream delivery error");
                continue;
            }
        };

        process_jetstream_message(message, projection_engine.clone()).await?;
    }

    Ok(())
}
