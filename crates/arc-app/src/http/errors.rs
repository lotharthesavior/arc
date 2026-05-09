use actix_web::{HttpResponse, ResponseError};
use arc_core::command_bus::CommandBusError;
use arc_core::event_store::EventStoreError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Command failed: {0}")]
    CommandFailed(#[from] CommandBusError),
}

// AppError is already handled by derive(thiserror::Error) for Display
impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::CommandFailed(err) => match err {
                CommandBusError::HandleFailed { message, .. } => {
                    HttpResponse::UnprocessableEntity().json(serde_json::json!({
                        "error": "ValidationFailed",
                        "message": message
                    }))
                }
                CommandBusError::AppendFailed { source, .. } => match source {
                    EventStoreError::ConcurrencyConflict { .. } => {
                        HttpResponse::Conflict().json(serde_json::json!({
                            "error": "ConcurrencyConflict",
                            "message": "Resource was modified by another request. Please try again."
                        }))
                    }
                    _ => HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "InternalServerError",
                        "message": "Storage error."
                    })),
                },
                CommandBusError::LoadFailed { .. } => {
                    HttpResponse::NotFound().json(serde_json::json!({
                        "error": "NotFound",
                        "message": "Resource not found."
                    }))
                }
                CommandBusError::InvalidAudit { .. } => {
                    HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "InvalidAudit",
                        "message": "Audit metadata for this request is missing or malformed."
                    }))
                }
                _ => HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "InternalServerError",
                    "message": "An unexpected error occurred."
                })),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use arc_core::audit::AuditError;

    #[test]
    fn test_invalid_audit_maps_to_bad_request() {
        let err = AppError::CommandFailed(CommandBusError::InvalidAudit {
            aggregate_id: "u-1".to_string(),
            source: AuditError::EmptyActorId,
        });
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_handle_failed_maps_to_unprocessable_entity() {
        let err = AppError::CommandFailed(CommandBusError::handle_failed("u-1", "bad email"));
        assert_eq!(
            err.error_response().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn test_concurrency_conflict_maps_to_conflict() {
        let err = AppError::CommandFailed(CommandBusError::AppendFailed {
            aggregate_id: "u-1".to_string(),
            source: EventStoreError::ConcurrencyConflict {
                aggregate_id: "u-1".to_string(),
                expected: 1,
                actual: 2,
            },
        });
        assert_eq!(err.error_response().status(), StatusCode::CONFLICT);
    }
}
