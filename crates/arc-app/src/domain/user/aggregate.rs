use crate::domain::user::commands::UserCommand;
use arc_core::{aggregate::Aggregate, event::Event};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserAggregateError {
    #[error("user already exists")]
    AlreadyExists,
    #[error("user not found")]
    NotFound,
    #[error("user already deleted")]
    AlreadyDeleted,
    #[error("invalid email format")]
    InvalidEmail,
}

#[derive(Default)]
pub struct UserAggregate {
    pub id: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub version: i64,
    pub exists: bool,
    pub deleted: bool,
}

#[async_trait]
impl Aggregate for UserAggregate {
    type Command = UserCommand;
    type Error = UserAggregateError;
    type Event = ();

    fn aggregate_type() -> &'static str {
        "User"
    }
    fn version(&self) -> i64 {
        self.version
    }

    async fn handle<'a>(&'a self, cmd: Self::Command) -> Result<Vec<Event>, Self::Error> {
        match cmd {
            UserCommand::RegisterUser {
                ref id,
                ref name,
                ref email,
                ref password_hash,
            } => {
                if self.exists {
                    return Err(UserAggregateError::AlreadyExists);
                }
                if !email.contains('@') {
                    return Err(UserAggregateError::InvalidEmail);
                }
                Ok(vec![Event::new(
                    "User",
                    id,
                    self.version + 1,
                    "UserRegistered",
                    serde_json::json!({ "id": id, "name": name,
                                        "email": email, "password_hash": password_hash }),
                )])
            }
            UserCommand::UpdateProfile { ref id, ref name } => {
                if !self.exists || self.deleted {
                    return Err(UserAggregateError::NotFound);
                }
                Ok(vec![Event::new(
                    "User",
                    id,
                    self.version + 1,
                    "ProfileUpdated",
                    serde_json::json!({ "name": name }),
                )])
            }
            UserCommand::ChangeEmail { ref id, ref email } => {
                if !self.exists || self.deleted {
                    return Err(UserAggregateError::NotFound);
                }
                if !email.contains('@') {
                    return Err(UserAggregateError::InvalidEmail);
                }
                Ok(vec![Event::new(
                    "User",
                    id,
                    self.version + 1,
                    "EmailChanged",
                    serde_json::json!({ "email": email }),
                )])
            }
            UserCommand::ChangePassword {
                ref id,
                ref password_hash,
            } => {
                if !self.exists || self.deleted {
                    return Err(UserAggregateError::NotFound);
                }
                Ok(vec![Event::new(
                    "User",
                    id,
                    self.version + 1,
                    "PasswordChanged",
                    serde_json::json!({ "password_hash": password_hash }),
                )])
            }
            UserCommand::DeleteUser { ref id } => {
                if !self.exists {
                    return Err(UserAggregateError::NotFound);
                }
                if self.deleted {
                    return Err(UserAggregateError::AlreadyDeleted);
                }
                Ok(vec![Event::new(
                    "User",
                    id,
                    self.version + 1,
                    "UserDeleted",
                    serde_json::json!({}),
                )])
            }
        }
    }

    fn apply(&mut self, event: &Event) {
        self.version = event.sequence;
        match event.event_type.as_str() {
            "UserRegistered" => {
                self.id = Some(event.payload["id"].as_str().unwrap().to_string());
                self.name = Some(event.payload["name"].as_str().unwrap().to_string());
                self.email = Some(event.payload["email"].as_str().unwrap().to_string());
                self.password_hash =
                    Some(event.payload["password_hash"].as_str().unwrap().to_string());
                self.exists = true;
            }
            "ProfileUpdated" => {
                self.name = Some(event.payload["name"].as_str().unwrap().to_string());
            }
            "EmailChanged" => {
                self.email = Some(event.payload["email"].as_str().unwrap().to_string());
            }
            "PasswordChanged" => {
                self.password_hash =
                    Some(event.payload["password_hash"].as_str().unwrap().to_string());
            }
            "UserDeleted" => {
                self.deleted = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::commands::UserCommand;

    #[tokio::test]
    async fn test_create_user_emits_user_registered_event() {
        let agg = UserAggregate::default();
        let cmd = UserCommand::RegisterUser {
            id: "uuid-123".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
        };

        let result = agg.handle(cmd).await;
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];

        assert_eq!(event.event_type, "UserRegistered");
        assert_eq!(event.aggregate_id, "uuid-123".to_string());
        assert_eq!(event.payload["email"], "john@example.com");
    }

    #[tokio::test]
    async fn test_create_user_returns_conflict_on_duplicate_user() {
        let agg = UserAggregate {
            exists: true,
            ..Default::default()
        };

        let cmd = UserCommand::RegisterUser {
            id: "uuid-123".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
        };

        let result = agg.handle(cmd).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "user already exists");
    }

    #[tokio::test]
    async fn test_update_profile_reflects_in_get() {
        let mut agg = UserAggregate::default();

        // setup state
        let create_event = Event::new(
            "User",
            "uuid-123",
            1,
            "UserRegistered",
            serde_json::json!({
                "id": "uuid-123", "name": "Old", "email": "o@e.c", "password_hash": "pw"
            }),
        );
        agg.apply(&create_event);

        let cmd = UserCommand::UpdateProfile {
            id: "uuid-123".to_string(),
            name: "New Name".to_string(),
        };

        let result = agg.handle(cmd).await.unwrap();
        assert_eq!(result[0].event_type, "ProfileUpdated");
        assert_eq!(result[0].payload["name"], "New Name");

        agg.apply(&result[0]);
        assert_eq!(agg.name.unwrap(), "New Name");
        assert_eq!(agg.version, 2);
    }
}
