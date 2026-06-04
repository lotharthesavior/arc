use arc_worker::{build_projection_engine, connect_consumer, run_consumer_loop, WorkerConfig};
use dotenv::dotenv;
use std::error::Error;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    init_tracing();
    dotenv().ok();

    let config = WorkerConfig::from_env();
    info!(
        nats_url = config.nats_url,
        nats_stream = config.nats_stream,
        durable = config.durable_name,
        database_url = config.database_url,
        "Starting arc-worker"
    );

    let projection_engine = build_projection_engine(&config.database_url).await?;
    if let Err(error) = projection_engine.rebuild_all().await {
        warn!(
            error = ?error,
            "Projection rebuild failed at worker startup; durable consumer will continue from JetStream"
        );
    } else {
        info!("Projections rebuilt from event store");
    }

    let consumer = connect_consumer(&config).await?;
    run_consumer_loop(consumer, projection_engine).await
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "arc_worker=info,arc=info,arc_core=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
