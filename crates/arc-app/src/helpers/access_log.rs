//! Build [`Identity`](arc_core::access_log::Identity) values from Actix
//! requests and run an `AccessLogger` call with the appropriate failure
//! policy for the resource's [`Sensitivity`].
//!
//! - PHI / PCI reads → [`FailurePolicy::FailHard`] by default. A logger sink
//!   failure becomes [`RecordReadOutcome::FailHard`] and the controller
//!   should respond 503; an audit gap on regulated data is unacceptable.
//! - Everything else → [`FailurePolicy::FailOpenWarn`]. Failure is warned and
//!   the read proceeds.

use crate::http::errors::AppError;
use actix_web::{http::StatusCode, HttpRequest, HttpResponse, Responder};
use arc_core::access_log::{
    AccessLogger, AccessedResource, FailurePolicy, Identity, PurposeOfUse, Sensitivity,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A wrapper for sensitive data that has not yet been audit-logged.
///
/// `Sensitive<T>` does not implement `Serialize` or `Responder`, preventing it
/// from being accidentally returned by a controller before an [`AccessLogger`]
/// call. It can only be "cleansed" into an [`AccessLogged<T>`] through the
/// appropriate audit helper.
#[derive(Debug)]
pub struct Sensitive<T> {
    data: T,
    sensitivity: Sensitivity,
}

impl<T> Sensitive<T> {
    pub fn new(data: T, sensitivity: Sensitivity) -> Self {
        Self { data, sensitivity }
    }

    #[allow(dead_code)]
    pub fn phi(data: T) -> Self {
        Self::new(data, Sensitivity::Phi)
    }

    #[allow(dead_code)]
    pub fn pci(data: T) -> Self {
        Self::new(data, Sensitivity::Pci)
    }

    pub fn pii(data: T) -> Self {
        Self::new(data, Sensitivity::Pii)
    }

    #[allow(dead_code)]
    pub fn confidential(data: T) -> Self {
        Self::new(data, Sensitivity::Confidential)
    }

    #[allow(dead_code)]
    pub fn internal(data: T) -> Self {
        Self::new(data, Sensitivity::Internal)
    }

    /// Access the sensitivity level without exposing the data.
    #[allow(dead_code)]
    pub fn sensitivity(&self) -> Sensitivity {
        self.sensitivity
    }

    /// Deconstruct the sensitive wrapper. This is intentionally internal to
    /// the framework's audit helpers.
    pub fn into_parts(self) -> (T, Sensitivity) {
        (self.data, self.sensitivity)
    }
}

/// A wrapper for data that has been successfully audit-logged (or at least
/// passed the failure policy check).
///
/// `AccessLogged<T>` implements `Serialize` and `Responder` (by delegating to `T`),
/// making it the standard return type for audited read controllers.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AccessLogged<T> {
    data: T,
}

impl<T> AccessLogged<T> {
    /// Wrap data that has been audited.
    pub fn new(data: T) -> Self {
        Self { data }
    }

    /// Return the underlying data.
    pub fn into_inner(self) -> T {
        self.data
    }
}

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

/// Log a read of [`Sensitive`] data. Picks failure policy from the resource's
/// sensitivity.
///
/// If successful, returns [`AccessLogged<T>`] which implements [`Responder`].
pub async fn record_read<T>(
    logger: &dyn AccessLogger,
    req: &HttpRequest,
    actor_id: impl Into<String>,
    mut resource: AccessedResource,
    purpose: PurposeOfUse,
    sensitive: Sensitive<T>,
) -> Result<AccessLogged<T>, AppError> {
    let (data, sensitivity) = sensitive.into_parts();
    resource.sensitivity = sensitivity;

    let policy = FailurePolicy::for_sensitivity(sensitivity);
    let identity = identity_from(req, actor_id);
    let correlation = correlation_from(req);

    match logger
        .log_access(identity, resource, purpose, correlation)
        .await
    {
        Ok(()) => Ok(AccessLogged::new(data)),
        Err(e) => match policy {
            FailurePolicy::FailHard => {
                tracing::error!(
                    error = %e,
                    "access log sink failed on regulated read — failing closed"
                );
                Err(AppError::AuditFailed {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    message: "Audit sink unavailable".into(),
                })
            }
            FailurePolicy::FailOpenWarn => {
                tracing::warn!(error = %e, "access log sink rejected read");
                Ok(AccessLogged::new(data))
            }
        },
    }
}

impl<T: Serialize> Responder for AccessLogged<T> {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(self.into_inner())
    }
}
