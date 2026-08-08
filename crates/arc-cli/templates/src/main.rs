use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use anyhow::{bail, Context};
use arc_core::command_bus::{CommandContext, SnapshotPolicy};
use arc_web::ArcApp;
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rand::RngCore;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::domain::user::commands::UserCommand;
use crate::domain::user::{
    aggregate::UserAggregate,
    projector::{UserProjector, USERS_VIEW},
};
use crate::domain::AppAggregate;
// arc:resource-imports

mod auth;
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
            setup_admin().await?;
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

fn prepare_env() -> anyhow::Result<()> {
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

#[allow(dead_code)]
async fn setup_admin() -> anyhow::Result<()> {
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "database/database.sqlite".to_string());
    let stack = arc_web::helpers::es_stack::build::<UserAggregate>(
        &database_url,
        vec![arc_web::ProjectorReg::new(UserProjector, USERS_VIEW)],
        Some(SnapshotPolicy::EveryNEvents(
            env::var("USER_SNAPSHOT_INTERVAL_EVENTS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(50),
        )),
    )
    .await
    .map_err(|error| anyhow::anyhow!("failed to build User runtime: {error}"))?;
    let active = stack
        .read_model_store
        .list(USERS_VIEW)
        .await
        .map_err(|error| anyhow::anyhow!("failed to inspect users_view: {error}"))?
        .into_iter()
        .any(|row| row["active"].as_bool() == Some(true));
    if active {
        println!("An active administrator already exists; credentials were left unchanged.");
        return Ok(());
    }
    let name = setup_value("ARC_SETUP_ADMIN_NAME", "Administrator name: ", false)?;
    let email = setup_value("ARC_SETUP_ADMIN_EMAIL", "Administrator email: ", false)?;
    let password = setup_value("ARC_SETUP_ADMIN_PASSWORD", "Administrator password: ", true)?;
    let password_hash = crate::auth::hash_password(&password)?;
    let mut bytes = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let id = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    stack
        .command_bus
        .dispatch(
            UserCommand::Register {
                id: id.clone(),
                name,
                email,
                password_hash,
            },
            CommandContext::system(),
        )
        .await
        .map_err(|error| anyhow::anyhow!("failed to create first administrator: {error}"))?;
    let row = stack
        .read_model_store
        .get(USERS_VIEW, &id)
        .await
        .map_err(|error| anyhow::anyhow!("failed to verify first administrator: {error}"))?;
    if row
        .and_then(|value| value["active"].as_bool())
        .ne(&Some(true))
    {
        bail!("first administrator command completed but users_view was not projected");
    }
    println!("Created the first administrator.");
    Ok(())
}

#[allow(dead_code)]
fn setup_value(variable: &str, prompt: &str, secret: bool) -> anyhow::Result<String> {
    if let Ok(value) = env::var(variable) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    if !io::stdin().is_terminal() {
        bail!("fresh setup requires {variable}; provide ARC_SETUP_ADMIN_NAME, ARC_SETUP_ADMIN_EMAIL, and ARC_SETUP_ADMIN_PASSWORD in noninteractive environments");
    }
    if secret {
        return rpassword::prompt_password(prompt).context("failed to read administrator password");
    }
    print!("{prompt}");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim().to_owned();
    if value.is_empty() {
        bail!("{variable} cannot be empty");
    }
    Ok(value)
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
        .register_aggregate::<UserAggregate>()
        .register_projector(UserProjector, USERS_VIEW)
        // arc:resource-registrations
        .register_routes(routes::config)
        .serve(host.clone(), port)
        .await
        .with_context(|| {
            format!(
                "could not start on {host}:{port}; the port may be busy (change APP_PORT in .env)"
            )
        })
}
