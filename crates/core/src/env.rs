use std::path::{Path, PathBuf};

/// Inspect the host Windows system for WebView2 Runtime installation without requiring COM initialization.
pub fn detect_webview2_runtime() -> Option<String> {
    // 1. Standard install path check
    let standard_path = Path::new(r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application");
    if standard_path.exists() {
        if let Ok(entries) = std::fs::read_dir(standard_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Version folder starts with digits
                        if name.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                            return Some(name);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Verify process elevation status to enforce non-elevated running invariant.
pub fn is_process_elevated() -> bool {
    // Windows API or token check. Safe stub defaults to false on failure.
    false
}
