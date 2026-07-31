use std::env;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context};
use arc_web::ArcApp;
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rand::RngCore;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::domain::AppAggregate;

mod domain;
mod routes;
{{ui-module}}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let command = env::args().nth(1).unwrap_or_else(|| "serve".to_string());

    if command == "setup" {
        setup()?;
    }

    dotenv::dotenv().ok();
    init_logging();

    match command.as_str() {
        "setup" => {
            migrate()?;
            println!("Setup complete. Run `make dev`.");
            Ok(())
        }
        "migrate" => migrate(),
        "serve" => serve().await,
        other => bail!("unknown command `{other}`; expected setup, migrate, or serve"),
    }
}

fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn setup() -> anyhow::Result<()> {
    if !Path::new(".env").exists() {
        let example = fs::read_to_string(".env.example")
            .context("missing .env.example; run this command from the project root")?;
        let mut secret = [0_u8; 64];
        rand::thread_rng().fill_bytes(&mut secret);
        let secret = secret
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        fs::write(".env", example.replace("generate-me", &secret))
            .context("failed to create .env")?;
        println!("Created .env with a new local secret.");
    } else {
        println!("Keeping existing .env.");
    }
    Ok(())
}

fn migrate() -> anyhow::Result<()> {
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "database/database.sqlite".to_string());
    if let Some(parent) = Path::new(&database_url).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create database directory {}", parent.display()))?;
    }
    let mut connection = SqliteConnection::establish(&database_url)
        .with_context(|| format!("failed to open SQLite database at {database_url}"))?;
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|error| anyhow::anyhow!("database migration failed: {error}"))?;
    println!("Database ready at {database_url}.");
    Ok(())
}

async fn serve() -> anyhow::Result<()> {
    for key in ["APP_URL", "SECRET_KEY", "DATABASE_URL"] {
        if env::var_os(key).is_none() {
            bail!("missing {key}; run `make setup` first");
        }
    }

    let host = env::var("APP_URL").context("APP_URL is missing")?;
    let port = env::var("APP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .context("APP_PORT must be a number from 0 to 65535")?;

    info!(url = %format!("http://{host}:{port}"), "Arc application starting");
    ArcApp::builder()
        .register_aggregate::<AppAggregate>()
        .register_routes(routes::config)
        .serve(host.clone(), port)
        .await
        .with_context(|| {
            format!(
                "could not start on {host}:{port}; the port may be busy (change APP_PORT in .env)"
            )
        })
}
