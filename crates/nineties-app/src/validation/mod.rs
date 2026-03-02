pub mod user_validation;

use actix_web::{HttpResponse, ResponseError};
use std::fmt;
use validator::ValidationErrors;

/// Custom validation error type
#[derive(Debug)]
pub struct ValidationError {
    pub errors: ValidationErrors,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation failed: {:?}", self.errors)
    }
}

impl ResponseError for ValidationError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Validation failed",
            "details": format!("{:?}", self.errors)
        }))
    }
}

impl From<ValidationErrors> for ValidationError {
    fn from(errors: ValidationErrors) -> Self {
        ValidationError { errors }
    }
}
