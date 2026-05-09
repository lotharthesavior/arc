//! Build a `CommandContext` from an Actix HTTP request.
//!
//! Pulls `source_ip`, `user_agent`, and an optional `X-Correlation-Id` header
//! into the context so every event written by the request carries
//! request-scoped audit metadata (HIPAA §164.312(b)).

use actix_web::HttpRequest;
use nineties_core::audit::ANONYMOUS_ACTOR;
use nineties_core::command_bus::CommandContext;
use uuid::Uuid;

/// Build a `CommandContext` for an authenticated request.
/// Pass the aggregate UUID resolved by the JWT middleware as `actor_id`.
pub fn for_actor(req: &HttpRequest, actor_id: impl Into<String>) -> CommandContext {
    CommandContext {
        actor_id: actor_id.into(),
        session_id: None,
        source_ip: req
            .connection_info()
            .realip_remote_addr()
            .map(str::to_string),
        user_agent: req
            .headers()
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string),
        correlation_id: correlation_from(req),
        causation_id: None,
    }
}

/// Build a `CommandContext` for an unauthenticated request (e.g. self-registration).
pub fn anonymous(req: &HttpRequest) -> CommandContext {
    for_actor(req, ANONYMOUS_ACTOR)
}

/// Read `X-Correlation-Id` from the incoming request, falling back to a fresh UUID.
fn correlation_from(req: &HttpRequest) -> Uuid {
    req.headers()
        .get("X-Correlation-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::new_v4)
}
