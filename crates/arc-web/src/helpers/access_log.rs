//! Build [`Identity`](arc_core::access_log::Identity) values from Actix
//! requests and run an `AccessLogger` call with the appropriate failure
//! policy for the resource's [`Sensitivity`].
//!
//! - PHI / PCI reads → [`FailurePolicy::FailHard`] by default. A logger sink
//!   failure returns an error with HTTP 503 before releasing the response.
//! - Everything else → [`FailurePolicy::FailOpenWarn`]. Failure is warned and
//!   the read proceeds.

use crate::http::errors::AppError;
use actix_web::{http::StatusCode, HttpRequest, HttpResponse, Responder};
use arc_core::access_log::{
    AccessLogError, AccessLogger, AccessedResource, FailurePolicy, Identity, PurposeOfUse,
    Sensitivity,
};
use serde::Serialize;
use uuid::Uuid;

/// A wrapper for sensitive data that has not yet been audit-logged.
///
/// `Sensitive<T>` does not implement `Serialize` or `Responder`, preventing it
/// from being accidentally returned by a controller before an [`AccessLogger`]
/// call. It can only be "cleansed" into an [`AccessLogged<T>`] through the
/// appropriate audit helper. This discipline applies only after data is wrapped.
///
/// ```compile_fail
/// use arc_web::helpers::access_log::Sensitive;
/// let _ = Sensitive::pii("secret").into_parts();
/// ```
///
/// ```compile_fail
/// use arc_web::helpers::access_log::Sensitive;
/// let _ = serde_json::to_string(&Sensitive::pii("secret"));
/// ```
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
    fn into_parts(self) -> (T, Sensitivity) {
        (self.data, self.sensitivity)
    }
}

/// A wrapper for data that has been successfully audit-logged (or at least
/// passed the failure policy check).
///
/// `AccessLogged<T>` implements `Serialize` and `Responder` (by delegating to `T`),
/// making it the standard return type for audited read controllers.
///
/// ```compile_fail
/// use arc_web::helpers::access_log::AccessLogged;
/// let _ = AccessLogged::new("unaudited");
/// ```
///
/// ```compile_fail
/// use arc_web::helpers::access_log::AccessLogged;
/// let _: AccessLogged<String> = serde_json::from_str(r#"{"data":"unaudited"}"#).unwrap();
/// ```
#[derive(Serialize, Clone, PartialEq)]
pub struct AccessLogged<T> {
    data: T,
}

impl<T> AccessLogged<T> {
    /// Wrap data that has been audited.
    fn new(data: T) -> Self {
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
                    error_kind = error_kind(&e),
                    "access log sink failed on regulated read — failing closed"
                );
                Err(AppError::AuditFailed {
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    message: "Audit sink unavailable".into(),
                })
            }
            FailurePolicy::FailOpenWarn => {
                tracing::warn!(error_kind = error_kind(&e), "access log sink rejected read");
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

// Sink errors are arbitrary strings and may include queries, credentials or payloads.
fn error_kind(error: &AccessLogError) -> &'static str {
    match error {
        AccessLogError::Validation(_) => "validation",
        AccessLogError::Sink(_) => "sink",
    }
}

impl<T> std::fmt::Debug for Sensitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sensitive")
            .field("sensitivity", &self.sensitivity)
            .finish_non_exhaustive()
    }
}

impl<T> std::fmt::Debug for AccessLogged<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessLogged").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{body::to_bytes, test::TestRequest};
    use arc_core::access_log::RecordingAccessLogger;

    struct FailingLogger;

    #[async_trait::async_trait]
    impl AccessLogger for FailingLogger {
        async fn log_access(
            &self,
            _: Identity,
            _: AccessedResource,
            _: PurposeOfUse,
            _: Option<Uuid>,
        ) -> Result<(), AccessLogError> {
            Err(AccessLogError::Sink("private sink details".into()))
        }
    }

    #[actix_web::test]
    async fn failure_policy_uses_wrapped_classification() {
        for sensitivity in [
            Sensitivity::Phi,
            Sensitivity::Pci,
            Sensitivity::Pii,
            Sensitivity::Confidential,
            Sensitivity::Internal,
            Sensitivity::Public,
        ] {
            let req = TestRequest::default().to_http_request();
            let result = record_read(
                &FailingLogger,
                &req,
                "actor",
                AccessedResource::new("Profile", "id", Sensitivity::Public),
                PurposeOfUse::UserInitiated,
                Sensitive::new("secret", sensitivity),
            )
            .await;
            if matches!(sensitivity, Sensitivity::Phi | Sensitivity::Pci) {
                let err = result.unwrap_err();
                assert_eq!(
                    actix_web::ResponseError::error_response(&err).status(),
                    StatusCode::SERVICE_UNAVAILABLE
                );
                assert!(!err.to_string().contains("private sink details"));
            } else {
                assert_eq!(result.unwrap().into_inner(), "secret");
            }
        }
    }

    #[actix_web::test]
    async fn records_metadata_before_preserving_response_shape() {
        let logger = RecordingAccessLogger::new();
        let correlation = Uuid::new_v4();
        let req = TestRequest::default()
            .insert_header(("X-Correlation-Id", correlation.to_string()))
            .to_http_request();
        let response = record_read(
            &logger,
            &req,
            "actor",
            AccessedResource::new("Profile", "id", Sensitivity::Public).with_fields(["email"]),
            PurposeOfUse::UserInitiated,
            Sensitive::pii(serde_json::json!({"email":"secret"})),
        )
        .await
        .unwrap();
        let entries = logger.entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].resource.sensitivity, Sensitivity::Pii);
        assert_eq!(entries[0].resource.fields, ["email"]);
        assert_eq!(entries[0].correlation_id, Some(correlation));
        assert!(!serde_json::to_string(&entries).unwrap().contains("secret"));
        let body = to_bytes(response.respond_to(&req).into_body())
            .await
            .unwrap();
        assert_eq!(body, r#"{"email":"secret"}"#);
    }

    #[test]
    fn diagnostics_exclude_payloads_and_sink_details() {
        assert!(!format!("{:?}", Sensitive::pii("secret")).contains("secret"));
        assert!(!format!("{:?}", AccessLogged::new("secret")).contains("secret"));
        assert_eq!(error_kind(&AccessLogError::Sink("secret".into())), "sink");
        assert_eq!(
            error_kind(&AccessLogError::Validation("secret".into())),
            "validation"
        );
    }
}
