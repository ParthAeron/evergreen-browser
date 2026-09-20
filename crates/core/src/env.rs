use std::path::Path;

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
                        if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
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
#[cfg(windows)]
pub fn is_process_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: We query the current process token with TOKEN_QUERY.
    // The HANDLE is closed cleanly before returning.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0u32;
        let success = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        let _ = CloseHandle(token);

        if success.is_ok() {
            elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

/// Fallback for non-Windows platforms (always false).
#[cfg(not(windows))]
pub fn is_process_elevated() -> bool {
    false
}

