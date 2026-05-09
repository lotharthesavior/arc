# Access Logging (HIPAA-2)

Where [`AuditMetadata`](audit-metadata.md) audits *writes*, `AccessLogger`
audits *reads*. Both are required by HIPAA §164.312(b) and equivalent
clauses in GDPR, PCI-DSS, and SOC 2. Reads do not go through the event
store — they need their own hook.

The trait is intentionally generic. PHI is one of several `Sensitivity`
tags; PCI, PII, Confidential, Internal, and Public share the same
mechanism, just with different routing rules in the sink.

---

## 1. Read-path lifecycle

![Sequence Diagram - Access log read path - JWT middleware resolves actor, controller loads aggregate via EventStore, calls AccessLogger before returning sensitive fields, sink persists entry while non-found responses log nothing](../diagrams/flow-17-access-log-read-path.svg)

Step by step:

1. JWT middleware resolves `actor_id` (aggregate UUID) from the bearer token.
2. The controller loads aggregate state through `command_bus.event_store()`.
3. **Before** returning the response body, the controller calls
   `access_log::record_read(...)` with the resource description, purpose,
   and the actor identity built from the request.
4. The configured `AccessLogger` validates inputs, builds an
   `AccessLogEntry` (UUID, timestamp, correlation), and forwards to its
   sink.
5. Sink failures are warned but **not bubbled up** — audit availability
   must not gate user-facing reads.
6. If the resource is missing or deleted, the controller returns 404
   *without* logging — there's nothing the user observed.

---

## 2. Architecture

![Architecture Diagram - AccessLogger architecture - Read controllers funnel through helpers::access_log into the AccessLogger trait, which has multiple implementations including no-op, recording, JetStream, and DB-backed sinks; entries carry a Sensitivity tag spanning PHI, PCI, PII, Confidential, Internal, and Public](../diagrams/architecture-22-access-log-architecture.svg)

The trait sits between read controllers and a pluggable sink. Default
deployments use `NoOpAccessLogger` (validates and discards). Tests use
`RecordingAccessLogger` (in-memory) to assert what would have been logged.
Step 3 adds a JetStream sink for distributed audit pipelines.

---

## 3. Data model

```rust
trait AccessLogger {
    async fn log_access(
        &self,
        actor:          Identity,
        resource:       AccessedResource,
        purpose:        PurposeOfUse,
        correlation_id: Option<Uuid>,
    ) -> Result<(), AccessLogError>;
}

struct Identity {
    actor_id:    String,           // aggregate UUID, "system", "anonymous", "legacy-pre-hipaa"
    session_id:  Option<String>,
    source_ip:   Option<String>,
    user_agent:  Option<String>,
}

struct AccessedResource {
    kind:        String,           // "UserProfile", "Patient", "Order"
    identifier:  String,           // the id of the row(s)
    fields:      Vec<String>,      // "name", "email", "vitals", ...
    sensitivity: Sensitivity,      // HIPAA/PCI/PII/Conf/Internal/Public
}

enum Sensitivity { Phi, Pci, Pii, Confidential, Internal, Public }

enum PurposeOfUse {
    Treatment, Payment, Operations, Emergency,
    UserInitiated, AuditReview, Other,
}
```

`Sensitivity::is_regulated()` returns `false` only for `Public`. Sinks may
use this to drop public-data reads when volume is a concern.

---

## 4. Wiring a read controller

```rust
use crate::helpers::access_log;
use nineties_core::access_log::{
    AccessLogger, AccessedResource, PurposeOfUse, Sensitivity,
};

#[get("/profile")]
pub async fn profile(
    req: HttpRequest,
    command_bus: web::Data<CommandBus<UserAggregate>>,
    access_logger: web::Data<dyn AccessLogger>,
) -> impl Responder {
    let agg_id = match req.extensions().get::<String>() {
        Some(id) => id.clone(),
        None => return HttpResponse::Unauthorized().finish(),
    };

    // Load the aggregate first; do NOT log if the resource doesn't exist.
    let events = command_bus.event_store().load(&agg_id).await?;
    if events.is_empty() {
        return HttpResponse::NotFound().finish();
    }
    let aggregate = UserAggregate::from_events(events);

    let resource = AccessedResource::new("UserProfile", agg_id.clone(), Sensitivity::Pii)
        .with_fields(["id", "name", "email"]);
    access_log::record_read(
        access_logger.as_ref(),
        &req,
        agg_id,
        resource,
        PurposeOfUse::UserInitiated,
    ).await;

    HttpResponse::Ok().json(aggregate.into_response())
}
```

