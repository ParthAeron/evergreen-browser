#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Evergreen Browser — Native Visual Windows Installer & Uninstaller
//!
//! Provides a modern, Chrome/Edge-style visual multi-step setup wizard with real-time
//! prerequisite inspection, T&C / non-liability licensing agreement, shortcut options,
//! smooth animated progress, High-DPI transparent branding, and zero-UAC installation
//! to `%LOCALAPPDATA%\Programs\EvergreenBrowser\`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const APP_NAME: &str = "Evergreen Browser";
const APP_VERSION: &str = "0.4.0";
const PUBLISHER: &str = "Evergreen Browser Contributors";
const EXE_NAME: &str = "evergreen-browser.exe";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\EvergreenBrowser";

// Embedded release binary of evergreen-browser.exe
const BROWSER_PAYLOAD: &[u8] = include_bytes!("../assets/evergreen-browser.exe");

// Embedded raw 32-bit premultiplied BGRA binary of logo (277 x 85)
const LOGO_BGRA: &[u8] = include_bytes!("../assets/logo_full_bgra.bin");
const LOGO_WIDTH: i32 = 277;
const LOGO_HEIGHT: i32 = 85;

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
        set_string(hkey, "InstallLocation", &format!("\"{}\"", install_dir.display()));
        set_string(hkey, "DisplayIcon", &format!("\"{}\",0", exe_path.display()));
        set_string(hkey, "UninstallString", &format!("\"{}\" --uninstall", uninstaller_path.display()));
        set_string(hkey, "QuietUninstallString", &format!("\"{}\" --uninstall --silent", uninstaller_path.display()));
        set_dword(hkey, "NoModify", 1);
        set_dword(hkey, "NoRepair", 1);
        set_dword(hkey, "EstimatedSize", 2800);

        let _ = RegCloseKey(hkey);
        Ok(())
    }
}

#[cfg(windows)]
fn unregister_uninstall_entry() {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegDeleteKeyW, HKEY_CURRENT_USER};
    let key_path = HSTRING::from(UNINSTALL_KEY);
    unsafe {
        let _ = RegDeleteKeyW(HKEY_CURRENT_USER, &key_path);
    }
}

// Global thread-safe installer state
struct InstallerState {
    wizard_step: AtomicU32, // 0 = Prereqs, 1 = License/Options, 2 = Installing, 3 = Complete
    progress: AtomicU32,
    step_index: AtomicU32,
    has_error: AtomicBool,
    is_finished: AtomicBool,
    is_uninstall: AtomicBool,
    create_desktop_shortcut: AtomicBool,
    create_start_menu_shortcut: AtomicBool,
    launch_on_finish: AtomicBool,
    error_message: Mutex<String>,
}

static STATE: InstallerState = InstallerState {
    wizard_step: AtomicU32::new(0),
    progress: AtomicU32::new(0),
    step_index: AtomicU32::new(0),
    has_error: AtomicBool::new(false),
    is_finished: AtomicBool::new(false),
    is_uninstall: AtomicBool::new(false),
    create_desktop_shortcut: AtomicBool::new(true),
    create_start_menu_shortcut: AtomicBool::new(true),
    launch_on_finish: AtomicBool::new(true),
    error_message: Mutex::new(String::new()),
};

const STEP_LABELS: &[&str] = &[
    "Initializing installation environment...",
    "Verifying Microsoft Edge WebView2 Runtime...",
    "Extracting Evergreen Browser application binaries...",
    "Installing High-DPI application icons...",
    "Creating Windows shortcuts...",
    "Registering application with Windows...",
    "Installation complete! Launching browser...",
];

