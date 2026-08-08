use arc_core::aggregate::Command;

#[allow(dead_code)]
pub enum UserCommand {
    Register {
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
    Deactivate {
        id: String,
    },
}

impl Command for UserCommand {
    fn aggregate_id(&self) -> &str {
        match self {
            Self::Register { id, .. }
            | Self::UpdateProfile { id, .. }
            | Self::ChangeEmail { id, .. }
            | Self::ChangePassword { id, .. }
            | Self::Deactivate { id } => id,
        }
    }
}
