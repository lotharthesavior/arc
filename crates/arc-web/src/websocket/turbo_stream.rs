/// Turbo Stream HTML generation utilities
/// These functions generate Turbo Stream-formatted HTML that can be sent
/// over WebSocket to update the DOM in real-time.
/// Generate a Turbo Stream append action
/// Appends content to the end of the target element
pub fn turbo_stream_append(target: &str, html: &str) -> String {
    format!(
        r#"<turbo-stream action="append" target="{}">
  <template>{}</template>
</turbo-stream>"#,
        target, html
    )
}

/// Generate a Turbo Stream prepend action
/// Prepends content to the beginning of the target element
pub fn turbo_stream_prepend(target: &str, html: &str) -> String {
    format!(
        r#"<turbo-stream action="prepend" target="{}">
  <template>{}</template>
</turbo-stream>"#,
        target, html
    )
}

/// Generate a Turbo Stream replace action
/// Replaces the entire target element (including itself)
pub fn turbo_stream_replace(target: &str, html: &str) -> String {
    format!(
        r#"<turbo-stream action="replace" target="{}">
  <template>{}</template>
</turbo-stream>"#,
        target, html
    )
}

/// Generate a Turbo Stream update action
/// Replaces the innerHTML of the target element
pub fn turbo_stream_update(target: &str, html: &str) -> String {
    format!(
        r#"<turbo-stream action="update" target="{}">
  <template>{}</template>
</turbo-stream>"#,
        target, html
    )
}

/// Generate a Turbo Stream remove action
/// Removes the target element from the DOM
pub fn turbo_stream_remove(target: &str) -> String {
    format!(
        r#"<turbo-stream action="remove" target="{}"></turbo-stream>"#,
        target
    )
}

/// Generate a Turbo Stream before action
/// Inserts content before the target element
pub fn turbo_stream_before(target: &str, html: &str) -> String {
    format!(
        r#"<turbo-stream action="before" target="{}">
  <template>{}</template>
</turbo-stream>"#,
        target, html
    )
}

/// Generate a Turbo Stream after action
/// Inserts content after the target element
pub fn turbo_stream_after(target: &str, html: &str) -> String {
    format!(
        r#"<turbo-stream action="after" target="{}">
  <template>{}</template>
</turbo-stream>"#,
        target, html
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turbo_stream_append() {
        let result = turbo_stream_append("notifications", "<div>Hello</div>");
        assert!(result.contains(r#"action="append""#));
        assert!(result.contains(r#"target="notifications""#));
        assert!(result.contains("<div>Hello</div>"));
    }

    #[test]
    fn test_turbo_stream_remove() {
        let result = turbo_stream_remove("item-123");
        assert!(result.contains(r#"action="remove""#));
        assert!(result.contains(r#"target="item-123""#));
    }
}
