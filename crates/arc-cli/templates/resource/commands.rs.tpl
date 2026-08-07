use arc_core::aggregate::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum {{Type}}Command {
    Create { id: String, name: String },
    Rename { id: String, name: String },
    Delete { id: String },
}

impl Command for {{Type}}Command {
    fn aggregate_id(&self) -> &str {
        match self {
            Self::Create { id, .. } | Self::Rename { id, .. } | Self::Delete { id } => id,
        }
    }
}