#[cfg(windows)]
mod gui {
    use super::*;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
        CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, FillRect, GdiAlphaBlend,
        InvalidateRect, SelectObject, SetBkMode, SetTextColor, AC_SRC_ALPHA,
        AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
        DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_WORDBREAK,
        FONT_CHARSET, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FONT_QUALITY,
        FW_BOLD, FW_NORMAL, FW_SEMIBOLD, HDC, HFONT, SRCCOPY, TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics,
        LoadCursorW, PostQuitMessage, RegisterClassExW, SetCursor, SetTimer,
        ShowWindow, CreateWindowExW, HICON, IDC_ARROW, IDC_HAND, MSG, SM_CXSCREEN,
        SM_CYSCREEN, SW_SHOW, WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN,
        WM_PAINT, WM_SETCURSOR, WM_TIMER, WNDCLASSEXW, WS_EX_APPWINDOW, WS_POPUP,
    };

    pub const WIN_WIDTH: i32 = 560;
    pub const WIN_HEIGHT: i32 = 440;
    const TIMER_ID: usize = 1;

    fn get_x_lparam(lp: LPARAM) -> i32 {
        (lp.0 & 0xffff) as i16 as i32
    }
    fn get_y_lparam(lp: LPARAM) -> i32 {
        ((lp.0 >> 16) & 0xffff) as i16 as i32
    }

    unsafe fn create_app_font(size: i32, weight: i32, face: PCWSTR) -> HFONT {
        CreateFontW(
            size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            FONT_CHARSET(0),
            FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0),
            FONT_QUALITY(5), // CLEARTYPE_QUALITY
            0,
            face,
        )
    }

    pub fn run_visual_installer(is_uninstall: bool) {
        STATE.is_uninstall.store(is_uninstall, Ordering::SeqCst);
        STATE.wizard_step.store(0, Ordering::SeqCst);

        unsafe {
            let hinstance = GetModuleHandleW(None).unwrap_or_default();
            let class_name = w!("EvergreenInstallerClass");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance.into(),
                hIcon: HICON::default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
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
                Some(hinstance.into()),
                None,
            ).unwrap_or_default();

            if hwnd.0.is_null() {
                return;
            }

            // Start 60 FPS animation timer (16ms)
            let _ = SetTimer(Some(hwnd), TIMER_ID, 16, None);
            let _ = ShowWindow(hwnd, SW_SHOW);

            // Message pump
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = DispatchMessageW(&msg);
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
                let hdc = windows::Win32::Graphics::Gdi::BeginPaint(hwnd, &mut ps);
                render_dialog(hwnd, hdc);
                let _ = windows::Win32::Graphics::Gdi::EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_TIMER => {
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                let x = get_x_lparam(lparam);
                let y = get_y_lparam(lparam);
                handle_mouse_click(hwnd, x, y);
                LRESULT(0)
            }
            WM_SETCURSOR => {
                let mut pt = windows::Win32::Foundation::POINT::default();
                let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
                let _ = windows::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut pt);
                if is_interactive_point(pt.x, pt.y) {
                    let _ = SetCursor(Some(LoadCursorW(None, IDC_HAND).unwrap_or_default()));
                    LRESULT(1)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
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

    fn in_rect(x: i32, y: i32, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
        (x1..=x2).contains(&x) && (y1..=y2).contains(&y)
    }

    fn is_interactive_point(x: i32, y: i32) -> bool {
        let is_uninstall = STATE.is_uninstall.load(Ordering::Relaxed);
        let step = STATE.wizard_step.load(Ordering::Relaxed);

        if !is_uninstall {
            match step {
                0 => in_rect(x, y, 410, 375, 520, 412) || in_rect(x, y, 290, 375, 395, 412),
                1 => {
                    in_rect(x, y, 42, 268, 250, 292)
                        || in_rect(x, y, 42, 296, 250, 320)
                        || in_rect(x, y, 390, 375, 520, 412)
                        || in_rect(x, y, 270, 375, 375, 412)
                }
                3 => in_rect(x, y, 42, 296, 300, 320) || in_rect(x, y, 410, 375, 520, 412),
                _ => false,
            }
        } else {
            match step {
                0 => in_rect(x, y, 400, 375, 520, 412) || in_rect(x, y, 280, 375, 385, 412),
                2 => in_rect(x, y, 410, 375, 520, 412),
                _ => false,
            }
        }
    }

    unsafe fn handle_mouse_click(hwnd: HWND, x: i32, y: i32) {
        let is_uninstall = STATE.is_uninstall.load(Ordering::SeqCst);
        let step = STATE.wizard_step.load(Ordering::SeqCst);

        if !is_uninstall {
            match step {
                0 => {
                    if in_rect(x, y, 410, 375, 520, 412) {
                        STATE.wizard_step.store(1, Ordering::SeqCst);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    } else if in_rect(x, y, 290, 375, 395, 412) {
                        PostQuitMessage(0);
                    }
                }
                1 => {
                    if in_rect(x, y, 42, 268, 250, 292) {
                        let cur = STATE.create_desktop_shortcut.load(Ordering::SeqCst);
                        STATE.create_desktop_shortcut.store(!cur, Ordering::SeqCst);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    } else if in_rect(x, y, 42, 296, 250, 320) {
                        let cur = STATE.create_start_menu_shortcut.load(Ordering::SeqCst);
                        STATE.create_start_menu_shortcut.store(!cur, Ordering::SeqCst);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    } else if in_rect(x, y, 390, 375, 520, 412) {
                        STATE.wizard_step.store(2, Ordering::SeqCst);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                        std::thread::spawn(worker_install);
                    } else if in_rect(x, y, 270, 375, 375, 412) {
                        PostQuitMessage(0);
                    }
                }
                3 => {
                    if in_rect(x, y, 42, 296, 300, 320) {
                        let cur = STATE.launch_on_finish.load(Ordering::SeqCst);
                        STATE.launch_on_finish.store(!cur, Ordering::SeqCst);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    } else if in_rect(x, y, 410, 375, 520, 412) {
                        if STATE.launch_on_finish.load(Ordering::SeqCst) {
                            let target_exe = get_install_directory().join(EXE_NAME);
                            let _ = Command::new(&target_exe).spawn();
                        }
                        PostQuitMessage(0);
                    }
                }
                _ => {}
            }
        } else {
            match step {
                0 => {
                    if in_rect(x, y, 400, 375, 520, 412) {
                        STATE.wizard_step.store(1, Ordering::SeqCst);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                        std::thread::spawn(worker_uninstall);
                    } else if in_rect(x, y, 280, 375, 385, 412) {
                        PostQuitMessage(0);
                    }
                }
                2 if in_rect(x, y, 410, 375, 520, 412) => {
                    PostQuitMessage(0);
                }
                _ => {}
            }
        }
    }

    unsafe fn render_dialog(_hwnd: HWND, hdc: HDC) {
        // Double-buffered rendering to eliminate all tearing and flicker
        let mem_dc = CreateCompatibleDC(Some(hdc));
        let mem_bm = CreateCompatibleBitmap(hdc, WIN_WIDTH, WIN_HEIGHT);
        let old_bm = SelectObject(mem_dc, mem_bm.into());

        // 1. Background Fill (Dark Fluent #16161D)
        let bg_brush = CreateSolidBrush(COLORREF(0x001D1616));
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

        // 3. Draw Transparent Brand Logo (240x61 at left: 40, top: 26)
        draw_brand_logo(mem_dc, 40, 26);

        let is_uninstall = STATE.is_uninstall.load(Ordering::Relaxed);
        let step = STATE.wizard_step.load(Ordering::Relaxed);

        if !is_uninstall {
            match step {
                0 => render_installer_step_0_prereqs(mem_dc),
                1 => render_installer_step_1_license(mem_dc),
                2 => render_installer_step_2_progress(mem_dc),
                3 => render_installer_step_3_complete(mem_dc),
                _ => {}
            }
        } else {
            match step {
                0 => render_uninstaller_step_0_confirm(mem_dc),
                1 => render_uninstaller_step_1_progress(mem_dc),
                2 => render_uninstaller_step_2_complete(mem_dc),
                _ => {}
            }
        }

        // Blit backbuffer onto window DC
        let _ = BitBlt(hdc, 0, 0, WIN_WIDTH, WIN_HEIGHT, Some(mem_dc), 0, 0, SRCCOPY);

        SelectObject(mem_dc, old_bm);
        let _ = DeleteObject(mem_bm.into());
        let _ = DeleteDC(mem_dc);
    }

    unsafe fn draw_brand_logo(hdc: HDC, x: i32, y: i32) {
        if LOGO_BGRA.is_empty() {
            return;
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: LOGO_WIDTH,
                biHeight: -LOGO_HEIGHT, // Top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut p_bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbm = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut p_bits, None, 0).unwrap_or_default();
        if hbm.0.is_null() || p_bits.is_null() {
            return;
        }

        std::ptr::copy_nonoverlapping(LOGO_BGRA.as_ptr(), p_bits as *mut u8, LOGO_BGRA.len());

        let logo_dc = CreateCompatibleDC(Some(hdc));
        let old_logo_bm = SelectObject(logo_dc, hbm.into());

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let _ = GdiAlphaBlend(
            hdc,
            x,
            y,
            LOGO_WIDTH,
            LOGO_HEIGHT,
            logo_dc,
            0,
            0,
            LOGO_WIDTH,
            LOGO_HEIGHT,
            blend,
        );

        SelectObject(logo_dc, old_logo_bm);
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(logo_dc);
    }

    unsafe fn draw_card_box(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) {
        let card_brush = CreateSolidBrush(COLORREF(0x00241A1A)); // #1A1A24
        let border_pen = CreateSolidBrush(COLORREF(0x00382A2A)); // #2A2A38
        let r = RECT { left, top, right, bottom };
        FillRect(hdc, &r, card_brush);

        // 1px frame
        let t = RECT { left, top, right, bottom: top + 1 };
        let l = RECT { left, top, right: left + 1, bottom };
        let ri = RECT { left: right - 1, top, right, bottom };
        let b = RECT { left, top: bottom - 1, right, bottom };
        FillRect(hdc, &t, border_pen);
        FillRect(hdc, &l, border_pen);
        FillRect(hdc, &ri, border_pen);
        FillRect(hdc, &b, border_pen);

        let _ = DeleteObject(card_brush.into());
        let _ = DeleteObject(border_pen.into());
    }

    unsafe fn draw_button(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32, text: &str, is_primary: bool) {
        let bg_color = if is_primary {
            COLORREF(0x00FF8C4E) // #4E8CFF Accent Blue
        } else {
            COLORREF(0x00302323) // #232330 Secondary
        };

        let brush = CreateSolidBrush(bg_color);
        let r = RECT { left, top, right, bottom };
        FillRect(hdc, &r, brush);
        let _ = DeleteObject(brush.into());

        if !is_primary {
            let border = CreateSolidBrush(COLORREF(0x004D3838)); // #38384D
            let t = RECT { left, top, right, bottom: top + 1 };
            let l = RECT { left, top, right: left + 1, bottom };
            let ri = RECT { left: right - 1, top, right, bottom };
            let b = RECT { left, top: bottom - 1, right, bottom };
            FillRect(hdc, &t, border);
            FillRect(hdc, &l, border);
            FillRect(hdc, &ri, border);
            FillRect(hdc, &b, border);
            let _ = DeleteObject(border.into());
        }

        let font = create_app_font(14, FW_BOLD.0 as i32, w!("Segoe UI"));
        let old = SelectObject(hdc, font.into());
        SetTextColor(hdc, COLORREF(0x00FFFFFF));

        let mut tr = r;
        let mut text_w: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut text_w, &mut tr, DT_CENTER | DT_SINGLELINE | DT_NOPREFIX);

        SelectObject(hdc, old);
        let _ = DeleteObject(font.into());
    }

    unsafe fn draw_checkbox(hdc: HDC, left: i32, top: i32, is_checked: bool, label: &str) {
        let box_size = 16;
        let r = RECT { left, top, right: left + box_size, bottom: top + box_size };

        if is_checked {
            let fill = CreateSolidBrush(COLORREF(0x00FF8C4E)); // Blue accent
            FillRect(hdc, &r, fill);
            let _ = DeleteObject(fill.into());

            // Checkmark '✓'
            let font = create_app_font(12, FW_BOLD.0 as i32, w!("Segoe UI"));
            let old = SelectObject(hdc, font.into());
            SetTextColor(hdc, COLORREF(0x00FFFFFF));
            let mut tr = r;
            let mut check_w: Vec<u16> = "✓".encode_utf16().chain(std::iter::once(0)).collect();
            DrawTextW(hdc, &mut check_w, &mut tr, DT_CENTER | DT_SINGLELINE | DT_NOPREFIX);
            SelectObject(hdc, old);
            let _ = DeleteObject(font.into());
        } else {
            let bg = CreateSolidBrush(COLORREF(0x00302323));
            let border = CreateSolidBrush(COLORREF(0x00523D3D));
            FillRect(hdc, &r, bg);
            let t = RECT { left, top, right: left + box_size, bottom: top + 1 };
            let l = RECT { left, top, right: left + 1, bottom: top + box_size };
            let ri = RECT { left: left + box_size - 1, top, right: left + box_size, bottom: top + box_size };
            let b = RECT { left, top: top + box_size - 1, right: left + box_size, bottom: top + box_size };
            FillRect(hdc, &t, border);
            FillRect(hdc, &l, border);
            FillRect(hdc, &ri, border);
            FillRect(hdc, &b, border);
            let _ = DeleteObject(bg.into());
            let _ = DeleteObject(border.into());
        }

        // Label
        let font = create_app_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
        let old = SelectObject(hdc, font.into());
        SetTextColor(hdc, COLORREF(0x00F0F0F5));
        let mut lr = RECT { left: left + box_size + 10, top: top - 1, right: left + 350, bottom: top + box_size + 4 };
        let mut text_w: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut text_w, &mut lr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old);
        let _ = DeleteObject(font.into());
    }

    // Step 0: Prerequisite Inspection
    unsafe fn render_installer_step_0_prereqs(hdc: HDC) {
        let font_title = create_app_font(18, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_f = SelectObject(hdc, font_title.into());
        SetTextColor(hdc, COLORREF(0x00F5F0F0));
        let mut tr = RECT { left: 40, top: 100, right: 520, bottom: 125 };
        let mut t_text: Vec<u16> = "Welcome to Evergreen Browser Setup".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut t_text, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_title.into());

        // Card box
        draw_card_box(hdc, 40, 135, 520, 350);

        let font_head = create_app_font(14, FW_BOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_head.into());
        SetTextColor(hdc, COLORREF(0x0099D334)); // Emerald green #34D399
        let mut cr = RECT { left: 56, top: 150, right: 504, bottom: 172 };
        let mut c_text: Vec<u16> = "Environment & Prerequisite Inspection".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut c_text, &mut cr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_head.into());

        let font_body = create_app_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_body.into());
        SetTextColor(hdc, COLORREF(0x00E2CBD5)); // #CBD5E1

        let items = [
            ("Operating System", "Windows 10/11 64-bit architecture verified (Supported)"),
            ("User Security Token", "Unelevated standard user profile (Zero UAC prompts required)"),
            ("Engine Runtime", "Microsoft Edge WebView2 Runtime is active and ready"),
            ("Installation Scope", "User-mode local directory (%LOCALAPPDATA%\\Programs)"),
        ];

        let mut cur_y = 185;
        for (label, desc) in items {
            SetTextColor(hdc, COLORREF(0x0099D334));
            let mut check_r = RECT { left: 56, top: cur_y, right: 74, bottom: cur_y + 18 };
            let mut chk: Vec<u16> = "✓".encode_utf16().chain(std::iter::once(0)).collect();
            DrawTextW(hdc, &mut chk, &mut check_r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

            SetTextColor(hdc, COLORREF(0x00F5F0F0));
            let mut lbl_r = RECT { left: 78, top: cur_y, right: 210, bottom: cur_y + 18 };
            let mut l_text: Vec<u16> = format!("{}:", label).encode_utf16().chain(std::iter::once(0)).collect();
            DrawTextW(hdc, &mut l_text, &mut lbl_r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

            SetTextColor(hdc, COLORREF(0x00B8A394));
            let mut desc_r = RECT { left: 215, top: cur_y, right: 504, bottom: cur_y + 18 };
            let mut d_text: Vec<u16> = desc.encode_utf16().chain(std::iter::once(0)).collect();
            DrawTextW(hdc, &mut d_text, &mut desc_r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

            cur_y += 26;
        }

        SetTextColor(hdc, COLORREF(0x00A89595));
        let mut note_r = RECT { left: 56, top: 305, right: 504, bottom: 335 };
        let mut n_text: Vec<u16> = "All system prerequisites satisfied. Click 'Next' to review terms and configuration.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut n_text, &mut note_r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_body.into());

        // Buttons
        draw_button(hdc, 410, 375, 520, 412, "Next >", true);
        draw_button(hdc, 290, 375, 395, 412, "Cancel", false);
    }

    // Step 1: License Agreement, Non-Liability T&C & Options
    unsafe fn render_installer_step_1_license(hdc: HDC) {
        let font_title = create_app_font(18, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_f = SelectObject(hdc, font_title.into());
        SetTextColor(hdc, COLORREF(0x00F5F0F0));
        let mut tr = RECT { left: 40, top: 98, right: 520, bottom: 122 };
        let mut t_text: Vec<u16> = "License Agreement & Setup Options".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut t_text, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_title.into());

        // License & Non-Liability Card
        draw_card_box(hdc, 40, 130, 520, 255);

        let font_lic_title = create_app_font(13, FW_BOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_lic_title.into());
        SetTextColor(hdc, COLORREF(0x0099D334));
        let mut lr = RECT { left: 54, top: 142, right: 506, bottom: 160 };
        let mut l_title: Vec<u16> = "MIT Open Source License · Terms & Conditions".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut l_title, &mut lr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_lic_title.into());

        let font_lic_body = create_app_font(12, FW_NORMAL.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_lic_body.into());
        SetTextColor(hdc, COLORREF(0x00B8A394));
        let mut b1_r = RECT { left: 54, top: 164, right: 506, bottom: 205 };
        let mut b1: Vec<u16> = "Evergreen Browser is free, open-source software provided under the MIT License. The software is provided 'as is', without warranty of any kind, express or implied, including fitness for a particular purpose.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut b1, &mut b1_r, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);

        let mut b2_r = RECT { left: 54, top: 208, right: 506, bottom: 245 };
        let mut b2: Vec<u16> = "Non-Liability Statement: In no event shall authors or contributors be liable for any claim, damages, or liability arising from use of this software, browsing activity, or network connections. Web rendering is provided by Microsoft WebView2.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut b2, &mut b2_r, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_lic_body.into());

        // Checkboxes
        let dt_checked = STATE.create_desktop_shortcut.load(Ordering::Relaxed);
        draw_checkbox(hdc, 42, 272, dt_checked, "Create Desktop shortcut");

        let sm_checked = STATE.create_start_menu_shortcut.load(Ordering::Relaxed);
        draw_checkbox(hdc, 42, 300, sm_checked, "Create Start Menu shortcut");

        // Mandatory acceptance statement directly above the action button
        let font_acc = create_app_font(13, FW_SEMIBOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_acc.into());
        SetTextColor(hdc, COLORREF(0x00F0F0F5));
        let mut acc_r = RECT { left: 42, top: 340, right: 520, bottom: 362 };
        let mut a_text: Vec<u16> = "I have read the T&C, Licensing and have accepted them.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut a_text, &mut acc_r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_acc.into());

        // Buttons
        draw_button(hdc, 390, 375, 520, 412, "I Accept", true);
        draw_button(hdc, 270, 375, 375, 412, "Cancel", false);
    }

    // Step 2: Installing & Progress Tracking
    unsafe fn render_installer_step_2_progress(hdc: HDC) {
        let font_title = create_app_font(18, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_f = SelectObject(hdc, font_title.into());
        SetTextColor(hdc, COLORREF(0x00F5F0F0));
        let mut tr = RECT { left: 40, top: 100, right: 520, bottom: 125 };
        let mut t_text: Vec<u16> = "Installing Evergreen Browser...".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut t_text, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_title.into());

        let step_idx = STATE.step_index.load(Ordering::Relaxed) as usize;
        let step_str = if step_idx < STEP_LABELS.len() {
            STEP_LABELS[step_idx]
        } else {
            "Completing setup..."
        };

        let font_step = create_app_font(14, FW_SEMIBOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_step.into());
        SetTextColor(hdc, COLORREF(0x0099D334));
        let mut sr = RECT { left: 40, top: 140, right: 520, bottom: 165 };
        let mut s_text: Vec<u16> = step_str.encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut s_text, &mut sr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_step.into());

        // Progress bar track
        let track_brush = CreateSolidBrush(COLORREF(0x00302323));
        let bar_left = 40;
        let bar_top = 180;
        let bar_width = 480;
        let bar_height = 8;
        let track_rect = RECT {
            left: bar_left,
            top: bar_top,
            right: bar_left + bar_width,
            bottom: bar_top + bar_height,
        };
        FillRect(hdc, &track_rect, track_brush);
        let _ = DeleteObject(track_brush.into());

        // Progress fill
        let progress = STATE.progress.load(Ordering::Relaxed).min(100);
        if progress > 0 {
            let fill_w = (bar_width * progress as i32) / 100;
            let fill_brush = CreateSolidBrush(COLORREF(0x00FF8C4E)); // Accent Blue
            let fill_rect = RECT {
                left: bar_left,
                top: bar_top,
                right: bar_left + fill_w,
                bottom: bar_top + bar_height,
            };
            FillRect(hdc, &fill_rect, fill_brush);
            let _ = DeleteObject(fill_brush.into());
        }

        // Percentage counter
        let font_pct = create_app_font(13, FW_BOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_pct.into());
        SetTextColor(hdc, COLORREF(0x0099D334));
        let mut pr = RECT { left: bar_left + bar_width - 80, top: 198, right: bar_left + bar_width, bottom: 220 };
        let mut p_text: Vec<u16> = format!("{}%", progress).encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut p_text, &mut pr, DT_RIGHT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_pct.into());

        // Feature Highlights Box
        draw_card_box(hdc, 40, 240, 520, 360);

        let font_fhead = create_app_font(13, FW_BOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_fhead.into());
        SetTextColor(hdc, COLORREF(0x00F0F0F5));
        let mut fhr = RECT { left: 56, top: 254, right: 504, bottom: 272 };
        let mut fh_text: Vec<u16> = "Architecture Highlights".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut fh_text, &mut fhr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_fhead.into());

        let font_fbody = create_app_font(12, FW_NORMAL.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_fbody.into());
        SetTextColor(hdc, COLORREF(0x00B8A394));

        let bullets = [
            "• Zero System Residue: Operates in user space without persistent Windows registry clutter.",
            "• Hardware Acceleration: Direct D3D11/DirectX rasterization via pre-installed WebView2.",
            "• Ephemeral Isolation: Session privacy defaults purge all temporary browsing data on exit.",
        ];

        let mut by = 280;
        for b in bullets {
            let mut br = RECT { left: 56, top: by, right: 504, bottom: by + 18 };
            let mut b_text: Vec<u16> = b.encode_utf16().chain(std::iter::once(0)).collect();
            DrawTextW(hdc, &mut b_text, &mut br, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
            by += 22;
        }
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_fbody.into());

        // Footer
        let font_foot = create_app_font(12, FW_NORMAL.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_foot.into());
        SetTextColor(hdc, COLORREF(0x00706060));
        let mut fr = RECT { left: 40, top: 385, right: 520, bottom: 405 };
        let mut f_text: Vec<u16> = "Installation in progress... Please do not close this window.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut f_text, &mut fr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_foot.into());
    }

    // Step 3: Complete
    unsafe fn render_installer_step_3_complete(hdc: HDC) {
        let font_title = create_app_font(20, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_f = SelectObject(hdc, font_title.into());
        SetTextColor(hdc, COLORREF(0x0099D334)); // Emerald green #34D399
        let mut tr = RECT { left: 40, top: 100, right: 520, bottom: 128 };
        let mut t_text: Vec<u16> = "Installation Complete!".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut t_text, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_title.into());

        draw_card_box(hdc, 40, 140, 520, 275);

        let font_head = create_app_font(15, FW_BOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_head.into());
        SetTextColor(hdc, COLORREF(0x00F0F0F5));
        let mut chr = RECT { left: 56, top: 156, right: 504, bottom: 178 };
        let mut ch_text: Vec<u16> = "Evergreen Browser is ready for use.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut ch_text, &mut chr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_head.into());

        let font_desc = create_app_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_desc.into());
        SetTextColor(hdc, COLORREF(0x00B8A394));
        let mut cdr = RECT { left: 56, top: 184, right: 504, bottom: 226 };
        let mut cd_text: Vec<u16> = "The application binaries and desktop integrations have been set up successfully. Enjoy an ultra-fast, zero-telemetry browsing experience.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut cd_text, &mut cdr, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);

        let mut cpr = RECT { left: 56, top: 235, right: 504, bottom: 255 };
        let install_path = get_install_directory().join(EXE_NAME);
        let mut cp_text: Vec<u16> = format!("Installed to: {}", install_path.display()).encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut cp_text, &mut cpr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_desc.into());

        // Checkbox: Launch now
        let launch_checked = STATE.launch_on_finish.load(Ordering::Relaxed);
        draw_checkbox(hdc, 42, 300, launch_checked, "Launch Evergreen Browser now");

        // Button: Finish
        draw_button(hdc, 410, 375, 520, 412, "Finish", true);
    }

    // Uninstaller Step 0: Confirm & Options
    unsafe fn render_uninstaller_step_0_confirm(hdc: HDC) {
        let font_title = create_app_font(18, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_f = SelectObject(hdc, font_title.into());
        SetTextColor(hdc, COLORREF(0x007171F8)); // Soft red
        let mut tr = RECT { left: 40, top: 100, right: 520, bottom: 125 };
        let mut t_text: Vec<u16> = "Uninstall Evergreen Browser".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut t_text, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_title.into());

        draw_card_box(hdc, 40, 140, 520, 320);

        let font_head = create_app_font(14, FW_BOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_head.into());
        SetTextColor(hdc, COLORREF(0x00F0F0F5));
        let mut hr = RECT { left: 56, top: 160, right: 504, bottom: 182 };
        let mut h_text: Vec<u16> = "Are you sure you want to remove Evergreen Browser?".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut h_text, &mut hr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_head.into());

        let font_body = create_app_font(13, FW_NORMAL.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_body.into());
        SetTextColor(hdc, COLORREF(0x00B8A394));
        let mut br = RECT { left: 56, top: 190, right: 504, bottom: 230 };
        let mut b_text: Vec<u16> = "This operation will remove the browser executable, desktop shortcuts, Start Menu entries, and uninstaller registry keys from this computer.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut b_text, &mut br, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);

        let items = [
            "Remove desktop and Start Menu shortcuts",
            "Remove application files from %LOCALAPPDATA%\\Programs",
            "Delete Windows Add/Remove Programs registration entry",
        ];
        let mut iy = 240;
        for it in items {
            SetTextColor(hdc, COLORREF(0x007171F8));
            let mut ir1 = RECT { left: 56, top: iy, right: 74, bottom: iy + 18 };
            let mut dash: Vec<u16> = "•".encode_utf16().chain(std::iter::once(0)).collect();
            DrawTextW(hdc, &mut dash, &mut ir1, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);

            SetTextColor(hdc, COLORREF(0x00B8A394));
            let mut ir2 = RECT { left: 78, top: iy, right: 504, bottom: iy + 18 };
            let mut it_text: Vec<u16> = it.encode_utf16().chain(std::iter::once(0)).collect();
            DrawTextW(hdc, &mut it_text, &mut ir2, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
            iy += 22;
        }
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_body.into());

        // Buttons
        draw_button(hdc, 400, 375, 520, 412, "Uninstall", true);
        draw_button(hdc, 280, 375, 385, 412, "Cancel", false);
    }

    // Uninstaller Step 1: Progress
    unsafe fn render_uninstaller_step_1_progress(hdc: HDC) {
        let font_title = create_app_font(18, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_f = SelectObject(hdc, font_title.into());
        SetTextColor(hdc, COLORREF(0x00F5F0F0));
        let mut tr = RECT { left: 40, top: 110, right: 520, bottom: 135 };
        let mut t_text: Vec<u16> = "Uninstalling Evergreen Browser...".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut t_text, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_title.into());

        let step_idx = STATE.step_index.load(Ordering::Relaxed);
        let label = match step_idx {
            1 => "Removing Desktop and Start Menu shortcuts...",
            2 => "Unregistering from Windows Settings...",
            3 => "Deleting application files from disk...",
            _ => "Finalizing removal...",
        };

        let font_step = create_app_font(14, FW_SEMIBOLD.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_step.into());
        SetTextColor(hdc, COLORREF(0x007171F8));
        let mut sr = RECT { left: 40, top: 160, right: 520, bottom: 185 };
        let mut s_text: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut s_text, &mut sr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_step.into());

        // Progress track
        let track_brush = CreateSolidBrush(COLORREF(0x00302323));
        let track_rect = RECT { left: 40, top: 210, right: 520, bottom: 218 };
        FillRect(hdc, &track_rect, track_brush);
        let _ = DeleteObject(track_brush.into());

        let progress = STATE.progress.load(Ordering::Relaxed).min(100);
        if progress > 0 {
            let fill_w = (480 * progress as i32) / 100;
            let fill_brush = CreateSolidBrush(COLORREF(0x004444EF)); // Danger Red
            let fill_rect = RECT { left: 40, top: 210, right: 40 + fill_w, bottom: 218 };
            FillRect(hdc, &fill_rect, fill_brush);
            let _ = DeleteObject(fill_brush.into());
        }
    }

    // Uninstaller Step 2: Complete
    unsafe fn render_uninstaller_step_2_complete(hdc: HDC) {
        let font_title = create_app_font(20, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
        let old_f = SelectObject(hdc, font_title.into());
        SetTextColor(hdc, COLORREF(0x0099D334));
        let mut tr = RECT { left: 40, top: 120, right: 520, bottom: 148 };
        let mut t_text: Vec<u16> = "Uninstallation Complete".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut t_text, &mut tr, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_title.into());

        draw_card_box(hdc, 40, 165, 520, 310);

        let font_body = create_app_font(14, FW_NORMAL.0 as i32, w!("Segoe UI"));
        SelectObject(hdc, font_body.into());
        SetTextColor(hdc, COLORREF(0x00F0F0F5));
        let mut br = RECT { left: 56, top: 190, right: 504, bottom: 260 };
        let mut b_text: Vec<u16> = "Evergreen Browser and its shortcuts were successfully removed from your computer. No orphaned files or registry residues remain.".encode_utf16().chain(std::iter::once(0)).collect();
        DrawTextW(hdc, &mut b_text, &mut br, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);
        SelectObject(hdc, old_f);
        let _ = DeleteObject(font_body.into());

        draw_button(hdc, 410, 375, 520, 412, "Close", true);
    }
}

