use std::path::{Path, PathBuf};

/// Check whether the browser should run in zero-residue portable mode.
/// Portable mode is active if:
/// 1. The `--portable` CLI argument is passed, OR
/// 2. A `portable.lock` or `portable.ini` file exists adjacent to the executable, OR
/// 3. A `user_data` or `data` directory already exists adjacent to the executable.
pub fn is_portable_mode(exe_dir: &Path, args: &[String]) -> bool {
    if args.iter().any(|a| a == "--portable") {
        return true;
    }
    if exe_dir.join("portable.lock").exists() || exe_dir.join("portable.ini").exists() {
        return true;
    }
    if exe_dir.join("user_data").is_dir() || exe_dir.join("data").is_dir() {
        return true;
    }
    false
}

/// Resolves the storage root for settings, cache, and profile data.
/// Returns `exe_dir.join("data")` or `exe_dir.join("user_data")` in portable mode,
/// or `%APPDATA%\evergreen-browser` in standard mode.
pub fn resolve_data_directory(exe_dir: &Path, args: &[String]) -> PathBuf {
    if is_portable_mode(exe_dir, args) {
        if exe_dir.join("data").is_dir() {
            exe_dir.join("data")
        } else {
            exe_dir.join("user_data")
        }
    } else if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("evergreen-browser")
    } else {
        exe_dir.join("evergreen-browser-data")
    }
}

/// Helper to parse a dotted version string (e.g. "153.0.4234.48") into numerical components for semver sorting.
pub fn parse_version_tuple(v: &str) -> Option<Vec<u64>> {
    let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse::<u64>().ok()).collect();
    if parts.len() >= 2 {
        Some(parts)
    } else {
        None
    }
}

/// Inspect the host Windows system for WebView2 Runtime installation without requiring COM initialization.
/// Evaluates system and per-user paths, sorting multiple installed directories to reliably return the latest active version.
pub fn detect_webview2_runtime() -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();

    let search_paths = [
        PathBuf::from(r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application"),
        PathBuf::from(r"C:\Program Files\Microsoft\EdgeWebView\Application"),
    ];

    for path in &search_paths {
        if path.exists() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if parse_version_tuple(&name).is_some() {
                                candidates.push(name);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        let user_path = PathBuf::from(local_appdata).join(r"Microsoft\EdgeWebView\Application");
        if user_path.exists() {
            if let Ok(entries) = std::fs::read_dir(user_path) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if parse_version_tuple(&name).is_some() {
                                candidates.push(name);
                            }
                        }
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| {
        let pa = parse_version_tuple(a).unwrap_or_default();
        let pb = parse_version_tuple(b).unwrap_or_default();
        pa.cmp(&pb)
    });

    candidates.pop()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_tuple_sorting() {
        let v1 = "153.0.4234.32";
        let v2 = "153.0.4234.48";
        let v3 = "154.0.100.1";
        let v_invalid = "SetupMetrics";

        assert_eq!(parse_version_tuple(v_invalid), None);
        assert!(parse_version_tuple(v1).is_some());

        let mut list = vec![v1.to_string(), v3.to_string(), v2.to_string()];
        list.sort_by(|a, b| {
            let pa = parse_version_tuple(a).unwrap_or_default();
            let pb = parse_version_tuple(b).unwrap_or_default();
            pa.cmp(&pb)
        });

        assert_eq!(list, vec![v1.to_string(), v2.to_string(), v3.to_string()]);
        assert_eq!(list.pop(), Some("154.0.100.1".to_string()));
    }
}