Things to **not** do:

- **Don't log before resolving the resource.** A 404 should NOT log a
  successful read — the actor did not see anything.
- **Don't make the response wait on a slow sink.** `record_read` is
  `async`, but failures are warned, not propagated. If your sink ever
  introduces a real backpressure path, document it explicitly and provide
  a fallback.
- **Don't pick `Sensitivity::Public` to "make it pass."** Public means
  intentionally public — login pages, marketing copy, terms-of-service
  endpoints. Anything that requires authentication is not public.

---

## 5. Wiring the logger in `serve.rs`

Default to `NoOpAccessLogger`:

```rust
use nineties_core::access_log::{AccessLogger, NoOpAccessLogger};
use std::sync::Arc;

let access_logger: Arc<dyn AccessLogger> = Arc::new(NoOpAccessLogger);
let access_logger_data = web::Data::from(access_logger);

App::new()
    .app_data(access_logger_data.clone())
    // ...
```

Production PHI/PCI deployments swap the constructor for the real sink
(JetStream-backed, DB-backed, etc.) when those land. The controller code
does not change.

---

## 6. Testing

`nineties-core` exposes `RecordingAccessLogger` behind the `test-utils`
feature for downstream tests. It captures every `log_access` call and
returns them via `entries().await`.

```rust
use nineties_core::access_log::{AccessLogger, RecordingAccessLogger};

let rec = RecordingAccessLogger::new();
let arc: Arc<dyn AccessLogger> = Arc::new(rec.clone());
let logger_data = web::Data::from(arc);

// Run the test app...
let entries = rec.entries().await;
assert_eq!(entries[0].resource.sensitivity, Sensitivity::Pii);
assert_eq!(entries[0].actor.actor_id, agg_id);
assert_eq!(entries[0].purpose, PurposeOfUse::UserInitiated);
```

Two HTTP-level tests already cover the wiring:

- `test_profile_get_invokes_access_logger` — verifies a successful GET
  produces exactly one entry, with the correct actor, fields,
  sensitivity, and purpose
- `test_profile_404_does_not_log_access` — verifies a 404 produces zero
  entries

---

## 7. What's logged vs. what's audited

| Operation | Mechanism | Example |
|---|---|---|
| State change | `EventStore` + `AuditMetadata` | `UserRegistered`, `ProfileUpdated`, `UserDeleted` |
| Read of sensitive data | `AccessLogger` | `GET /profile`, `GET /patients/{id}/vitals`, admin dashboard data fetches |
| Read of public data | None required | login page, marketing pages |
| Failed authn / 404 | None required | the actor did not see anything |
| Failed authz (403) | **Should** log — TODO | actor *attempted* to read; sink should know |

The 403 case will be addressed when the framework grows a uniform authz
layer (out of HIPAA-2 scope).

---

## 8. Limitations and open questions

- **No durable sink yet.** Step 3's JetStream sink is the first
  production-ready implementation. `NoOpAccessLogger` is fine for
  development and any non-regulated app.
- **Per-request batching.** A controller that returns many resources
  should call `log_access` once per resource — not once per response. Sink
  implementations should batch on the inbound side, not in the
  controller.
- **PHI redaction in trace logs.** `User-Agent` and `source_ip` may be PII
  in some jurisdictions. The `tracing::warn!` fallback when sinks fail
  prints those fields. Production logging configs should redact
  accordingly. A `redacted_for_logging()` helper is on the roadmap.

---

## See also

- `docs/guides/audit-metadata.md` — write-side audit (HIPAA-1)
- `crates/nineties-core/src/access_log.rs` — source of truth for the
  trait and types
- `docs/ark/refactor-plan.md` HIPAA appendix — broader compliance context
