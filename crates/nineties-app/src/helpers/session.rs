use crate::helpers::database::get_connection;
use crate::models::user::User;
use crate::schema::users::dsl::users;
use actix_session::Session;
use diesel::{QueryDsl, RunQueryDsl};

/// Check if user is authenticated by looking for cached user data in session.
/// Falls back to database query only if user_data is missing but user_id exists.
pub fn is_authenticated(session: &Session) -> bool {
    // First check if we have cached user data
    if let Ok(Some(_user)) = session.get::<User>("user_data") {
        return true;
    }

    // Fallback: check user_id and query DB (for backwards compatibility)
    let user_id: i32 = session
        .get::<i32>("user_id")
        .unwrap_or(Some(0))
        .unwrap_or(0);
    if user_id == 0 {
        return false;
    }

    let user: Result<User, diesel::result::Error> =
        users.find(user_id).first::<User>(&mut get_connection());
    if let Ok(user) = user {
        // Cache the user data for future requests
        let _ = session.insert("user_data", user);
        true
    } else {
        false
    }
}

/// Retrieves and clears the flash message from the session.
/// Returns a tuple of (message_type, message_text) where type is "success" or "error".
/// Set `is_json` to true for structured JSON messages (used by the sign-in page).
pub fn get_session_message(session: &Session, is_json: bool) -> (String, String) {
    if !is_json {
        let simple_message = (
            "success".to_string(),
            session
                .get::<String>("message")
                .unwrap_or(Some("".to_string()))
                .unwrap_or("".to_string()),
        );

        session.remove("message");

        return simple_message;
    }

    let session_message = session
        .get::<serde_json::Value>("message")
        .unwrap_or(Some(serde_json::json!({})))
        .unwrap_or(serde_json::json!({}));

    if session_message.is_null() {
        session.remove("message");

        return ("success".to_string(), "".to_string());
    }

    if session_message["error"].is_string()
        && !session_message["error"].as_str().unwrap().is_empty()
    {
        session.remove("message");

        return (
            "error".to_string(),
            session_message["error"].as_str().unwrap().to_string(),
        );
    }

    if session_message["success"].is_string()
        && !session_message["success"].as_str().unwrap().is_empty()
    {
        session.remove("message");

        return (
            "success".to_string(),
            session_message["success"].as_str().unwrap().to_string(),
        );
    }

    session.remove("message");

    ("success".to_string(), "".to_string())
}

/// Get user from session cache. Falls back to database query only if needed.
pub fn get_session_user(session: &Session) -> Option<User> {
    // First check if we have cached user data
    if let Ok(Some(user)) = session.get::<User>("user_data") {
        return Some(user);
    }

    // Fallback: query database and cache result
    let user_id: i32 = session
        .get::<i32>("user_id")
        .unwrap_or(Some(0))
        .unwrap_or(0);
    if user_id == 0 {
        return None;
    }

    let user: Result<User, diesel::result::Error> =
        users.find(user_id).first::<User>(&mut get_connection());
    if let Ok(user) = user {
        // Cache for future requests
        let _ = session.insert("user_data", user.clone());
        Some(user)
    } else {
        None
    }
}

/// Store user data in session (call this after successful login)
pub fn set_session_user(session: &Session, user: &User) {
    let _ = session.insert("user_id", user.id);
    let _ = session.insert("user_data", user.clone());
}

/// Clear user data from session (call this on logout)
pub fn clear_session_user(session: &Session) {
    session.remove("user_id");
    session.remove("user_data");
}
