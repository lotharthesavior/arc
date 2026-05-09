#!/usr/bin/env bash
# Scaffold a new aggregate under crates/nineties-app/src/domain/<entity>/
# Usage: scripts/new-aggregate.sh <Entity>
#
# Follows CONVENTIONS.md: creates aggregate/commands/events with skeleton
# impls for Aggregate + Command traits.
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <Entity>"
    echo "Example: $0 Task"
    exit 1
fi

ENTITY_PASCAL="$1"
ENTITY_LOWER="$(echo "$ENTITY_PASCAL" | tr '[:upper:]' '[:lower:]')"

REPO_ROOT="$(git rev-parse --show-toplevel)"
TARGET_DIR="$REPO_ROOT/crates/nineties-app/src/domain/$ENTITY_LOWER"

if [[ -d "$TARGET_DIR" ]]; then
    echo "Error: $TARGET_DIR already exists"
    exit 1
fi

mkdir -p "$TARGET_DIR"

cat > "$TARGET_DIR/mod.rs" <<EOF
pub mod aggregate;
pub mod commands;
pub mod events;
EOF

cat > "$TARGET_DIR/commands.rs" <<EOF
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ${ENTITY_PASCAL}Command {
    Create${ENTITY_PASCAL} { id: String },
    // Add more commands here
}

impl nineties_core::aggregate::Command for ${ENTITY_PASCAL}Command {
    fn aggregate_id(&self) -> &str {
        match self {
            Self::Create${ENTITY_PASCAL} { id } => id,
        }
    }
}
EOF

cat > "$TARGET_DIR/events.rs" <<EOF
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ${ENTITY_PASCAL}DomainEvent {
    ${ENTITY_PASCAL}Created { id: String },
}
EOF

cat > "$TARGET_DIR/aggregate.rs" <<EOF
use crate::domain::${ENTITY_LOWER}::commands::${ENTITY_PASCAL}Command;
use async_trait::async_trait;
use nineties_core::{aggregate::Aggregate, event::Event};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ${ENTITY_PASCAL}AggregateError {
    #[error("${ENTITY_LOWER} already exists")]
    AlreadyExists,
    #[error("${ENTITY_LOWER} not found")]
    NotFound,
}

#[derive(Default)]
pub struct ${ENTITY_PASCAL}Aggregate {
    pub id: Option<String>,
    pub version: i64,
    pub exists: bool,
}

#[async_trait]
impl Aggregate for ${ENTITY_PASCAL}Aggregate {
    type Command = ${ENTITY_PASCAL}Command;
    type Event = ();
    type Error = ${ENTITY_PASCAL}AggregateError;

    fn aggregate_type() -> &'static str {
        "${ENTITY_PASCAL}"
    }

    fn version(&self) -> i64 {
        self.version
    }

    async fn handle(&self, cmd: Self::Command) -> Result<Vec<Event>, Self::Error> {
        match cmd {
            ${ENTITY_PASCAL}Command::Create${ENTITY_PASCAL} { ref id } => {
                if self.exists {
                    return Err(${ENTITY_PASCAL}AggregateError::AlreadyExists);
                }
                Ok(vec![Event::new(
                    "${ENTITY_PASCAL}",
                    id,
                    self.version + 1,
                    "${ENTITY_PASCAL}Created",
                    serde_json::json!({ "id": id }),
                )])
            }
        }
    }

    fn apply(&mut self, event: &Event) {
        self.version = event.sequence;
        match event.event_type.as_str() {
            "${ENTITY_PASCAL}Created" => {
                self.id = Some(event.payload["id"].as_str().unwrap().to_string());
                self.exists = true;
            }
            _ => {}
        }
    }
}
EOF

echo "Created $TARGET_DIR"
echo
echo "Next steps:"
echo "  1. Add 'pub mod $ENTITY_LOWER;' to crates/nineties-app/src/domain/mod.rs"
echo "  2. Register a CommandBus<${ENTITY_PASCAL}Aggregate> in commands/serve.rs"
echo "  3. Add controller routes that dispatch ${ENTITY_PASCAL}Command"
