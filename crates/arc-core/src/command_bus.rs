//! # Command Bus Module
//!
//! Coordinates command handling through aggregates with event persistence and publishing.
//!
//! ## Flow
//!
//! 1. Load events from `EventStore` for the target aggregate
//! 2. Reconstruct aggregate state via `Aggregate::from_events()`
//! 3. Handle command through `Aggregate::handle()` to produce new events
//!    (events leave `handle()` with `audit = AuditMetadata::pending()`)
//! 4. **Stamp** each event with a fully-validated [`AuditMetadata`] derived from
//!    the request-scoped [`CommandContext`]
//! 5. Append events to `EventStore` with optimistic concurrency check; the
//!    store re-validates audit (defense-in-depth)
//! 6. Publish events to `EventBus` for projections and side effects
//!
//! ## Audit invariant
//!
//! Every persisted event carries [`AuditMetadata`] (HIPAA §164.312(b)).
//! `dispatch` requires a [`CommandContext`] argument; production code cannot
//! omit it. Internal jobs use [`CommandContext::system`].

use crate::aggregate::{Aggregate, Command};
use crate::audit::{AuditError, AuditMetadata, SYSTEM_ACTOR};
use crate::event::Event;
use crate::event_bus::{EventBus, EventBusError};
use crate::event_store::{EventStore, EventStoreError, VersionCheck};
use std::marker::PhantomData;
use thiserror::Error;
use uuid::Uuid;

/// Request-scoped context for command dispatch.
///
/// Constructed once per HTTP request (or by [`CommandContext::system`] for
/// internal jobs). Carries the data needed to build [`AuditMetadata`] for every
/// event the command produces.
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// Required. Aggregate UUID, `"system"`, `"anonymous"`, or
    /// `"legacy-pre-hipaa"`. Must be non-empty.
    pub actor_id: String,

    /// Optional session id (paired with HIPAA-4 server-side session store).
    pub session_id: Option<String>,

    /// Source IP. `None` for system jobs.
    pub source_ip: Option<String>,

    /// `User-Agent` header.
    pub user_agent: Option<String>,

    /// Required. Groups every event from one logical request together.
    pub correlation_id: Uuid,

    /// Optional event id that triggered this command (saga / projection follow-up).
    pub causation_id: Option<Uuid>,
}

impl CommandContext {
    /// Convenience for an authenticated HTTP request. Synthesizes
    /// `correlation_id` if not supplied by the caller.
    pub fn for_actor(actor_id: impl Into<String>) -> Self {
        Self {
            actor_id: actor_id.into(),
            session_id: None,
            source_ip: None,
            user_agent: None,
            correlation_id: Uuid::new_v4(),
            causation_id: None,
        }
    }

    /// System-internal context (cron, seeders, migrations).
    pub fn system() -> Self {
        Self::for_actor(SYSTEM_ACTOR)
    }

    /// Build a context whose causation chains from a triggering event. Inherit
    /// the upstream `correlation_id` so the saga is traceable end-to-end.
    pub fn caused_by(actor_id: impl Into<String>, triggering: &Event) -> Self {
        Self {
            actor_id: actor_id.into(),
            session_id: None,
            source_ip: None,
            user_agent: None,
            correlation_id: triggering.audit.correlation_id,
            causation_id: Some(triggering.event_id),
        }
    }

