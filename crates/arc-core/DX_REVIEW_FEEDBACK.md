# DX Review: Feedback for Event Sourcing Implementation

> **From**: Agent 7 (DX Specialist)
> **To**: Implementation Team
> **Date**: 2026-03-01
> **Status**: Ready for Implementation

---

## Executive Summary

The planned event sourcing API design is **solid and well-architected**. The following recommendations will make it more developer-friendly and prevent common mistakes.

---

## Key Recommendations

### 1. Naming Consistency (CRITICAL)

**Issue**: Mixed terminology can confuse developers learning event sourcing.

**Recommendation**: Enforce strict naming conventions:
- Commands → imperative verbs (`CreateUser`, not `UserCreate`)
- Events → past tense (`UserCreated`, not `CreateUser`)
- Aggregates → domain nouns with `Aggregate` suffix (`UserAggregate`)
- Projections → purpose with `Projection` suffix (`UserListProjection`)

**Action**: Add clippy lints to enforce naming patterns.

---

### 2. Error Messages (HIGH PRIORITY)

**Issue**: Current error design (from implementation guide) doesn't include context.

**Current (from docs)**:
```rust
pub enum UserError {
    #[error("User already exists")]
    AlreadyExists,
}
```

**Recommended**:
```rust
pub enum UserError {
    #[error("User with email '{email}' already exists")]
    AlreadyExists { email: String },
}
```

**Why**: Developers need context to debug. Generic errors waste time.

**Action**: All domain errors should include:
1. What went wrong
2. Which entity/aggregate
3. How to fix it (when possible)

---

### 3. Progressive Disclosure (HIGH PRIORITY)

**Issue**: Event sourcing has a steep learning curve. Developers need an on-ramp.

**Recommendation**: Support TWO complexity paths:

#### Path 1: Simple (No Aggregates)
```rust
// Services emit events directly
let event = Event::new("User", id, 1, "UserCreated", json!({...}));
event_store.append(id, None, vec![event]).await?;
```

#### Path 2: Complex (Full CQRS)
```rust
// Full aggregate with business logic
let events = command_bus.dispatch(CreateUserCommand { ... }).await?;
```

**Action**:
- Document both paths clearly
- Show when to use each
- Provide migration guide from simple → complex

---

### 4. Concurrency Control (CRITICAL)

**Issue**: Forgetting version checks causes data corruption in production.

**Current API** (allows skipping version):
```rust
async fn append(
    &self,
    aggregate_id: &str,
    expected_version: Option<i64>,  // ← Can pass None!
    events: Vec<Event>,
) -> Result<()>;
```

**Recommendation**: Make it harder to skip version checks:

```rust
// Force explicit choice
pub enum VersionCheck {
    New,                    // First event for aggregate
    Expected(i64),          // Requires version
    Auto,                   // Load current version automatically
}

async fn append(
    &self,
    aggregate_id: &str,
    version_check: VersionCheck,
    events: Vec<Event>,
) -> Result<()>;
```

**Action**: Update EventStore trait to use VersionCheck enum.

---

### 5. Event Versioning (MEDIUM PRIORITY)

**Issue**: Event schema changes will break replay unless handled explicitly.

**Recommendation**: Build versioning into Event type:

```rust
pub struct Event {
    pub event_id: Uuid,
    pub event_type: String,
    pub event_version: u32,  // ← Add this
    pub payload: serde_json::Value,
    // ...
}
```

**Action**:
- Add `event_version` field to Event
- Provide `EventMigration` trait for upcasting
- Document versioning strategy in architecture docs

---

### 6. Testing Support (HIGH PRIORITY)

**Issue**: Developers will struggle to test aggregates without test utilities.

**Recommendation**: Include test helpers in arc-core:

```rust
// arc-core/src/testing.rs
pub mod testing {
    pub struct InMemoryEventStore { ... }
    pub struct AggregateFixture<A: Aggregate> { ... }
}
```

**Usage**:
```rust
#[tokio::test]
async fn test_user_creation() {
    AggregateFixture::<UserAggregate>::new()
        .given(/* prior events */)
        .when(CreateUserCommand { ... })
        .await
        .then_expect_events(vec![/* expected events */]);
}
```

**Action**: Create `testing` module as part of Phase 1.

---

### 7. IDE Support (MEDIUM PRIORITY)

**Issue**: Complex generic types hurt autocomplete and type inference.

**Current** (from implementation guide):
```rust
pub struct CommandBus<A: Aggregate> {
    event_store: Box<dyn EventStore>,
    // ...
}
```

**Recommendation**: Prefer associated types over generics:

```rust
pub trait Aggregate {
    type Command: Command;  // ← Associated type
    type Event;
    type Error: Error;
}
```

