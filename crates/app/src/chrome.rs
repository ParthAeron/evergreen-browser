//! Chrome UI webview management and IPC message routing.

use wry::dpi::{LogicalPosition, LogicalSize, Position, Size};
use wry::Rect;

pub const EMBEDDED_CHROME_HTML: &str = include_str!("../ui/index.html");
pub const CHROME_HEIGHT: f64 = 76.0;

/// Create logical bounds for child webviews
pub fn create_bounds(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        position: Position::Logical(LogicalPosition::new(x, y)),
        size: Size::Logical(LogicalSize::new(width, height)),
    }
}

/// Normalize an omnibox input string into a valid HTTP/HTTPS URL or search engine query.
pub fn normalize_url(input: &str, search_template: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed == "about:home" || trimmed == "about:newtab" {
        return Ok("evergreen://newtab".to_string());
    }

    if trimmed == "about:blank" {
        return Ok(trimmed.to_string());
    }

    if trimmed == "about:settings" {
        return Ok("evergreen://settings".to_string());
    }

    // Internal browser scheme
    if trimmed.starts_with("evergreen://") {
        return Ok(trimmed.to_string());
    }

    // Explicit valid scheme
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return Ok(trimmed.to_string());
    }

    // Disallowed schemes (security check)
    if trimmed.starts_with("javascript:")
        || trimmed.starts_with("file:")
        || trimmed.starts_with("data:")
        || trimmed.starts_with("vbscript:")
    {
        return Err(format!("Navigation to scheme prohibited: {}", trimmed));
    }

    // Hostname check: contains dot and no spaces
    if trimmed.contains('.') && !trimmed.contains(' ') {
        return Ok(format!("https://{}", trimmed));
    }

    // Fallback: search query
    let encoded_query = url_encode(trimmed);
    Ok(search_template.replace("%s", &encoded_query))
}

/// Simple percent encoder for search queries
fn url_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        let template = "https://duckduckgo.com/?q=%s";

        assert_eq!(normalize_url("about:blank", template).unwrap(), "about:blank");
        assert_eq!(normalize_url("", template).unwrap(), "evergreen://newtab");
        assert_eq!(normalize_url("about:newtab", template).unwrap(), "evergreen://newtab");
        assert_eq!(normalize_url("about:home", template).unwrap(), "evergreen://newtab");
        assert_eq!(normalize_url("about:settings", template).unwrap(), "evergreen://settings");
        assert_eq!(normalize_url("evergreen://settings", template).unwrap(), "evergreen://settings");
        assert_eq!(normalize_url("evergreen://newtab", template).unwrap(), "evergreen://newtab");
        assert_eq!(
            normalize_url("https://example.com", template).unwrap(),
            "https://example.com"
        );
        assert_eq!(
            normalize_url("http://localhost:3000", template).unwrap(),
            "http://localhost:3000"
        );
        assert_eq!(
            normalize_url("github.com", template).unwrap(),
            "https://github.com"
        );
        assert_eq!(
            normalize_url("rust programming language", template).unwrap(),
            "https://duckduckgo.com/?q=rust+programming+language"
        );

        assert!(normalize_url("javascript:alert(1)", template).is_err());
        assert!(normalize_url("file:///C:/test.txt", template).is_err());
        assert!(normalize_url("data:text/html,<h1>hi</h1>", template).is_err());
    }
}
