#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Evergreen Browser — Native Windows Installer & Uninstaller
//!
//! Provides a fast, zero-UAC, per-user setup experience that installs
//! Evergreen Browser to `%LOCALAPPDATA%\Programs\EvergreenBrowser\`,
//! creates Start Menu and Desktop shortcuts, registers Windows Add/Remove
//! Programs entries, and launches the browser immediately.

use std::path::{Path, PathBuf};
use std::process::Command;

const APP_NAME: &str = "Evergreen Browser";
const APP_VERSION: &str = "0.4.0";
const PUBLISHER: &str = "Evergreen Browser Contributors";
const EXE_NAME: &str = "evergreen-browser.exe";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\EvergreenBrowser";

// Embedded release binary of evergreen-browser.exe
const BROWSER_PAYLOAD: &[u8] = include_bytes!("../assets/evergreen-browser.exe");

#[cfg(windows)]
fn show_message_box(title: &str, text: &str, is_error: bool) {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    };

    let flags = if is_error {
        MB_OK | MB_ICONERROR
    } else {
        MB_OK | MB_ICONINFORMATION
    };

    let title_h = HSTRING::from(title);
    let text_h = HSTRING::from(text);

    unsafe {
        let _ = MessageBoxW(None, &text_h, &title_h, flags);
    }
}

#[cfg(not(windows))]
fn show_message_box(_title: &str, text: &str, is_error: bool) {
    if is_error {
        eprintln!("{}", text);
    } else {
        println!("{}", text);
    }
}

fn get_install_directory() -> PathBuf {
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_appdata).join("Programs").join("EvergreenBrowser")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile).join("AppData").join("Local").join("Programs").join("EvergreenBrowser")
    } else {
        PathBuf::from("EvergreenBrowser")
    }
}

fn get_start_menu_shortcut_path() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        Some(PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join(format!("{}.lnk", APP_NAME)))
    } else {
        None
    }
}

fn get_desktop_shortcut_path() -> Option<PathBuf> {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        Some(PathBuf::from(userprofile).join("Desktop").join(format!("{}.lnk", APP_NAME)))
    } else {
        None
    }
}

/// Create a Windows shortcut (.lnk) using PowerShell COM automation for universal compatibility.
fn create_shortcut(target_exe: &Path, shortcut_path: &Path, icon_path: &Path) -> Result<(), String> {
    if let Some(parent) = shortcut_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let ps_script = format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}'); $s.TargetPath = '{}'; $s.IconLocation = '{},0'; $s.Description = '{}'; $s.Save()",
        shortcut_path.display().to_string().replace('\'', "''"),
        target_exe.display().to_string().replace('\'', "''"),
        icon_path.display().to_string().replace('\'', "''"),
        APP_NAME
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to invoke PowerShell: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[cfg(windows)]
fn register_uninstall_entry(install_dir: &Path, exe_path: &Path, uninstaller_path: &Path) -> Result<(), String> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::WIN32_ERROR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
        REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let key_path = HSTRING::from(UNINSTALL_KEY);
    let mut hkey = HKEY::default();

    unsafe {
        let res = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &key_path,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        );

        if res != WIN32_ERROR(0) {
            return Err(format!("RegCreateKeyExW failed with error code: {:?}", res));
        }

        fn set_string(hkey: HKEY, name: &str, val: &str) {
            let name_h = HSTRING::from(name);
            let val_wide: Vec<u16> = val.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let _ = RegSetValueExW(
                    hkey,
                    &name_h,
                    None,
                    REG_SZ,
                    Some(std::slice::from_raw_parts(
                        val_wide.as_ptr() as *const u8,
                        val_wide.len() * 2,
                    )),
                );
            }
        }

        fn set_dword(hkey: HKEY, name: &str, val: u32) {
            let name_h = HSTRING::from(name);
            let bytes = val.to_ne_bytes();
            unsafe {
                let _ = RegSetValueExW(
                    hkey,
                    &name_h,
                    None,
                    REG_DWORD,
                    Some(&bytes),
                );
            }
        }

        set_string(hkey, "DisplayName", APP_NAME);
        set_string(hkey, "DisplayVersion", APP_VERSION);
        set_string(hkey, "Publisher", PUBLISHER);
        set_string(hkey, "DisplayIcon", &format!("{},0", exe_path.display()));
        set_string(hkey, "InstallLocation", &install_dir.display().to_string());
        set_string(hkey, "UninstallString", &format!("\"{}\" --uninstall", uninstaller_path.display()));
        set_dword(hkey, "NoModify", 1);
        set_dword(hkey, "NoRepair", 1);
        set_dword(hkey, "EstimatedSize", 2048); // ~2 MB

        let _ = RegCloseKey(hkey);
    }

    Ok(())
}

