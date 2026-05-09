//! Seeds the default user (`jekyll@example.com`) by dispatching a
//! `UserCommand::RegisterUser` through the `CommandBus`. The legacy
//! direct-Diesel seeder has been retired alongside the `users` table.

use crate::domain::user::aggregate::UserAggregate;
use crate::domain::user::commands::UserCommand;
use crate::services::user_service::{lookup_aggregate_id_by_email_view, prepare_password};
use arc_core::command_bus::{CommandBus, CommandContext};
use arc_core::read_model_store::ReadModelStore;
use tracing::info;

/// Default user the framework seeds for local dev / test fixtures.
pub const DEFAULT_USER_EMAIL: &str = "jekyll@example.com";
pub const DEFAULT_USER_NAME: &str = "Jekyll";
pub const DEFAULT_USER_PASSWORD: &str = "password";

/// Seed the default user. Idempotent: if the projection already has a row
/// for the email, returns the existing aggregate id without dispatching.
pub async fn seed_default_user(
    command_bus: &CommandBus<UserAggregate>,
    rm_store: &dyn ReadModelStore,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(id) = lookup_aggregate_id_by_email_view(rm_store, DEFAULT_USER_EMAIL).await {
        info!(email = DEFAULT_USER_EMAIL, "User already seeded");
        return Ok(id);
    }

    info!("Creating default user");
    let id = uuid::Uuid::new_v4().to_string();
    let cmd = UserCommand::RegisterUser {
        id: id.clone(),
        name: DEFAULT_USER_NAME.to_string(),
        email: DEFAULT_USER_EMAIL.to_string(),
        password_hash: prepare_password(DEFAULT_USER_PASSWORD),
    };
    command_bus.dispatch(cmd, CommandContext::system()).await?;
    Ok(id)
}
