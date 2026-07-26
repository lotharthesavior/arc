use actix_session::Session;
use rand::Rng;

const CSRF_TOKEN_KEY: &str = "csrf_token";
const CSRF_TOKEN_LENGTH: usize = 32;

/// Generates a new CSRF token and stores it in the session.
/// Returns the generated token.
pub fn generate_csrf_token(session: &Session) -> String {
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(CSRF_TOKEN_LENGTH)
        .map(char::from)
        .collect();

    session.insert(CSRF_TOKEN_KEY, &token).unwrap();
    token
}

/// Retrieves the current CSRF token from the session.
/// If no token exists, generates a new one.
pub fn get_csrf_token(session: &Session) -> String {
    match session.get::<String>(CSRF_TOKEN_KEY) {
        Ok(Some(token)) => token,
        _ => generate_csrf_token(session),
    }
}

/// Validates a CSRF token against the one stored in the session.
/// Returns true if the token is valid, false otherwise.
pub fn validate_csrf_token(session: &Session, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }

    match session.get::<String>(CSRF_TOKEN_KEY) {
        Ok(Some(session_token)) => {
            // Use constant-time comparison to prevent timing attacks
            constant_time_compare(&session_token, token)
        }
        _ => false,
    }
}

/// Validates the CSRF token and regenerates it after validation.
/// This provides additional security by ensuring each token is single-use.
pub fn validate_and_regenerate_csrf_token(session: &Session, token: &str) -> bool {
    let is_valid = validate_csrf_token(session, token);
    if is_valid {
        // Regenerate token after successful validation (single-use tokens)
        generate_csrf_token(session);
    }
    is_valid
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc123", "abc123"));
        assert!(!constant_time_compare("abc123", "abc124"));
        assert!(!constant_time_compare("abc123", "abc12"));
        assert!(!constant_time_compare("", "abc"));
        assert!(constant_time_compare("", ""));
    }
}
