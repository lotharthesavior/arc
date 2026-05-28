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

/// User registration form validation
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct RegisterForm {
    #[validate(length(
        min = 2,
        max = 100,
        message = "Name must be between 2 and 100 characters"
    ))]
    pub name: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    #[validate(custom(function = "validate_password_strength"))]
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

/// Password change form validation
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordForm {
    #[validate(length(min = 1, message = "Current password is required"))]
    pub current_password: String,

    #[validate(length(min = 8, message = "New password must be at least 8 characters long"))]
    #[validate(custom(function = "validate_password_strength"))]
    pub new_password: String,

    #[validate(must_match(other = "new_password", message = "Passwords do not match"))]
    pub confirm_password: String,
}

/// Custom validator for password strength
/// Requires at least one lowercase, one uppercase, and one digit
fn validate_password_strength(password: &str) -> Result<(), validator::ValidationError> {
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    if has_lowercase && has_uppercase && has_digit {
        Ok(())
    } else {
        Err(validator::ValidationError::new("password_strength"))
    }
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

    #[test]
    fn test_password_too_short() {
        let form = RegisterForm {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "Short1".to_string(),
        };
        assert!(form.validate().is_err());
    }

    #[test]
    fn test_password_strength_validation() {
        // Missing uppercase
        assert!(validate_password_strength("password123").is_err());

        // Missing lowercase
        assert!(validate_password_strength("PASSWORD123").is_err());

        // Missing digit
        assert!(validate_password_strength("PasswordABC").is_err());

        // Valid password
        assert!(validate_password_strength("Password123").is_ok());
    }

    #[test]
    fn test_valid_register_form() {
        let form = RegisterForm {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
        };
        assert!(form.validate().is_ok());
    }

    #[test]
    fn test_name_too_short() {
        let form = RegisterForm {
            name: "A".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123".to_string(),
        };
        assert!(form.validate().is_err());
    }
}
