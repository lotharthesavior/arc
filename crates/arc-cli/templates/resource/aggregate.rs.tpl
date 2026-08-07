use super::commands::{{Type}}Command;
use super::events::{{{Type}}Created, {{Type}}Renamed, CREATED, DELETED, RENAMED};
use arc_core::aggregate::Aggregate;
use arc_core::event::{Event, NewEvent};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum {{Type}}Error {
    #[error("{{module}} already exists")]
    AlreadyExists,
    #[error("{{module}} not found")]
    NotFound,
    #[error("name cannot be empty")]
    EmptyName,
    #[error("could not serialize event payload: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Default, Serialize, Deserialize)]
pub struct {{Type}}Aggregate {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: i64,
    pub exists: bool,
}

#[async_trait]
impl Aggregate for {{Type}}Aggregate {
    type Command = {{Type}}Command;
    type Event = ();
    type Error = {{Type}}Error;

    fn aggregate_type() -> &'static str {
        "{{Type}}"
    }

    fn version(&self) -> i64 {
        self.version
    }

    async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
        match command {
            {{Type}}Command::Create { id, name } => {
                if self.exists {
                    return Err({{Type}}Error::AlreadyExists);
                }
                validate_name(&name)?;
                Ok(vec![Event::new(NewEvent {
                    aggregate_type: Self::aggregate_type(),
                    aggregate_id: &id,
                    sequence: self.version + 1,
                    event_type: CREATED,
                    payload: serde_json::to_value({{Type}}Created {
                        id: id.clone(),
                        name,
                    })?,
                })])
            }
            {{Type}}Command::Rename { id, name } => {
                if !self.exists {
                    return Err({{Type}}Error::NotFound);
                }
                validate_name(&name)?;
                Ok(vec![Event::new(NewEvent {
                    aggregate_type: Self::aggregate_type(),
                    aggregate_id: &id,
                    sequence: self.version + 1,
                    event_type: RENAMED,
                    payload: serde_json::to_value({{Type}}Renamed { name })?,
                })])
            }
            {{Type}}Command::Delete { id } => {
                if !self.exists {
                    return Err({{Type}}Error::NotFound);
                }
                Ok(vec![Event::new(NewEvent {
                    aggregate_type: Self::aggregate_type(),
                    aggregate_id: &id,
                    sequence: self.version + 1,
                    event_type: DELETED,
                    payload: serde_json::Value::Object(Default::default()),
                })])
            }
        }
    }

    fn apply(&mut self, event: &Event) {
        self.version = event.sequence;
        match event.event_type.as_str() {
            CREATED => {
                self.id = event
                    .payload
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned);
                self.name = event
                    .payload
                    .get("name")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned);
                self.exists = true;
            }
            RENAMED => {
                self.name = event
                    .payload
                    .get("name")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned);
            }
            DELETED => self.exists = false,
            _ => {}
        }
    }

    fn to_snapshot(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self).ok()
    }

    fn from_snapshot(state: serde_json::Value) -> Option<Self> {
        serde_json::from_value(state).ok()
    }
}

fn validate_name(name: &str) -> Result<(), {{Type}}Error> {
    if name.trim().is_empty() {
        Err({{Type}}Error::EmptyName)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_emits_created_event() {
        let events = {{Type}}Aggregate::default()
            .handle({{Type}}Command::Create {
                id: "resource-1".to_string(),
                name: "First resource".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, CREATED);
        assert_eq!(events[0].sequence, 1);
    }

    #[tokio::test]
    async fn rename_requires_an_existing_resource() {
        let result = {{Type}}Aggregate::default()
            .handle({{Type}}Command::Rename {
                id: "missing".to_string(),
                name: "New name".to_string(),
            })
            .await;

        assert!(matches!(result, Err({{Type}}Error::NotFound)));
    }
}
