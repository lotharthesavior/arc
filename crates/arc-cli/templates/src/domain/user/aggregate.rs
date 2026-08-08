use super::{commands::UserCommand, events::*};
use arc_core::{
    aggregate::Aggregate,
    event::{Event, NewEvent},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user already exists")]
    AlreadyExists,
    #[error("active user not found")]
    NotFound,
    #[error("name cannot be empty")]
    EmptyName,
    #[error("invalid email address")]
    InvalidEmail,
    #[error("password hash cannot be empty")]
    EmptyPasswordHash,
}

#[derive(Default, Serialize, Deserialize)]
pub struct UserAggregate {
    id: Option<String>,
    name: Option<String>,
    email: Option<String>,
    password_hash: Option<String>,
    version: i64,
    active: bool,
}

fn valid_name(value: &str) -> Result<(), UserError> {
    if value.trim().is_empty() {
        Err(UserError::EmptyName)
    } else {
        Ok(())
    }
}
fn normalized_email(value: &str) -> Result<String, UserError> {
    let value = value.trim().to_ascii_lowercase();
    if value
        .split_once('@')
        .is_some_and(|(left, right)| !left.is_empty() && right.contains('.'))
    {
        Ok(value)
    } else {
        Err(UserError::InvalidEmail)
    }
}
fn valid_hash(value: &str) -> Result<(), UserError> {
    if value.trim().is_empty() {
        Err(UserError::EmptyPasswordHash)
    } else {
        Ok(())
    }
}

#[async_trait]
impl Aggregate for UserAggregate {
    type Command = UserCommand;
    type Event = ();
    type Error = UserError;
    fn aggregate_type() -> &'static str {
        "User"
    }
    fn version(&self) -> i64 {
        self.version
    }
    async fn handle(&self, command: UserCommand) -> Result<Vec<Event>, UserError> {
        let (id, event_type, payload) = match command {
            UserCommand::Register {
                id,
                name,
                email,
                password_hash,
            } => {
                if self.version > 0 {
                    return Err(UserError::AlreadyExists);
                }
                valid_name(&name)?;
                let email = normalized_email(&email)?;
                valid_hash(&password_hash)?;
                let payload = serde_json::json!({"id":id,"name":name.trim(),"email":email,"password_hash":password_hash});
                (id, REGISTERED, payload)
            }
            UserCommand::UpdateProfile { id, name } => {
                if !self.active {
                    return Err(UserError::NotFound);
                }
                valid_name(&name)?;
                let payload = serde_json::json!({"name":name.trim()});
                (id, PROFILE_UPDATED, payload)
            }
            UserCommand::ChangeEmail { id, email } => {
                if !self.active {
                    return Err(UserError::NotFound);
                }
                let email = normalized_email(&email)?;
                (id, EMAIL_CHANGED, serde_json::json!({"email":email}))
            }
            UserCommand::ChangePassword { id, password_hash } => {
                if !self.active {
                    return Err(UserError::NotFound);
                }
                valid_hash(&password_hash)?;
                (
                    id,
                    PASSWORD_CHANGED,
                    serde_json::json!({"password_hash":password_hash}),
                )
            }
            UserCommand::Deactivate { id } => {
                if !self.active {
                    return Err(UserError::NotFound);
                }
                (id, DEACTIVATED, serde_json::json!({}))
            }
        };
        Ok(vec![Event::new(NewEvent {
            aggregate_type: Self::aggregate_type(),
            aggregate_id: &id,
            sequence: self.version + 1,
            event_type,
            payload,
        })])
    }
    fn apply(&mut self, event: &Event) {
        self.version = event.sequence;
        match event.event_type.as_str() {
            REGISTERED => {
                self.id = Some(event.aggregate_id.clone());
                self.name = event.payload["name"].as_str().map(str::to_owned);
                self.email = event.payload["email"].as_str().map(str::to_owned);
                self.password_hash = event.payload["password_hash"].as_str().map(str::to_owned);
                self.active = true;
            }
            PROFILE_UPDATED => self.name = event.payload["name"].as_str().map(str::to_owned),
            EMAIL_CHANGED => self.email = event.payload["email"].as_str().map(str::to_owned),
            PASSWORD_CHANGED => {
                self.password_hash = event.payload["password_hash"].as_str().map(str::to_owned)
            }
            DEACTIVATED => self.active = false,
            _ => {}
        }
    }
    fn to_snapshot(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self).ok()
    }
    fn from_snapshot(value: serde_json::Value) -> Option<Self> {
        serde_json::from_value(value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn registration_normalizes_email_and_does_not_redact_persisted_hash() {
        let events = UserAggregate::default()
            .handle(UserCommand::Register {
                id: "u1".into(),
                name: "Admin".into(),
                email: " ADMIN@Example.COM ".into(),
                password_hash: "$argon2id$hash".into(),
            })
            .await
            .unwrap();
        assert_eq!(events[0].payload["email"], "admin@example.com");
        assert_eq!(events[0].payload["password_hash"], "$argon2id$hash");
    }
}
