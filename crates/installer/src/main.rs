#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Evergreen Browser — Native Visual Windows Installer & Uninstaller
//!
//! Provides a modern, Chrome/Edge-style visual installer dialog with real-time
//! step visualization, smooth animated progress, High-DPI branding, zero-UAC installation
//! to `%LOCALAPPDATA%\Programs\EvergreenBrowser\`, and native uninstallation support.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

const APP_NAME: &str = "Evergreen Browser";
const APP_VERSION: &str = "0.4.0";
const PUBLISHER: &str = "Evergreen Browser Contributors";
const EXE_NAME: &str = "evergreen-browser.exe";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\EvergreenBrowser";

// Embedded release binary of evergreen-browser.exe
const BROWSER_PAYLOAD: &[u8] = include_bytes!("../assets/evergreen-browser.exe");

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

// Global installation state shared between worker and GUI thread
struct InstallerState {
    progress: AtomicU32,       // 0 to 100
    step_index: AtomicU32,     // 0..=5
    is_finished: AtomicBool,
    has_error: AtomicBool,
    error_message: std::sync::Mutex<String>,
}

static STATE: InstallerState = InstallerState {
    progress: AtomicU32::new(0),
    step_index: AtomicU32::new(0),
    is_finished: AtomicBool::new(false),
    has_error: AtomicBool::new(false),
    error_message: std::sync::Mutex::new(String::new()),
};

const STEP_LABELS: &[&str] = &[
    "Preparing installation...",
    "Checking system environment and WebView2 Runtime...",
    "Extracting Evergreen Browser application files...",
    "Creating Start Menu and Desktop shortcuts...",
    "Registering application in Windows...",
    "Installation complete! Starting Evergreen Browser...",
];

