//! Diagnostic endpoints, mounted only when `APP_ENV=e2e`.
//!
//! These routes expose enough of the event log for browser-driven tests to
//! verify HIPAA audit metadata landed correctly, without coupling the test
//! runner to any specific event-store backend (the same handler works against
//! SQLite today and Postgres later — both go through the `EventStore` trait).
//!
//! Mounted from `routes::config` only when `APP_ENV=e2e`. Production builds
//! never expose these endpoints.

use crate::domain::user::aggregate::UserAggregate;
use actix_web::{get, web, HttpResponse, Responder};
use nineties_core::command_bus::CommandBus;
use serde_json::json;

/// `GET /__diag__/events/{aggregate_id}`
///
/// Returns every event for the given aggregate, including the full
/// `AuditMetadata`. Payload is included verbatim — tests should be aware they
/// see the same data the application stored.
#[get("/events/{aggregate_id}")]
pub async fn list_events(
    path: web::Path<String>,
    command_bus: web::Data<CommandBus<UserAggregate>>,
) -> impl Responder {
    let aggregate_id = path.into_inner();
    match command_bus.event_store().load(&aggregate_id).await {
        Ok(events) => {
            let body: Vec<_> = events
                .into_iter()
                .map(|e| {
                    json!({
                        "event_id":       e.event_id,
                        "aggregate_type": e.aggregate_type,
                        "aggregate_id":   e.aggregate_id,
                        "sequence":       e.sequence,
                        "event_type":     e.event_type,
                        "payload":        e.payload,
                        "audit": {
                            "actor_id":         e.audit.actor_id,
                            "actor_session_id": e.audit.actor_session_id,
                            "source_ip":        e.audit.source_ip,
                            "user_agent":       e.audit.user_agent,
                            "timestamp_utc_us": e.audit.timestamp_utc_us,
                            "causation_id":     e.audit.causation_id,
                            "correlation_id":   e.audit.correlation_id,
                        },
                    })
                })
                .collect();
            HttpResponse::Ok().json(json!({ "events": body }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": "EventStoreError",
            "message": e.to_string(),
        })),
    }
}

/// `GET /__diag__/health` — always available when diag is mounted; cheap
/// readiness probe for the global setup script.
#[get("/health")]
pub async fn diag_health() -> impl Responder {
    HttpResponse::Ok().json(json!({ "status": "diag-ok" }))
}
