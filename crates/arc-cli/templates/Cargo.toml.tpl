[package]
name = "{{project-name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-files = "0.6"
actix-web = "4"
anyhow = "1"
arc-core = "{{arc-version}}"
arc-web = "{{arc-version}}"
async-trait = "0.1"
diesel = { version = "2.2", features = ["sqlite", "r2d2"] }
diesel_migrations = "2.2"
dotenv = "0.15"
rand = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tera = "1.20"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