**Why**: IDEs can infer associated types better than generic parameters.

**Action**: Review all traits - prefer associated types where possible.

---

### 8. Documentation Standards (MEDIUM PRIORITY)

**Issue**: Examples that don't compile waste developer time.

**Recommendation**: All doc comments must have working examples:

```rust
/// Append events to the event store.
///
/// # Example
///
/// ```rust
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use arc_core::{Event, EventStore, SqliteEventStore};
///
/// let event_store = SqliteEventStore::new("events.db").await?;
/// let event = Event::new("User", "user-123", 1, "UserCreated", json!({}));
/// event_store.append("user-123", None, vec![event]).await?;
/// # Ok(())
/// # }
/// ```
pub async fn append(&self, ...) -> Result<()>;
```

**Action**:
- Add `cargo test --doc` to CI
- Require examples in all public APIs

---

### 9. Common Pitfalls Documentation (LOW PRIORITY)

**Issue**: Event sourcing has non-obvious gotchas (projection performance, aggregate boundaries, etc.)

**Recommendation**: Create "Pitfalls and Best Practices" section in docs:

Topics to cover:
- Aggregate size limits (< 100 events)
- Projection rebuild performance
- Event payload design (values vs references)
- Concurrency violation handling
- Event schema evolution

**Action**: Add to DX_GUIDELINES.md (already done).

---

### 10. Type Safety Improvements (LOW PRIORITY)

**Issue**: Current Event type allows invalid states (sequence without aggregate_id).

**Current**:
```rust
pub struct Event {
    pub event_id: Uuid,
    pub aggregate_id: String,
    pub sequence: i64,
    // All fields public - can be modified independently
}
```

**Recommendation**: Make Event construction safer:

```rust
pub struct Event {
    event_id: Uuid,           // Private
    aggregate_id: String,     // Private
    sequence: i64,            // Private
    // Only accessible via constructor or getters
}

impl Event {
    pub fn new(
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        sequence: i64,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        // Constructor enforces invariants
        Self {
            event_id: Uuid::new_v4(),
            aggregate_type: aggregate_type.into(),
            aggregate_id: aggregate_id.into(),
            sequence,
            event_type: event_type.into(),
            payload,
            metadata: json!({}),
            timestamp: SystemTime::now(),
        }
    }
}
```

**Action**: Make Event fields private, add constructor and getters.

---

## Implementation Priority

| Priority | Recommendation | Effort | Impact |
|----------|---------------|--------|--------|
| P0 | Naming Consistency | Low | High |
| P0 | Concurrency Control | Medium | Critical |
| P0 | Error Messages | Low | High |
| P1 | Testing Support | Medium | High |
| P1 | Progressive Disclosure (docs) | Low | High |
| P2 | Event Versioning | Medium | Medium |
| P2 | IDE Support | Low | Medium |
| P2 | Documentation Standards | Low | Medium |
| P3 | Common Pitfalls (docs) | Low | Low |
| P3 | Type Safety | Low | Low |

---

## Quick Wins (Can Be Done Now)

1. **Add naming lint rules** to Cargo.toml:
   ```toml
   [lints.clippy]
   enum_variant_names = "warn"
   ```

2. **Update UserError** in implementation guide examples with context fields

3. **Create testing module stub** in Phase 1 workspace setup

4. **Add event_version field** to Event type design

5. **Document simple vs complex paths** in architecture doc

---

## Questions for Architecture Team

1. Should we support automatic event upcasting or require explicit migration code?
2. What's the policy on breaking changes to event schemas? (Recommend: never break, always add)
3. Should CommandBus automatically retry on concurrency violations or fail immediately?
4. Do we want snapshot support in v1 or defer to v2?

---

## Developer Onboarding Checklist

When a new developer joins, they should:

1. Read `/docs/09-event-sourcing-architecture.md` (concepts)
2. Read `/docs/10-event-sourcing-implementation-guide.md` (implementation)
3. Read `/crates/arc-core/DX_GUIDELINES.md` (patterns and pitfalls)
4. Run example: "Simple event store" (no aggregates)
5. Run example: "Full CQRS" (with aggregates)
6. Build a sample aggregate (Blog post or similar)
7. Write tests using AggregateFixture

---

## Conclusion

The API design is **architecturally sound**. These DX improvements will:
- Reduce learning curve (progressive disclosure)
- Prevent common mistakes (concurrency, versioning)
- Improve debugging experience (rich errors)
- Accelerate development (test utilities, IDE support)

All recommendations are captured in detail in `DX_GUIDELINES.md`.

**Recommendation**: Implement P0 items before Phase 1 begins. P1-P2 items during Phase 1-2. P3 items as time permits.

---

**Ready for Team Review** ✓
