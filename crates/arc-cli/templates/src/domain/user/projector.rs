use super::events::*;
use arc_core::{
    event::Event,
    projection::{ProjectionError, ProjectionResult, Projector},
    read_model_store::{ReadModelStore, Upsert},
};
use async_trait::async_trait;
use serde_json::json;

pub const USERS_VIEW: &str = "users_view";
pub struct UserProjector;

#[async_trait]
impl Projector for UserProjector {
    fn name(&self) -> &str {
        "UserProjector"
    }
    fn handles(&self) -> Vec<String> {
        [
            REGISTERED,
            PROFILE_UPDATED,
            EMAIL_CHANGED,
            PASSWORD_CHANGED,
            DEACTIVATED,
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
    async fn apply(&self, event: &Event, store: &dyn ReadModelStore) -> ProjectionResult<()> {
        let mut row = if event.event_type == REGISTERED {
            json!({"id":event.aggregate_id,"name":field(event,"name")?,"email":field(event,"email")?,"password_hash":field(event,"password_hash")?,"active":true,"version":event.sequence})
        } else if let Some(row) = store
            .get(USERS_VIEW, &event.aggregate_id)
            .await
            .map_err(|e| fail(self, event, e.to_string()))?
        {
            row
        } else {
            return Ok(());
        };
        match event.event_type.as_str() {
            PROFILE_UPDATED => row["name"] = json!(field(event, "name")?),
            EMAIL_CHANGED => row["email"] = json!(field(event, "email")?),
            PASSWORD_CHANGED => row["password_hash"] = json!(field(event, "password_hash")?),
            DEACTIVATED => row["active"] = json!(false),
            _ => {}
        }
        row["version"] = json!(event.sequence);
        store
            .upsert(Upsert::new(USERS_VIEW, &event.aggregate_id, row))
            .await
            .map_err(|e| fail(self, event, e.to_string()))
    }
}
fn field<'a>(event: &'a Event, name: &str) -> ProjectionResult<&'a str> {
    event
        .payload
        .get(name)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ProjectionError::other(format!("event payload missing '{name}'")))
}
fn fail(projector: &UserProjector, event: &Event, message: String) -> ProjectionError {
    ProjectionError::handle_failed(
        projector.name(),
        &event.event_type,
        event.event_id.to_string(),
        message,
    )
}
