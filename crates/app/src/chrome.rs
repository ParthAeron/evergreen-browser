//! Chrome UI webview management and IPC message routing.

use wry::dpi::{LogicalPosition, LogicalSize, Position, Size};
use wry::Rect;

pub const RAW_CHROME_HTML: &str = include_str!("../ui/index.html");
pub const LOGO_BASE64: &str = include_str!("../ui/logo.b64");

pub fn get_chrome_html(search_engine: &str) -> String {
    let display_name = match search_engine.to_lowercase().as_str() {
        "google" => "Google",
        "bing" => "Bing",
        "brave" => "Brave",
        "ecosia" => "Ecosia",
        _ => "DuckDuckGo",
    };
    RAW_CHROME_HTML
        .replace("{{LOGO_BASE64}}", LOGO_BASE64.trim())
        .replace("{{SEARCH_ENGINE_NAME}}", display_name)
}

pub const CHROME_HEIGHT: f64 = 76.0;

/// Create logical bounds for child webviews
pub fn create_bounds(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        position: Position::Logical(LogicalPosition::new(x, y)),
        size: Size::Logical(LogicalSize::new(width, height)),
    }
}

pub use evergreen_core::tabs::normalize_url;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        let template = "https://duckduckgo.com/?q=%s";

        assert_eq!(
            normalize_url("about:blank", template).unwrap(),
            "about:blank"
        );
        assert_eq!(normalize_url("", template).unwrap(), "evergreen://newtab");
        assert_eq!(
            normalize_url("about:newtab", template).unwrap(),
            "evergreen://newtab"
        );
        assert_eq!(
            normalize_url("about:home", template).unwrap(),
            "evergreen://newtab"
        );
        assert_eq!(
            normalize_url("about:settings", template).unwrap(),
            "evergreen://settings"
        );
        assert_eq!(
            normalize_url("evergreen://settings", template).unwrap(),
            "evergreen://settings"
        );
        assert_eq!(
            normalize_url("evergreen://newtab", template).unwrap(),
            "evergreen://newtab"
        );
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

    #[test]
    fn test_get_chrome_html_placeholder() {
        let html_ddg = get_chrome_html("duckduckgo");
        assert!(html_ddg.contains("Search DuckDuckGo or enter web address..."));

        let html_google = get_chrome_html("google");
        assert!(html_google.contains("Search Google or enter web address..."));
    }
}
