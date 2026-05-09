use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserCommand {
    RegisterUser {
        id: String,
        name: String,
        email: String,
        password_hash: String,
    },
    UpdateProfile {
        id: String,
        name: String,
    },
    ChangeEmail {
        id: String,
        email: String,
    },
    ChangePassword {
        id: String,
        password_hash: String,
    },
    DeleteUser {
        id: String,
    },
}

impl arc_core::aggregate::Command for UserCommand {
    fn aggregate_id(&self) -> &str {
        match self {
            Self::RegisterUser { id, .. } => id,
            Self::UpdateProfile { id, .. } => id,
            Self::ChangeEmail { id, .. } => id,
            Self::ChangePassword { id, .. } => id,
            Self::DeleteUser { id } => id,
        }
    }
}