#[cfg(windows)]
fn unregister_uninstall_entry() {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};

    let key_path = HSTRING::from(UNINSTALL_KEY);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &key_path);
    }
}

fn run_uninstall(is_silent: bool) {
    let install_dir = get_install_directory();
    let exe_path = install_dir.join(EXE_NAME);

    // 1. Remove Shortcuts
    if let Some(sm) = get_start_menu_shortcut_path() {
        let _ = std::fs::remove_file(sm);
    }
    if let Some(dt) = get_desktop_shortcut_path() {
        let _ = std::fs::remove_file(dt);
    }

    // 2. Unregister from Windows Settings / Add-Remove Programs
    #[cfg(windows)]
    unregister_uninstall_entry();

    // 3. Remove Program Files
    let _ = std::fs::remove_file(&exe_path);
    let uninstaller_exe = install_dir.join("uninstall.exe");
    let current_exe = std::env::current_exe().unwrap_or_default();

    if current_exe != uninstaller_exe {
        let _ = std::fs::remove_file(&uninstaller_exe);
        let _ = std::fs::remove_dir_all(&install_dir);
    } else {
        // If running directly as uninstall.exe, schedule self-deletion upon exit
        let cmd_str = format!(
            "timeout /t 2 /nobreak > NUL & del /f /q \"{}\" & rmdir /s /q \"{}\"",
            uninstaller_exe.display(),
            install_dir.display()
        );
        let _ = Command::new("cmd.exe")
            .args(["/c", &cmd_str])
            .spawn();
    }

    if !is_silent {
        show_message_box(
            APP_NAME,
            "Evergreen Browser was successfully removed from your computer.",
            false,
        );
    }
}

fn run_install(is_silent: bool) {
    // 1. Check for WebView2 Runtime prerequisite
    if evergreen_core::env::detect_webview2_runtime().is_none() {
        show_message_box(
            "Prerequisite Missing — Evergreen Browser",
            "Microsoft Edge WebView2 Runtime is required to run Evergreen Browser.\n\nPlease install the WebView2 Runtime from Microsoft or Windows Update, then launch setup again.",
            true,
        );
        return;
    }

    // 2. Prepare target directory
    let install_dir = get_install_directory();
    if let Err(e) = std::fs::create_dir_all(&install_dir) {
        show_message_box(
            "Installation Error",
            &format!("Failed to create installation directory at {}:\n{}", install_dir.display(), e),
            true,
        );
        return;
    }

    let target_exe = install_dir.join(EXE_NAME);

    // 3. Write embedded browser payload (or copy if run during dev testing)
    if BROWSER_PAYLOAD.is_empty() {
        // Fallback for dev builds if payload not embedded yet
        let dev_src = Path::new("target").join("release").join(EXE_NAME);
        if dev_src.exists() {
            let _ = std::fs::copy(&dev_src, &target_exe);
        } else {
            show_message_box(
                "Installation Error",
                "Browser payload is missing from installer package. Please build with scripts/build-installer.ps1.",
                true,
            );
            return;
        }
    } else {
        if let Err(e) = std::fs::write(&target_exe, BROWSER_PAYLOAD) {
            show_message_box(
                "Installation Error",
                &format!("Failed to write application executable to {}:\n{}", target_exe.display(), e),
                true,
            );
            return;
        }
    }

    // 4. Save a copy of this setup executable as uninstall.exe
    let target_uninstaller = install_dir.join("uninstall.exe");
    if let Ok(cur_exe) = std::env::current_exe() {
        let _ = std::fs::copy(cur_exe, &target_uninstaller);
    }

    // 5. Create Start Menu shortcut
    if let Some(sm_path) = get_start_menu_shortcut_path() {
        let _ = create_shortcut(&target_exe, &sm_path, &target_exe);
    }

    // 6. Create Desktop shortcut
    if let Some(dt_path) = get_desktop_shortcut_path() {
        let _ = create_shortcut(&target_exe, &dt_path, &target_exe);
    }

    // 7. Register Windows Add/Remove Programs entry
    #[cfg(windows)]
    let _ = register_uninstall_entry(&install_dir, &target_exe, &target_uninstaller);

    // 8. Launch the installed browser
    let _ = Command::new(&target_exe).spawn();

    // 9. Notify user if non-silent
    if !is_silent {
        // Installation succeeds and browser launches immediately; no blocking alert required.
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_silent = args.iter().any(|a| a == "--silent" || a == "-s");
    let is_uninstall = args.iter().any(|a| a == "--uninstall" || a == "-u");

    if is_uninstall {
        run_uninstall(is_silent);
    } else {
        run_install(is_silent);
    }
}

