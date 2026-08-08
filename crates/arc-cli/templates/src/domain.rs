use arc_core::aggregate::{Aggregate, Command};
use arc_core::event::Event;
use async_trait::async_trait;
use thiserror::Error;

// arc:domain-modules
pub mod user;

#[derive(Default)]
pub struct AppAggregate {
    version: i64,
}

pub struct AppCommand {
    aggregate_id: String,
}

impl Command for AppCommand {
    fn aggregate_id(&self) -> &str {
        &self.aggregate_id
    }
}

#[derive(Debug, Error)]
#[error("application command failed")]
pub struct AppError;

#[async_trait]
impl Aggregate for AppAggregate {
    type Command = AppCommand;
    type Event = ();
    type Error = AppError;

    fn aggregate_type() -> &'static str {
        "App"
    }

    fn version(&self) -> i64 {
        self.version
    }

    async fn handle(&self, _command: Self::Command) -> Result<Vec<Event>, Self::Error> {
        Ok(Vec::new())
    }

    fn apply(&mut self, event: &Event) {
        self.version = event.sequence;
    }
}