    /// Convert into the [`AuditMetadata`] that will stamp produced events.
    /// Sets `timestamp_utc_us = now`. Validates before returning.
    pub fn to_audit(&self) -> Result<AuditMetadata, AuditError> {
        let m = AuditMetadata {
            actor_id: self.actor_id.clone(),
            actor_session_id: self.session_id.clone(),
            source_ip: self.source_ip.clone(),
            user_agent: self.user_agent.clone(),
            timestamp_utc_us: crate::audit::now_us(),
            causation_id: self.causation_id,
            correlation_id: self.correlation_id,
        };
        m.validate()?;
        Ok(m)
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for CommandContext {
    fn default() -> Self {
        Self::for_actor("test")
    }
}

/// Errors that can occur during command bus operations.
#[derive(Debug, Error)]
pub enum CommandBusError {
    #[error("Failed to load aggregate '{aggregate_id}': {source}")]
    LoadFailed {
        aggregate_id: String,
        #[source]
        source: EventStoreError,
    },

    #[error("Command handling failed for aggregate '{aggregate_id}': {message}")]
    HandleFailed {
        aggregate_id: String,
        message: String,
    },

    #[error("Failed to append events for aggregate '{aggregate_id}': {source}")]
    AppendFailed {
        aggregate_id: String,
        #[source]
        source: EventStoreError,
    },

    #[error("Failed to publish events for aggregate '{aggregate_id}': {source}")]
    PublishFailed {
        aggregate_id: String,
        #[source]
        source: EventBusError,
    },

    #[error("Audit metadata validation failed for aggregate '{aggregate_id}': {source}")]
    InvalidAudit {
        aggregate_id: String,
        #[source]
        source: AuditError,
    },

    #[error("Command bus error: {message}")]
    Other { message: String },
}

impl CommandBusError {
    pub fn handle_failed(aggregate_id: impl Into<String>, message: impl Into<String>) -> Self {
        CommandBusError::HandleFailed {
            aggregate_id: aggregate_id.into(),
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        CommandBusError::Other {
            message: message.into(),
        }
    }
}

pub type CommandBusResult<T> = Result<T, CommandBusError>;

/// Command bus for dispatching commands to aggregates.
pub struct CommandBus<A: Aggregate> {
    event_store: Box<dyn EventStore>,
    event_bus: Box<dyn EventBus>,
    _phantom: PhantomData<A>,
}

impl<A: Aggregate> CommandBus<A> {
    pub fn new(event_store: Box<dyn EventStore>, event_bus: Box<dyn EventBus>) -> Self {
        Self {
            event_store,
            event_bus,
            _phantom: PhantomData,
        }
    }

    /// Dispatch a command with its request-scoped [`CommandContext`].
    ///
    /// Steps: load → reconstruct → handle → **stamp audit** → append → publish.
    /// The aggregate's `handle()` returns events with placeholder audit; this
    /// method overwrites it with a single validated [`AuditMetadata`] per
    /// dispatch (all events from one command share the same audit stamp).
    pub async fn dispatch(
        &self,
        command: A::Command,
        context: CommandContext,
    ) -> CommandBusResult<Vec<Event>> {
        let aggregate_id = command.aggregate_id().to_string();

        // Step 1: Load existing events
        let events = self
            .event_store
            .load(&aggregate_id)
            .await
            .map_err(|source| CommandBusError::LoadFailed {
                aggregate_id: aggregate_id.clone(),
                source,
            })?;

        let current_version = events.last().map(|e| e.sequence).unwrap_or(0);

        // Step 2: Reconstruct
        let aggregate = A::from_events(events);

        // Step 3: Handle
        let new_events = aggregate
            .handle(command)
            .await
            .map_err(|e| CommandBusError::handle_failed(&aggregate_id, e.to_string()))?;

        if new_events.is_empty() {
            return Ok(vec![]);
        }

        // Step 4: Stamp audit (one validated stamp shared across all produced events)
        let audit = context
            .to_audit()
            .map_err(|source| CommandBusError::InvalidAudit {
                aggregate_id: aggregate_id.clone(),
                source,
            })?;
        let new_events: Vec<Event> = new_events
            .into_iter()
            .map(|e| e.with_audit(audit.clone()))
            .collect();

        // Step 5: Append (store re-validates audit defense-in-depth)
        let version_check = if current_version == 0 {
            VersionCheck::New
        } else {
            VersionCheck::Expected(current_version)
        };

        self.event_store
            .append(&aggregate_id, version_check, new_events.clone())
            .await
            .map_err(|source| CommandBusError::AppendFailed {
                aggregate_id: aggregate_id.clone(),
                source,
            })?;

        // Step 6: Publish
        self.event_bus
            .publish(new_events.clone())
            .await
            .map_err(|source| CommandBusError::PublishFailed {
                aggregate_id: aggregate_id.clone(),
                source,
            })?;

        Ok(new_events)
    }

    pub fn event_store(&self) -> &dyn EventStore {
        self.event_store.as_ref()
    }

    pub fn event_bus(&self) -> &dyn EventBus {
        self.event_bus.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::Aggregate;
    use crate::event::Event;
    use crate::event_bus::{EventHandler, InProcessEventBus};
    use crate::event_store::{
        EventStore, EventStoreError, EventStoreResult, InMemoryEventStore, VersionCheck,
    };
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    #[derive(Debug, Clone, PartialEq)]
    struct CounterCommand {
        id: String,
        increment: i64,
    }

    impl Command for CounterCommand {
        fn aggregate_id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Debug, Clone, Default)]
    struct CounterAggregate {
        id: Option<String>,
        value: i64,
        version: i64,
    }

    #[derive(Debug, thiserror::Error)]
    enum CounterError {
        #[error("Negative increment not allowed")]
        NegativeIncrement,
    }

    #[async_trait]
    impl Aggregate for CounterAggregate {
        type Command = CounterCommand;
        type Event = ();
        type Error = CounterError;

        fn aggregate_type() -> &'static str {
            "Counter"
        }

        fn version(&self) -> i64 {
            self.version
        }

        async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
            if command.increment < 0 {
                return Err(CounterError::NegativeIncrement);
            }
            Ok(vec![Event::new(
                "Counter",
                &command.id,
                self.version + 1,
                "CounterIncremented",
                json!({ "increment": command.increment }),
            )])
        }

        fn apply(&mut self, event: &Event) {
            if event.event_type == "CounterIncremented" {
                self.id = Some(event.aggregate_id.clone());
                self.value += event.payload["increment"].as_i64().unwrap_or(0);
                self.version = event.sequence;
            }
        }
    }

    fn ctx() -> CommandContext {
        CommandContext::for_actor("test-actor")
    }

    #[tokio::test]
    async fn test_command_bus_new() {
        let _bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
    }

    #[tokio::test]
    async fn test_dispatch_first_command() {
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        let cmd = CounterCommand {
            id: "counter-1".into(),
            increment: 5,
        };
        let events = bus.dispatch(cmd, ctx()).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "CounterIncremented");
        assert_eq!(events[0].sequence, 1);
        assert!(!events[0].audit.is_pending());
        assert_eq!(events[0].audit.actor_id, "test-actor");
    }

