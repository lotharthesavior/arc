/// Extracts a URL-encoded form field value from a raw request body string.
/// Returns a descriptive fallback message if the field is missing or invalid.
pub fn get_from_form_body(field: String, req_body: String) -> String {
    req_body
        .split('&')
        .find(|param| param.starts_with(&format!("{}=", field)))
        .and_then(|param| param.split('=').nth(1))
        .map(|field_found| {
            urlencoding::decode(field_found)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| format!("Invalid {}", field))
        })
        .unwrap_or_else(|| format!("No {} provided", field))
}
