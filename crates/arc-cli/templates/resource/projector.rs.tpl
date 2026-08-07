use super::events::{CREATED, DELETED, RENAMED};
use arc_core::event::Event;
use arc_core::projection::{ProjectionError, ProjectionResult, Projector};
use arc_core::read_model_store::{ReadModelStore, Upsert};
use async_trait::async_trait;
use serde_json::json;

pub const {{CONSTANT}}_VIEW: &str = "{{view}}_view";

pub struct {{Type}}Projector;

#[async_trait]
impl Projector for {{Type}}Projector {
    fn name(&self) -> &str {
        "{{Type}}Projector"
    }

    fn handles(&self) -> Vec<String> {
        vec![
            CREATED.to_string(),
            RENAMED.to_string(),
            DELETED.to_string(),
        ]
    }

    async fn apply(&self, event: &Event, store: &dyn ReadModelStore) -> ProjectionResult<()> {
        match event.event_type.as_str() {
            CREATED => {
                let row = json!({
                    "id": event.aggregate_id,
                    "name": payload_name(event)?,
                    "version": event.sequence,
                });
                store
                    .upsert(Upsert::new({{CONSTANT}}_VIEW, &event.aggregate_id, row))
                    .await
                    .map_err(|error| projection_error(self, event, error.to_string()))?;
            }
            RENAMED => {
                let Some(mut row) = store
                    .get({{CONSTANT}}_VIEW, &event.aggregate_id)
                    .await
                    .map_err(|error| projection_error(self, event, error.to_string()))?
                else {
                    return Ok(());
                };
                row["name"] = json!(payload_name(event)?);
                row["version"] = json!(event.sequence);
                store
                    .upsert(Upsert::new({{CONSTANT}}_VIEW, &event.aggregate_id, row))
                    .await
                    .map_err(|error| projection_error(self, event, error.to_string()))?;
            }
            DELETED => store
                .delete({{CONSTANT}}_VIEW, &event.aggregate_id)
                .await
                .map_err(|error| projection_error(self, event, error.to_string()))?,
            _ => {}
        }
        Ok(())
    }
}

fn payload_name(event: &Event) -> ProjectionResult<&str> {
    event
        .payload
        .get("name")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ProjectionError::other("event payload missing string field 'name'"))
}

fn projection_error(
    projector: &{{Type}}Projector,
    event: &Event,
    message: String,
) -> ProjectionError {
    ProjectionError::handle_failed(
        projector.name(),
        &event.event_type,
        event.event_id.to_string(),
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_every_generated_event() {
        assert_eq!(
            {{Type}}Projector.handles(),
            vec![
                CREATED.to_string(),
                RENAMED.to_string(),
                DELETED.to_string()
            ]
        );
    }
}