#[cfg(windows)]
mod gui {
    use super::*;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
        CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect,
        SelectObject, SetBkMode, SetTextColor, DT_LEFT,
        DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, FONT_CHARSET, FONT_CLIP_PRECISION,
        FONT_OUTPUT_PRECISION, FONT_QUALITY, FW_BOLD, FW_SEMIBOLD,
        HDC, HFONT, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics,
        KillTimer, PostMessageW, PostQuitMessage, RegisterClassExW, SetTimer, ShowWindow,
        TranslateMessage, HICON, HTCAPTION, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW,
        WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_NCLBUTTONDOWN, WM_PAINT,
        WM_TIMER, WNDCLASSEXW, WS_EX_APPWINDOW, WS_POPUP,
    };

    const WIN_WIDTH: i32 = 460;
    const WIN_HEIGHT: i32 = 270;
    const TIMER_ID: usize = 1;

    unsafe fn create_app_font(size: i32, weight: i32, facename: PCWSTR) -> HFONT {
        CreateFontW(
            size, 0, 0, 0, weight, 0, 0, 0,
            FONT_CHARSET(1),
            FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0),
            FONT_QUALITY(5), // CLEARTYPE_QUALITY
            0,
            facename,
        )
    }

    pub fn run_visual_installer(is_uninstall: bool) {
        unsafe {
            let class_name = w!("EvergreenSetupWindow");
            let hinstance: HINSTANCE = HINSTANCE::default();

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: HICON::default(),
                hCursor: windows::Win32::UI::WindowsAndMessaging::LoadCursorW(None, windows::Win32::UI::WindowsAndMessaging::IDC_ARROW).unwrap_or_default(),
                hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
                hIconSm: HICON::default(),
            };

            RegisterClassExW(&wnd_class);

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let x = (screen_w - WIN_WIDTH) / 2;
            let y = (screen_h - WIN_HEIGHT) / 2;

            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                class_name,
                w!("Evergreen Browser Setup"),
                WS_POPUP,
                x,
                y,
                WIN_WIDTH,
                WIN_HEIGHT,
                None,
                None,
                Some(hinstance),
                None,
            ).unwrap_or_default();

            if hwnd.0.is_null() {
                return;
            }

            // Start 60 FPS animation timer (16ms)
            let _ = SetTimer(Some(hwnd), TIMER_ID, 16, None);
            let _ = ShowWindow(hwnd, SW_SHOW);

            // Spawn background worker thread
            if is_uninstall {
                std::thread::spawn(move || {
                    super::worker_uninstall();
                });
            } else {
                std::thread::spawn(move || {
                    super::worker_install();
                });
            }

            // Standard Windows message pump
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                render_dialog(hwnd, hdc);

                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1), // Prevent flickering
            WM_TIMER => {
                if wparam.0 == TIMER_ID {
                    // Check if finished
                    if STATE.is_finished.load(Ordering::SeqCst) {
                        let _ = KillTimer(Some(hwnd), TIMER_ID);
                        std::thread::sleep(Duration::from_millis(600));
                        PostQuitMessage(0);
                        return LRESULT(0);
                    }
                    // Trigger repaint for smooth progress animation
                    let rect = RECT { left: 0, top: 0, right: WIN_WIDTH, bottom: WIN_HEIGHT };
                    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), Some(&rect), false);
                }
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                // Allow dragging window from anywhere
                let _ = PostMessageW(Some(hwnd), WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), lparam);
                LRESULT(0)
            }
            WM_KEYDOWN => {
                if wparam.0 == 0x1B { // VK_ESCAPE
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn render_dialog(_hwnd: HWND, hdc: HDC) {
        // Double-buffered rendering to eliminate all tearing and flicker
        let mem_dc = CreateCompatibleDC(Some(hdc));
        let mem_bm = CreateCompatibleBitmap(hdc, WIN_WIDTH, WIN_HEIGHT);
        let old_bm = SelectObject(mem_dc, mem_bm.into());

        // 1. Background Fill (Dark Fluent #16161D)
        let bg_brush = CreateSolidBrush(COLORREF(0x001D1616)); // 0x00BBGGRR -> 0x16161D
        let bg_rect = RECT { left: 0, top: 0, right: WIN_WIDTH, bottom: WIN_HEIGHT };
        FillRect(mem_dc, &bg_rect, bg_brush);
        let _ = DeleteObject(bg_brush.into());

        // 2. Card Border (Subtle 1px border #2A2A38)
        let border_brush = CreateSolidBrush(COLORREF(0x00382A2A));
        let top_border = RECT { left: 0, top: 0, right: WIN_WIDTH, bottom: 1 };
        let left_border = RECT { left: 0, top: 0, right: 1, bottom: WIN_HEIGHT };
        let right_border = RECT { left: WIN_WIDTH - 1, top: 0, right: WIN_WIDTH, bottom: WIN_HEIGHT };
        let bottom_border = RECT { left: 0, top: WIN_HEIGHT - 1, right: WIN_WIDTH, bottom: WIN_HEIGHT };
        FillRect(mem_dc, &top_border, border_brush);
        FillRect(mem_dc, &left_border, border_brush);
        FillRect(mem_dc, &right_border, border_brush);
        FillRect(mem_dc, &bottom_border, border_brush);
        let _ = DeleteObject(border_brush.into());

        let _ = SetBkMode(mem_dc, TRANSPARENT);

        // 3. Draw Header Title ("Evergreen Browser Setup")
        let title_font = create_app_font(22, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_font = SelectObject(mem_dc, title_font.into());
        SetTextColor(mem_dc, COLORREF(0x00F5F0F0)); // #F0F0F5

        let mut title_rect = RECT { left: 40, top: 38, right: WIN_WIDTH - 40, bottom: 68 };
        let title_text: Vec<u16> = format!("{} Setup", APP_NAME).encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(mem_dc, &mut title_text.clone(), &mut title_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

        // 4. Draw Version Tag
        let ver_font = create_app_font(13, FW_SEMIBOLD.0 as i32, w!("Segoe UI"));
        SelectObject(mem_dc, ver_font.into());
        SetTextColor(mem_dc, COLORREF(0x00B8A394)); // #94A3B8

        let mut ver_rect = RECT { left: 40, top: 72, right: WIN_WIDTH - 40, bottom: 92 };
        let ver_text: Vec<u16> = format!("Version {}", APP_VERSION).encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(mem_dc, &mut ver_text.clone(), &mut ver_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

        // 5. Draw Dynamic Status Step Description
        let step_idx = STATE.step_index.load(Ordering::Relaxed) as usize;
        let step_str = if step_idx < STEP_LABELS.len() {
            STEP_LABELS[step_idx]
        } else {
            "Completing..."
        };

        let desc_font = create_app_font(15, FW_SEMIBOLD.0 as i32, w!("Segoe UI"));
        SelectObject(mem_dc, desc_font.into());
        SetTextColor(mem_dc, COLORREF(0x00E2CBD5)); // #CBD5E1

        let mut desc_rect = RECT { left: 40, top: 122, right: WIN_WIDTH - 40, bottom: 146 };
        let desc_text: Vec<u16> = step_str.encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(mem_dc, &mut desc_text.clone(), &mut desc_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

        // 6. Draw Progress Bar Track
        let track_brush = CreateSolidBrush(COLORREF(0x00302323)); // #232330
        let bar_left = 40;
        let bar_top = 160;
        let bar_width = WIN_WIDTH - 80; // 380px
        let bar_height = 8;
        let track_rect = RECT {
            left: bar_left,
            top: bar_top,
            right: bar_left + bar_width,
            bottom: bar_top + bar_height,
        };
        FillRect(mem_dc, &track_rect, track_brush);
        let _ = DeleteObject(track_brush.into());

        // 7. Draw Progress Bar Fill
        let progress = STATE.progress.load(Ordering::Relaxed).min(100);
        if progress > 0 {
            let fill_w = (bar_width * progress as i32) / 100;
            // Vibrant Cyan-Blue to Emerald Accent (RGB: 78, 140, 255 -> 0x00FF8C4E)
            let fill_brush = CreateSolidBrush(COLORREF(0x00FF8C4E));
            let fill_rect = RECT {
                left: bar_left,
                top: bar_top,
                right: bar_left + fill_w,
                bottom: bar_top + bar_height,
            };
            FillRect(mem_dc, &fill_rect, fill_brush);
            let _ = DeleteObject(fill_brush.into());
        }

        // 8. Percentage Counter Label
        let pct_font = create_app_font(13, FW_SEMIBOLD.0 as i32, w!("Segoe UI"));
        SelectObject(mem_dc, pct_font.into());
        SetTextColor(mem_dc, COLORREF(0x0099D334)); // #34D399 (Emerald Green)

        let mut pct_rect = RECT { left: bar_left + bar_width - 60, top: 176, right: bar_left + bar_width, bottom: 196 };
        let pct_text: Vec<u16> = format!("{}%", progress).encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(mem_dc, &mut pct_text.clone(), &mut pct_rect, DT_RIGHT | DT_SINGLELINE | DT_NOPREFIX);

        // 9. Footer Info: "Fast User Installation · Zero System Residue"
        let footer_font = create_app_font(12, 400, w!("Segoe UI"));
        SelectObject(mem_dc, footer_font.into());
        SetTextColor(mem_dc, COLORREF(0x00706060)); // Subtle muted

        let mut footer_rect = RECT { left: 40, top: 224, right: WIN_WIDTH - 40, bottom: 244 };
        let footer_text: Vec<u16> = "Fast User Installation · Zero System Residue".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(mem_dc, &mut footer_text.clone(), &mut footer_rect, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

        // Blit backbuffer onto window DC
        let _ = BitBlt(hdc, 0, 0, WIN_WIDTH, WIN_HEIGHT, Some(mem_dc), 0, 0, SRCCOPY);

        // Cleanup GDI objects
        SelectObject(mem_dc, old_font);
        let _ = DeleteObject(title_font.into());
        let _ = DeleteObject(ver_font.into());
        let _ = DeleteObject(desc_font.into());
        let _ = DeleteObject(pct_font.into());
        let _ = DeleteObject(footer_font.into());

        SelectObject(mem_dc, old_bm);
        let _ = DeleteObject(mem_bm.into());
        let _ = DeleteDC(mem_dc);
    }
}

// Background installation worker logic
fn worker_install() {
    // Step 0: Initializing
    STATE.step_index.store(0, Ordering::SeqCst);
    animate_progress_to(15, 200);

    // Step 1: Check WebView2 Runtime
    STATE.step_index.store(1, Ordering::SeqCst);
    animate_progress_to(30, 250);

    if evergreen_core::env::detect_webview2_runtime().is_none() {
        STATE.has_error.store(true, Ordering::SeqCst);
        *STATE.error_message.lock().unwrap() = "Microsoft Edge WebView2 Runtime is required to run Evergreen Browser.".to_string();
        show_message_box(
            "Prerequisite Missing — Evergreen Browser",
            "Microsoft Edge WebView2 Runtime is required to run Evergreen Browser.\n\nPlease install the WebView2 Runtime from Microsoft or Windows Update, then launch setup again.",
            true,
        );
        STATE.is_finished.store(true, Ordering::SeqCst);
        return;
    }

    // Step 2: Extracting Files
    STATE.step_index.store(2, Ordering::SeqCst);
    animate_progress_to(55, 300);

    let install_dir = get_install_directory();
    if let Err(e) = std::fs::create_dir_all(&install_dir) {
        show_message_box("Installation Error", &format!("Failed to create install directory: {}", e), true);
        STATE.is_finished.store(true, Ordering::SeqCst);
        return;
    }

    let target_exe = install_dir.join(EXE_NAME);
    if BROWSER_PAYLOAD.is_empty() {
        let dev_src = Path::new("target").join("release").join(EXE_NAME);
        if dev_src.exists() {
            let _ = std::fs::copy(&dev_src, &target_exe);
        }
    } else {
        let _ = std::fs::write(&target_exe, BROWSER_PAYLOAD);
    }

    let target_uninstaller = install_dir.join("uninstall.exe");
    if let Ok(cur_exe) = std::env::current_exe() {
        let _ = std::fs::copy(cur_exe, &target_uninstaller);
    }

    // Step 3: Configuring Shortcuts
    STATE.step_index.store(3, Ordering::SeqCst);
    animate_progress_to(75, 250);

    if let Some(sm_path) = get_start_menu_shortcut_path() {
        let _ = create_shortcut(&target_exe, &sm_path, &target_exe);
    }
    if let Some(dt_path) = get_desktop_shortcut_path() {
        let _ = create_shortcut(&target_exe, &dt_path, &target_exe);
    }

    // Step 4: Registering Application
    STATE.step_index.store(4, Ordering::SeqCst);
    animate_progress_to(90, 200);

    #[cfg(windows)]
    let _ = register_uninstall_entry(&install_dir, &target_exe, &target_uninstaller);

    // Step 5: Finished & Launching
    STATE.step_index.store(5, Ordering::SeqCst);
    animate_progress_to(100, 200);

    let _ = Command::new(&target_exe).spawn();
    std::thread::sleep(Duration::from_millis(400));
    STATE.is_finished.store(true, Ordering::SeqCst);
}

// Background uninstallation worker logic
fn worker_uninstall() {
    let install_dir = get_install_directory();
    let exe_path = install_dir.join(EXE_NAME);

    // Step 1: Remove shortcuts
    STATE.step_index.store(1, Ordering::SeqCst);
    animate_progress_to(35, 250);
    if let Some(sm) = get_start_menu_shortcut_path() {
        let _ = std::fs::remove_file(sm);
    }
    if let Some(dt) = get_desktop_shortcut_path() {
        let _ = std::fs::remove_file(dt);
    }

    // Step 2: Unregister
    STATE.step_index.store(2, Ordering::SeqCst);
    animate_progress_to(65, 250);
    #[cfg(windows)]
    unregister_uninstall_entry();

    // Step 3: Remove program files
    STATE.step_index.store(3, Ordering::SeqCst);
    animate_progress_to(90, 250);
    let _ = std::fs::remove_file(&exe_path);
    let uninstaller_exe = install_dir.join("uninstall.exe");
    let current_exe = std::env::current_exe().unwrap_or_default();

    if current_exe != uninstaller_exe {
        let _ = std::fs::remove_file(&uninstaller_exe);
        let _ = std::fs::remove_dir_all(&install_dir);
    } else {
        let cmd_str = format!(
            "timeout /t 2 /nobreak > NUL & del /f /q \"{}\" & rmdir /s /q \"{}\"",
            uninstaller_exe.display(),
            install_dir.display()
        );
        let _ = Command::new("cmd.exe").args(["/c", &cmd_str]).spawn();
    }

    // Step 4: Finished
    STATE.step_index.store(4, Ordering::SeqCst);
    animate_progress_to(100, 200);
    std::thread::sleep(Duration::from_millis(500));
    STATE.is_finished.store(true, Ordering::SeqCst);
}

fn animate_progress_to(target: u32, duration_ms: u64) {
    let current = STATE.progress.load(Ordering::Relaxed);
    if target <= current {
        return;
    }
    let steps = (target - current) as u64;
    let sleep_per_step = (duration_ms / steps).max(5);

    for p in (current + 1)..=target {
        STATE.progress.store(p, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(sleep_per_step));
    }
}

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

fn run_silent_install() {
    if evergreen_core::env::detect_webview2_runtime().is_none() {
        return;
    }

    let install_dir = get_install_directory();
    let _ = std::fs::create_dir_all(&install_dir);
    let target_exe = install_dir.join(EXE_NAME);

    if !BROWSER_PAYLOAD.is_empty() {
        let _ = std::fs::write(&target_exe, BROWSER_PAYLOAD);
    } else {
        let dev_src = Path::new("target").join("release").join(EXE_NAME);
        if dev_src.exists() {
            let _ = std::fs::copy(&dev_src, &target_exe);
        }
    }

    let target_uninstaller = install_dir.join("uninstall.exe");
    if let Ok(cur_exe) = std::env::current_exe() {
        let _ = std::fs::copy(cur_exe, &target_uninstaller);
    }

    if let Some(sm_path) = get_start_menu_shortcut_path() {
        let _ = create_shortcut(&target_exe, &sm_path, &target_exe);
    }
    if let Some(dt_path) = get_desktop_shortcut_path() {
        let _ = create_shortcut(&target_exe, &dt_path, &target_exe);
    }

    #[cfg(windows)]
    let _ = register_uninstall_entry(&install_dir, &target_exe, &target_uninstaller);

    let _ = Command::new(&target_exe).spawn();
}

fn run_silent_uninstall() {
    let install_dir = get_install_directory();
    let exe_path = install_dir.join(EXE_NAME);

    if let Some(sm) = get_start_menu_shortcut_path() {
        let _ = std::fs::remove_file(sm);
    }
    if let Some(dt) = get_desktop_shortcut_path() {
        let _ = std::fs::remove_file(dt);
    }

    #[cfg(windows)]
    unregister_uninstall_entry();

    let _ = std::fs::remove_file(&exe_path);
    let uninstaller_exe = install_dir.join("uninstall.exe");
    let current_exe = std::env::current_exe().unwrap_or_default();

    if current_exe != uninstaller_exe {
        let _ = std::fs::remove_file(&uninstaller_exe);
        let _ = std::fs::remove_dir_all(&install_dir);
    } else {
        let cmd_str = format!(
            "timeout /t 2 /nobreak > NUL & del /f /q \"{}\" & rmdir /s /q \"{}\"",
            uninstaller_exe.display(),
            install_dir.display()
        );
        let _ = Command::new("cmd.exe").args(["/c", &cmd_str]).spawn();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_silent = args.iter().any(|a| a == "--silent" || a == "-s");
    let is_uninstall = args.iter().any(|a| a == "--uninstall" || a == "-u");

    if is_silent {
        if is_uninstall {
            run_silent_uninstall();
        } else {
            run_silent_install();
        }
        return;
    }

    #[cfg(windows)]
    gui::run_visual_installer(is_uninstall);

    #[cfg(not(windows))]
    {
        if is_uninstall {
            run_silent_uninstall();
        } else {
            run_silent_install();
        }
    }
}