    #[tokio::test]
    async fn test_dispatch_multiple_commands() {
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        bus.dispatch(
            CounterCommand {
                id: "counter-1".into(),
                increment: 5,
            },
            ctx(),
        )
        .await
        .unwrap();
        let events = bus
            .dispatch(
                CounterCommand {
                    id: "counter-1".into(),
                    increment: 3,
                },
                ctx(),
            )
            .await
            .unwrap();
        assert_eq!(events[0].sequence, 2);
    }

    #[tokio::test]
    async fn test_dispatch_validates_command() {
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        let result = bus
            .dispatch(
                CounterCommand {
                    id: "c1".into(),
                    increment: -5,
                },
                ctx(),
            )
            .await;
        match result.unwrap_err() {
            CommandBusError::HandleFailed { message, .. } => {
                assert!(message.contains("Negative increment"));
            }
            other => panic!("expected HandleFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_dispatch_publishes_events() {
        let mut event_bus = InProcessEventBus::new();
        let published = Arc::new(TokioMutex::new(Vec::new()));
        let captured = published.clone();

        struct H {
            captured: Arc<TokioMutex<Vec<String>>>,
        }
        #[async_trait]
        impl EventHandler for H {
            fn handles(&self) -> Vec<String> {
                vec!["CounterIncremented".to_string()]
            }
            async fn handle(
                &self,
                event: &Event,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                self.captured.lock().await.push(event.event_type.clone());
                Ok(())
            }
        }
        event_bus.subscribe(Box::new(H { captured })).await.unwrap();

        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(event_bus),
        );
        bus.dispatch(
            CounterCommand {
                id: "c1".into(),
                increment: 5,
            },
            ctx(),
        )
        .await
        .unwrap();
        assert_eq!(published.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn test_dispatch_empty_events() {
        #[derive(Default)]
        struct NoOpAggregate {
            version: i64,
        }
        struct NoOpCommand {
            id: String,
        }
        impl Command for NoOpCommand {
            fn aggregate_id(&self) -> &str {
                &self.id
            }
        }
        #[derive(Debug, thiserror::Error)]
        #[error("noop")]
        struct NoOpErr;
        #[async_trait]
        impl Aggregate for NoOpAggregate {
            type Command = NoOpCommand;
            type Event = ();
            type Error = NoOpErr;
            fn aggregate_type() -> &'static str {
                "NoOp"
            }
            fn version(&self) -> i64 {
                self.version
            }
            async fn handle(&self, _: Self::Command) -> Result<Vec<Event>, Self::Error> {
                Ok(vec![])
            }
            fn apply(&mut self, _: &Event) {}
        }
        let bus = CommandBus::<NoOpAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        let events = bus
            .dispatch(NoOpCommand { id: "n1".into() }, ctx())
            .await
            .unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_aggregate_state_reconstruction() {
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        bus.dispatch(
            CounterCommand {
                id: "c1".into(),
                increment: 5,
            },
            ctx(),
        )
        .await
        .unwrap();
        bus.dispatch(
            CounterCommand {
                id: "c1".into(),
                increment: 3,
            },
            ctx(),
        )
        .await
        .unwrap();
        let events = bus.event_store().load("c1").await.unwrap();
        let agg = CounterAggregate::from_events(events);
        assert_eq!(agg.value, 8);
        assert_eq!(agg.version, 2);
    }

    #[tokio::test]
    async fn test_optimistic_concurrency() {
        struct ConflictingStore;
        #[async_trait]
        impl EventStore for ConflictingStore {
            async fn append(
                &self,
                aggregate_id: &str,
                version_check: VersionCheck,
                _events: Vec<Event>,
            ) -> EventStoreResult<()> {
                if let Some(expected) = version_check.version() {
                    Err(EventStoreError::ConcurrencyConflict {
                        aggregate_id: aggregate_id.to_string(),
                        expected,
                        actual: expected + 1,
                    })
                } else {
                    Ok(())
                }
            }
            async fn load(&self, _: &str) -> EventStoreResult<Vec<Event>> {
                Ok(vec![])
            }
            async fn load_from(&self, _: &str, _: i64) -> EventStoreResult<Vec<Event>> {
                Ok(vec![])
            }
            async fn stream_all(&self, _: i64) -> EventStoreResult<Vec<Event>> {
                Ok(vec![])
            }
            async fn get_version(&self, _: &str) -> EventStoreResult<i64> {
                Ok(0)
            }
        }
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(ConflictingStore),
            Box::new(InProcessEventBus::new()),
        );
        let err = bus
            .dispatch(
                CounterCommand {
                    id: "c1".into(),
                    increment: 5,
                },
                ctx(),
            )
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            CommandBusError::AppendFailed {
                source: EventStoreError::ConcurrencyConflict { .. },
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_dispatch_stamps_audit_on_every_event() {
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        let ctx = CommandContext {
            actor_id: "alice-uuid".into(),
            session_id: Some("sess-1".into()),
            source_ip: Some("10.0.0.1".into()),
            user_agent: Some("test-agent".into()),
            correlation_id: Uuid::new_v4(),
            causation_id: None,
        };
        let corr = ctx.correlation_id;
        let events = bus
            .dispatch(
                CounterCommand {
                    id: "c1".into(),
                    increment: 5,
                },
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(events[0].audit.actor_id, "alice-uuid");
        assert_eq!(events[0].audit.actor_session_id.as_deref(), Some("sess-1"));
        assert_eq!(events[0].audit.source_ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(events[0].audit.user_agent.as_deref(), Some("test-agent"));
        assert_eq!(events[0].audit.correlation_id, corr);
        assert!(events[0].audit.timestamp_utc_us > 0);
    }

    #[tokio::test]
    async fn test_dispatch_rejects_invalid_actor() {
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        let bad_ctx = CommandContext {
            actor_id: "".into(), // empty
            session_id: None,
            source_ip: None,
            user_agent: None,
            correlation_id: Uuid::new_v4(),
            causation_id: None,
        };
        let err = bus
            .dispatch(
                CounterCommand {
                    id: "c1".into(),
                    increment: 5,
                },
                bad_ctx,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CommandBusError::InvalidAudit { .. }));
    }

    #[tokio::test]
    async fn test_concurrent_dispatches_keep_distinct_correlation_ids() {
        let bus = Arc::new(CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        ));

        let ctx_a = CommandContext::for_actor("alice");
        let ctx_b = CommandContext::for_actor("bob");
        let corr_a = ctx_a.correlation_id;
        let corr_b = ctx_b.correlation_id;
        assert_ne!(corr_a, corr_b);

        let bus_a = bus.clone();
        let bus_b = bus.clone();
        let h_a = tokio::spawn(async move {
            bus_a
                .dispatch(
                    CounterCommand {
                        id: "agg-a".into(),
                        increment: 1,
                    },
                    ctx_a,
                )
                .await
        });
        let h_b = tokio::spawn(async move {
            bus_b
                .dispatch(
                    CounterCommand {
                        id: "agg-b".into(),
                        increment: 1,
                    },
                    ctx_b,
                )
                .await
        });
        let res_a = h_a.await.unwrap().unwrap();
        let res_b = h_b.await.unwrap().unwrap();

        assert_eq!(res_a[0].audit.correlation_id, corr_a);
        assert_eq!(res_a[0].audit.actor_id, "alice");
        assert_eq!(res_b[0].audit.correlation_id, corr_b);
        assert_eq!(res_b[0].audit.actor_id, "bob");
    }

    #[tokio::test]
    async fn test_caused_by_inherits_correlation() {
        let bus = CommandBus::<CounterAggregate>::new(
            Box::new(InMemoryEventStore::new()),
            Box::new(InProcessEventBus::new()),
        );
        let first_ctx = CommandContext::for_actor("alice");
        let trigger_corr = first_ctx.correlation_id;
        let triggers = bus
            .dispatch(
                CounterCommand {
                    id: "c1".into(),
                    increment: 5,
                },
                first_ctx,
            )
            .await
            .unwrap();

        let follow_ctx = CommandContext::caused_by("projection-worker", &triggers[0]);
        let follow = bus
            .dispatch(
                CounterCommand {
                    id: "c2".into(),
                    increment: 1,
                },
                follow_ctx,
            )
            .await
            .unwrap();

        assert_eq!(follow[0].audit.correlation_id, trigger_corr);
        assert_eq!(follow[0].audit.causation_id, Some(triggers[0].event_id));
    }

    #[test]
    fn test_error_messages() {
        let e = CommandBusError::handle_failed("user-123", "Invalid email");
        assert!(e.to_string().contains("user-123"));
        assert!(e.to_string().contains("Invalid email"));
        assert!(CommandBusError::other("X").to_string().contains("X"));
    }
}
