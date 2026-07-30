//! Application-owned database migrations layered over Arc's pool helpers.

pub use arc_web::helpers::database::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};

/// The thin application's schema, embedded from its workspace migration set.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations");
