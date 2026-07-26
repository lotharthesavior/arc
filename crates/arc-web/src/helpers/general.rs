// General helper utilities.
//
// Password hashing lives in application services, not in the framework.

/// Generate a Gravatar URL from an email address.
/// Uses MD5 hash of the lowercase, trimmed email as per Gravatar spec.
/// Returns a 200x200 pixel image with "identicon" as the default for emails without a Gravatar.
pub fn gravatar_url(email: &str) -> String {
    let email_normalized = email.trim().to_lowercase();
    let hash = format!("{:x}", md5::compute(email_normalized.as_bytes()));
    format!("https://www.gravatar.com/avatar/{}?s=200&d=identicon", hash)
}
