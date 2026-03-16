use dotenv::dotenv;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Mutex;
use std::{env, fs};
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;
mod http {
    pub mod middlewares {
        pub mod auth_middleware;
        pub mod jwt_middleware;
        pub mod rate_limit_middleware;
    }

    pub mod controllers {
        pub mod admin_controller;
        pub mod api_controller;
        pub mod auth_controller;
        pub mod home_controller;
    }
}

mod database {
    pub mod backend;

    pub mod seeders {
        pub mod create_users;

        pub mod traits {
            pub mod seeder;
        }
    }
}

mod models {
    pub mod user;
}

mod schema;

mod helpers {
    pub mod config;
    pub mod csrf;
    pub mod database;
    pub mod general;
    pub mod jwt;
    pub mod rate_limit;
    pub mod session;
    pub mod template;
    pub mod test;
}

mod services {
    pub mod user_service;
}

mod validation;

mod commands;
pub mod websocket;
/// Shared application state accessible by all request handlers via `web::Data`.
#[derive(Debug)]
pub struct AppState {
    app_name: Mutex<String>,
    _user_id: Mutex<Option<i32>>,
}

/// Checks that the application environment is healthy (e.g., `.env` file exists).
/// Copies `.env.example` to `.env` if no `.env` file is found.
fn check_app_health() {
    info!("Checking app health");
    if !fs::exists(PathBuf::from(".env")).unwrap() {
        info!("Creating .env file from .env.example");
        fs::copy(PathBuf::from(".env.example"), PathBuf::from(".env"))
            .expect("Failed to copy .env.example to .env");
    }
}

/// Validates that all required environment variables are set at startup.
/// Fails fast with clear error messages instead of panicking at random points.
fn validate_environment() {
    let required_vars = ["APP_URL", "SECRET_KEY", "DATABASE_URL"];
    let mut missing = Vec::new();
    for var in required_vars {
        if env::var(var).is_err() {
            missing.push(var);
        }
    }
    if !missing.is_empty() {
        error!(
            "Missing required environment variables: {}. Check your .env file.",
            missing.join(", ")
        );
        exit(1);
    }
    debug!("All required environment variables present");
}

/// Verifies the configured database backend is valid and that the database
/// exists at the configured `DATABASE_URL`. Exits with code 1 and a helpful
/// message otherwise.
pub fn check_database_health() {
    info!("Checking database health");

    if let Err(err) = database::backend::validate_backend_configuration() {
        error!("{}", err);
        exit(1);
    }

    let database: String = database::backend::database_url();

    if !database::backend::database_exists(&database) {
        error!("Database not found at: {}", database);
        error!("Please run `cargo run migrate` to create the database");
        exit(1);
    }
    debug!("Database found at: {}", database);
}

/// Validates the JWT configuration when JWT auth is enabled. Exits with code 1
/// if the configuration is invalid.
fn check_jwt_health() {
    if let Err(err) = helpers::jwt::validate_jwt_configuration() {
        error!("{}", err);
        exit(1);
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing subscriber with environment-based filtering
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nineties=info,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    check_app_health();

    dotenv().ok();

    validate_environment();

    check_jwt_health();

    info!("Nineties application starting");

    let args: Vec<String> = env::args().collect();
    let app_url: String = env::var("APP_URL").expect("APP_URL must be set");
    let app_port: u16 = env::var("APP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("APP_PORT must be a valid u16");

    let mut command: &str = "serve";
    if args.len() > 1 {
        command = args[1].as_str();
    }

    match command {
        "serve" => commands::serve::run(app_url.clone(), app_port).await,
        "develop" => {
            check_database_health();
            commands::develop::run_development().await
        }
        "migrate" => commands::migrate::run(&args),
        "seed" => commands::seed::run(),
        _ => {
            error!("Unknown command: {}", command);
            Ok(())
        }
    }
}
