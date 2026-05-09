use serde::Deserialize;
use validator::Validate;

/// Login form validation
#[derive(Debug, Deserialize, Validate)]
pub struct LoginForm {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// User profile update form validation
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileForm {
    #[validate(length(
        min = 2,
        max = 100,
        message = "Name must be between 2 and 100 characters"
    ))]
    pub name: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_valid_login_form() {
        let form = LoginForm {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };
        assert!(form.validate().is_ok());
    }

    #[test]
    fn test_invalid_email() {
        let form = LoginForm {
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
        };
        assert!(form.validate().is_err());
    }
}
