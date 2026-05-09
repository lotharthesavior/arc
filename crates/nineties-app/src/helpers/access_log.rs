//! Build [`Identity`](nineties_core::access_log::Identity) values from Actix
//! requests and run an `AccessLogger` call with the appropriate failure
//! policy for the resource's [`Sensitivity`].
//!
//! - PHI / PCI reads → [`FailurePolicy::FailHard`] by default. A logger sink
//!   failure becomes [`RecordReadOutcome::FailHard`] and the controller
//!   should respond 503; an audit gap on regulated data is unacceptable.
//! - Everything else → [`FailurePolicy::FailOpenWarn`]. Failure is warned and
//!   the read proceeds.

use actix_web::HttpRequest;
use nineties_core::access_log::{
    AccessLogger, AccessedResource, FailurePolicy, Identity, PurposeOfUse,
};
use uuid::Uuid;

/// Build an `Identity` from a request's audit-relevant headers.
pub fn identity_from(req: &HttpRequest, actor_id: impl Into<String>) -> Identity {
    Identity {
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
    }
}

/// Read `X-Correlation-Id` from the incoming request, if present and parseable.
pub fn correlation_from(req: &HttpRequest) -> Option<Uuid> {
    req.headers()
        .get("X-Correlation-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Result of [`record_read`]. `Ok` ⇒ the controller proceeds with returning
/// data. `FailHard` ⇒ the controller MUST refuse the read with 503.
#[derive(Debug, PartialEq)]
pub enum RecordReadOutcome {
    Ok,
    FailHard,
}

/// Log a read. Picks failure policy from the resource's [`Sensitivity`].
pub async fn record_read(
    logger: &dyn AccessLogger,
    req: &HttpRequest,
    actor_id: impl Into<String>,
    resource: AccessedResource,
    purpose: PurposeOfUse,
) -> RecordReadOutcome {
    let policy = FailurePolicy::for_sensitivity(resource.sensitivity);
    let identity = identity_from(req, actor_id);
    let correlation = correlation_from(req);

    match logger
        .log_access(identity, resource, purpose, correlation)
        .await
    {
        Ok(()) => RecordReadOutcome::Ok,
        Err(e) => match policy {
            FailurePolicy::FailHard => {
                tracing::error!(
                    error = %e,
                    "access log sink failed on regulated read — failing closed"
                );
                RecordReadOutcome::FailHard
            }
            FailurePolicy::FailOpenWarn => {
                tracing::warn!(error = %e, "access log sink rejected read");
                RecordReadOutcome::Ok
            }
        },
    }
}