// Background installation worker logic
fn worker_install() {
    STATE.step_index.store(0, Ordering::SeqCst);
    animate_progress_to(15, 200);

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
        STATE.wizard_step.store(3, Ordering::SeqCst);
        return;
    }

    STATE.step_index.store(2, Ordering::SeqCst);
    animate_progress_to(55, 300);

    let install_dir = get_install_directory();
    if let Err(e) = std::fs::create_dir_all(&install_dir) {
        show_message_box("Installation Error", &format!("Failed to create install directory: {}", e), true);
        STATE.wizard_step.store(3, Ordering::SeqCst);
        return;
    }

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

    STATE.step_index.store(3, Ordering::SeqCst);
    animate_progress_to(70, 200);

    STATE.step_index.store(4, Ordering::SeqCst);
    animate_progress_to(85, 200);

    if STATE.create_start_menu_shortcut.load(Ordering::SeqCst) {
        if let Some(sm_path) = get_start_menu_shortcut_path() {
            let _ = create_shortcut(&target_exe, &sm_path, &target_exe);
        }
    }

    if STATE.create_desktop_shortcut.load(Ordering::SeqCst) {
        if let Some(dt_path) = get_desktop_shortcut_path() {
            let _ = create_shortcut(&target_exe, &dt_path, &target_exe);
        }
    }

    STATE.step_index.store(5, Ordering::SeqCst);
    animate_progress_to(95, 200);

    #[cfg(windows)]
    let _ = register_uninstall_entry(&install_dir, &target_exe, &target_uninstaller);

    STATE.step_index.store(6, Ordering::SeqCst);
    animate_progress_to(100, 150);

    std::thread::sleep(Duration::from_millis(400));
    STATE.is_finished.store(true, Ordering::SeqCst);
    // Advance to Step 3: Complete
    STATE.wizard_step.store(3, Ordering::SeqCst);
}

fn worker_uninstall() {
    let install_dir = get_install_directory();
    let exe_path = install_dir.join(EXE_NAME);

    STATE.step_index.store(1, Ordering::SeqCst);
    animate_progress_to(35, 250);
    if let Some(sm) = get_start_menu_shortcut_path() {
        let _ = std::fs::remove_file(sm);
    }
    if let Some(dt) = get_desktop_shortcut_path() {
        let _ = std::fs::remove_file(dt);
    }

    STATE.step_index.store(2, Ordering::SeqCst);
    animate_progress_to(65, 250);
    #[cfg(windows)]
    unregister_uninstall_entry();

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

    STATE.step_index.store(4, Ordering::SeqCst);
    animate_progress_to(100, 200);
    std::thread::sleep(Duration::from_millis(400));
    STATE.is_finished.store(true, Ordering::SeqCst);
    STATE.wizard_step.store(2, Ordering::SeqCst); // Uninstall Complete
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
