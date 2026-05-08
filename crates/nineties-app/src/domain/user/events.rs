use serde::{Deserialize, Serialize};

/// Typed domain event payloads. Event payloads are stored on `Event::payload` as
/// untyped JSON, but this enum documents the wire shape and will be used as a
/// deserialization target by Step 2 projections that need typed access.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserDomainEvent {
    UserRegistered {
        id: String,
        name: String,
        email: String,
        password_hash: String,
    },
    ProfileUpdated {
        name: String,
    },
    EmailChanged {
        email: String,
    },
    PasswordChanged {
        password_hash: String,
    },
    UserDeleted,
}
