use std::path::{Path, PathBuf};

/// Check whether the browser should run in zero-residue portable mode.
/// Portable mode is active if:
/// 1. The `--portable` CLI argument is passed, OR
/// 2. A `portable.lock` file exists in the directory containing the executable, OR
/// 3. A `user_data` directory already exists adjacent to the executable.
pub fn is_portable_mode(exe_dir: &Path, args: &[String]) -> bool {
    if args.iter().any(|a| a == "--portable") {
        return true;
    }
    if exe_dir.join("portable.lock").exists() {
        return true;
    }
    if exe_dir.join("user_data").is_dir() {
        return true;
    }
    false
}

/// Resolves the storage root for settings, cache, and profile data.
/// Returns `exe_dir.join("user_data")` in portable mode, or `%APPDATA%\evergreen-browser`
/// in standard mode.
pub fn resolve_data_directory(exe_dir: &Path, args: &[String]) -> PathBuf {
    if is_portable_mode(exe_dir, args) {
        exe_dir.join("user_data")
    } else if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("evergreen-browser")
    } else {
        exe_dir.join("evergreen-browser-data")
    }
}

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

