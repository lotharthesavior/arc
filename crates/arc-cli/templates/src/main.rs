use std::{env, fs, path::Path};

use anyhow::{bail, Context};
use arc_web::{ArcApp, ArcAppBuilder, PluginSetupContext};
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rand::RngCore;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::domain::AppAggregate;
// arc:resource-imports
// arc:plugin-imports

mod domain;
mod routes;
// {{ui-module}}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let command = env::args().nth(1).unwrap_or_else(|| "serve".to_string());
    if command == "setup" {
        prepare_env()?;
    }
    dotenv::dotenv().ok();
    init_logging();
    match command.as_str() {
        "setup" => {
            migrate()?;
            setup_plugins().await?;
            println!("Setup complete. Run `make dev`.");
            Ok(())
        }
        "migrate" => migrate(),
        "serve" => serve().await,
        other => bail!("unknown command `{other}`; expected setup, migrate, or serve"),
    }
}

fn builder() -> ArcAppBuilder {
    ArcApp::builder()
        .register_aggregate::<AppAggregate>()
        // {{ui-host-registration}}
        // arc:resource-registrations
        // arc:plugin-registrations
        .register_routes(routes::config)
}

async fn setup_plugins() -> anyhow::Result<()> {
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "database/database.sqlite".into());
    builder()
        .setup_plugins(&PluginSetupContext {
            database_url: &database_url,
            project_root: Path::new("."),
        })
        .await?;
    Ok(())
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
fn prepare_env() -> anyhow::Result<()> {
    if !Path::new(".env").exists() {
        let example = fs::read_to_string(".env.example").context("missing .env.example")?;
        let mut secret = [0_u8; 64];
        rand::thread_rng().fill_bytes(&mut secret);
        let secret = secret
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        fs::write(".env", example.replace("generate-me", &secret))?;
    }
    Ok(())
}
fn migrate() -> anyhow::Result<()> {
    let url = env::var("DATABASE_URL").unwrap_or_else(|_| "database/database.sqlite".into());
    if let Some(parent) = Path::new(&url).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut connection = SqliteConnection::establish(&url)?;
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow::anyhow!("migration failed: {e}"))?;
    Ok(())
}
async fn serve() -> anyhow::Result<()> {
    for key in ["APP_URL", "SECRET_KEY", "DATABASE_URL"] {
        if env::var_os(key).is_none() {
            bail!("missing {key}; run `make setup` first");
        }
    }
    let host = env::var("APP_URL")?;
    let port = env::var("APP_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse::<u16>()?;
    info!(url=%format!("http://{host}:{port}"),"Arc application starting");
    builder().serve(host, port).await?;
    Ok(())
}
