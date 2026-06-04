//! NATS JetStream event bus backend for Arc events.

use arc_core::event::Event;
use arc_core::event_bus::{EventBus, EventBusError, EventBusResult, EventHandler};
use async_nats::jetstream;
use async_nats::{HeaderMap, HeaderValue};
use async_trait::async_trait;

const EVENTS_SUBJECT: &str = "events.>";
const MSG_ID_HEADER: &str = "Nats-Msg-Id";

/// Publishes full [`Event`] payloads to a NATS JetStream stream.
pub struct NatsEventBus {
    jetstream: jetstream::Context,
    stream: String,
}

impl NatsEventBus {
    /// Connect to NATS and idempotently ensure the event stream exists.
    pub async fn new(url: &str, stream: &str) -> EventBusResult<Self> {
        let client = async_nats::connect(url)
            .await
            .map_err(|error| EventBusError::other(format!("NATS connection failed: {error}")))?;
        let jetstream = jetstream::new(client);
        let stream = stream.to_string();

        jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: stream.clone(),
                subjects: vec![EVENTS_SUBJECT.to_string()],
                ..Default::default()
            })
            .await
            .map_err(|error| {
                EventBusError::other(format!("JetStream stream setup failed: {error}"))
            })?;

        Ok(Self { jetstream, stream })
    }

    /// The configured JetStream stream name.
    pub fn stream(&self) -> &str {
        &self.stream
    }
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish(&self, events: Vec<Event>) -> EventBusResult<()> {
        for event in events {
            let subject = subject_for(&event);
            let payload = serde_json::to_vec(&event).map_err(|error| {
                EventBusError::other(format!("Event serialization failed: {error}"))
            })?;
            let mut headers = HeaderMap::new();
            headers.insert(MSG_ID_HEADER, HeaderValue::from(event.event_id.to_string()));

            let ack = self
                .jetstream
                .publish_with_headers(subject, headers, payload.into())
                .await
                .map_err(|error| {
                    EventBusError::other(format!("JetStream publish failed: {error}"))
                })?;

            ack.await.map_err(|error| {
                EventBusError::other(format!("JetStream publish ack failed: {error}"))
            })?;
        }

        Ok(())
    }

    async fn subscribe(&mut self, _handler: Box<dyn EventHandler>) -> EventBusResult<()> {
        Ok(())
    }
}

/// Map an event to `events.<aggregate_type>.<event_type>` using lowercase snake_case.
pub fn subject_for(event: &Event) -> String {
    format!(
        "events.{}.{}",
        to_snake_case(&event.aggregate_type),
        to_snake_case(&event.event_type)
    )
}

fn to_snake_case(input: &str) -> String {
    let mut output = String::new();
    let mut previous_kind = CharacterKind::Separator;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        if character.is_ascii_alphanumeric() {
            let is_upper = character.is_ascii_uppercase();
            let next_is_lower = characters
                .peek()
                .is_some_and(|next| next.is_ascii_lowercase());

            let should_insert_separator = !output.is_empty()
                && is_upper
                && (matches!(previous_kind, CharacterKind::LowerOrDigit)
                    || (next_is_lower && matches!(previous_kind, CharacterKind::Upper)));

            if should_insert_separator {
                output.push('_');
            }

            output.push(character.to_ascii_lowercase());
            previous_kind = if is_upper {
                CharacterKind::Upper
            } else {
                CharacterKind::LowerOrDigit
            };
        } else if !output.is_empty() && !matches!(previous_kind, CharacterKind::Separator) {
            output.push('_');
            previous_kind = CharacterKind::Separator;
        }
    }

    while output.ends_with('_') {
        output.pop();
    }

    output
}

#[derive(Clone, Copy)]
enum CharacterKind {
    Upper,
    LowerOrDigit,
    Separator,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn subject_for_uses_convention_shape() {
        let event = Event::new(
            "User",
            "user-1",
            1,
            "UserRegistered",
            json!({"email": "test@example.com"}),
        );

        assert_eq!(subject_for(&event), "events.user.user_registered");
    }

    #[test]
    fn subject_for_normalizes_existing_separators() {
        let event = Event::new(
            "Billing Account",
            "account-1",
            1,
            "Payment-Method Updated",
            json!({}),
        );

        assert_eq!(
            subject_for(&event),
            "events.billing_account.payment_method_updated"
        );
    }

    #[test]
    fn snake_case_handles_digits() {
        assert_eq!(to_snake_case("User2FAEnabled"), "user2_fa_enabled");
    }
}
