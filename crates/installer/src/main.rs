//! Evergreen Browser Setup & Standalone Uninstaller.
//! Built with modern Fluent acrylic aesthetics, SVG icons, and WebView2 rendering.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod installer_ui;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Icon, Window, WindowAttributes, WindowId};
use wry::WebViewBuilder;

const CREATE_NO_WINDOW: u32 = 0x08000000;

const APP_NAME: &str = "Evergreen Browser";
const APP_VERSION: &str = "0.4.0";
const PUBLISHER: &str = "Evergreen Browser Contributors";
const EXE_NAME: &str = "evergreen-browser.exe";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\EvergreenBrowser";

// Embedded assets
const BROWSER_PAYLOAD: &[u8] = include_bytes!("../assets/evergreen-browser.exe");
const LOGO_BASE64: &str = include_str!("../../app/ui/logo_full.b64");
const ICON_RGBA: &[u8] = include_bytes!("../../app/ui/icon_64.rgba");

fn get_install_directory() -> PathBuf {
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_appdata)
            .join("Programs")
            .join("EvergreenBrowser")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile)
            .join("AppData")
            .join("Local")
            .join("Programs")
            .join("EvergreenBrowser")
    } else {
        PathBuf::from("EvergreenBrowser")
    }
}

fn get_start_menu_shortcut_path() -> Option<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        Some(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join(format!("{}.lnk", APP_NAME)),
        )
    } else {
        None
    }
}

fn get_desktop_shortcut_path() -> Option<PathBuf> {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        Some(
            PathBuf::from(userprofile)
                .join("Desktop")
                .join(format!("{}.lnk", APP_NAME)),
        )
    } else {
        None
    }
}

fn create_shortcut(
    target_exe: &Path,
    shortcut_path: &Path,
    icon_path: &Path,
) -> Result<(), String> {
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

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to invoke PowerShell: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn register_uninstall_entry(install_dir: &Path, uninstall_exe: &Path) -> Result<(), String> {
    let uninst_cmd = format!("\"{}\" --uninstall", uninstall_exe.display());
    let target_exe = install_dir.join(EXE_NAME);
    let display_icon = target_exe.display().to_string();

    let ps_script = format!(
        "New-Item -Path 'HKCU:\\{}' -Force | Out-Null; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'DisplayName' -Value '{}'; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'DisplayVersion' -Value '{}'; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'Publisher' -Value '{}'; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'DisplayIcon' -Value '{}'; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'UninstallString' -Value '{}'; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'InstallLocation' -Value '{}'; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'NoModify' -Value 1 -Type DWord; \
         Set-ItemProperty -Path 'HKCU:\\{}' -Name 'NoRepair' -Value 1 -Type DWord",
        UNINSTALL_KEY,
        UNINSTALL_KEY,
        APP_NAME,
        UNINSTALL_KEY,
        APP_VERSION,
        UNINSTALL_KEY,
        PUBLISHER,
        UNINSTALL_KEY,
        display_icon.replace('\'', "''"),
        UNINSTALL_KEY,
        uninst_cmd.replace('\'', "''"),
        UNINSTALL_KEY,
        install_dir.display().to_string().replace('\'', "''"),
        UNINSTALL_KEY,
        UNINSTALL_KEY
    );

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let _ = cmd.output();

    Ok(())
}

fn unregister_uninstall_entry() {
    let ps_script = format!(
        "Remove-Item -Path 'HKCU:\\{}' -Recurse -Force -ErrorAction SilentlyContinue",
        UNINSTALL_KEY
    );
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let _ = cmd.output();
}

#[derive(Debug)]
enum InstallerEvent {
    Progress { pct: u32, text: String },
    InstallComplete,
    UninstallComplete,
    Exit,
}

struct InstallerApp {
    is_uninstall: bool,
    proxy: EventLoopProxy<InstallerEvent>,
    window: Option<Arc<Window>>,
    webview: Option<wry::WebView>,
    is_installing: Arc<AtomicBool>,
}

impl ApplicationHandler<InstallerEvent> for InstallerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title(if self.is_uninstall {
                "Uninstall Evergreen Browser"
            } else {
                "Evergreen Browser Setup"
            })
            .with_inner_size(LogicalSize::new(580.0, 500.0))
            .with_resizable(false)
            .with_theme(Some(winit::window::Theme::Dark));

        let attrs = if let Ok(icon) = Icon::from_rgba(ICON_RGBA.to_vec(), 64, 64) {
            attrs.with_window_icon(Some(icon))
        } else {
            attrs
        };

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("Failed to create installer window: {}", e);
                event_loop.exit();
                return;
            }
        };

        // Center on screen
        if let Some(monitor) = window
            .primary_monitor()
            .or_else(|| window.available_monitors().next())
        {
            let m_size = monitor.size();
            let scale = monitor.scale_factor();
            let w_phys = (580.0 * scale) as u32;
            let h_phys = (500.0 * scale) as u32;
            let x = (m_size.width.saturating_sub(w_phys)) / 2;
            let y = (m_size.height.saturating_sub(h_phys)) / 2;
            window.set_outer_position(LogicalPosition::new(x as f64 / scale, y as f64 / scale));
        }

        let html = installer_ui::get_installer_html(LOGO_BASE64, self.is_uninstall);

        let proxy = self.proxy.clone();
        let is_installing = self.is_installing.clone();
        let is_uninstall_mode = self.is_uninstall;

        let webview_builder = WebViewBuilder::new()
            .with_background_color((22, 22, 29, 255))
            .with_html(html)
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                let msg = req.body();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(msg) {
                    let action = val["action"].as_str().unwrap_or("");
                    match action {
                        "cancel" => {
                            let _ = proxy.send_event(InstallerEvent::Exit);
                        }
                        "accept"
                            if !is_uninstall_mode
                                && !is_installing.swap(true, Ordering::SeqCst) =>
                        {
                            let create_desktop = val["desktop"].as_bool().unwrap_or(true);
                            let create_start_menu = val["start_menu"].as_bool().unwrap_or(true);
                            let worker_proxy = proxy.clone();
                            std::thread::spawn(move || {
                                run_installer_worker(
                                    create_desktop,
                                    create_start_menu,
                                    worker_proxy,
                                );
                            });
                        }
                        "finish" => {
                            let launch = val["launch"].as_bool().unwrap_or(true);
                            if launch {
                                let install_dir = get_install_directory();
                                let target_exe = install_dir.join(EXE_NAME);
                                if target_exe.exists() {
                                    let _ = Command::new(&target_exe).spawn();
                                }
                            }
                            let _ = proxy.send_event(InstallerEvent::Exit);
                        }
                        "start_uninstall"
                            if is_uninstall_mode && !is_installing.swap(true, Ordering::SeqCst) =>
                        {
                            let remove_data = val["remove_data"].as_bool().unwrap_or(false);
                            let worker_proxy = proxy.clone();
                            std::thread::spawn(move || {
                                run_uninstaller_worker(remove_data, worker_proxy);
                            });
                        }
                        _ => {}
                    }
                }
            });

        match webview_builder.build(&*window) {
            Ok(wv) => {
                if self.is_uninstall {
                    let _ = wv.evaluate_script(
                        "if (window.initUninstallMode) { window.initUninstallMode(); }",
                    );
                }
                self.webview = Some(wv);
                self.window = Some(window);
            }
            Err(e) => {
                eprintln!("Failed to initialize WebView2 installer shell: {}", e);
                event_loop.exit();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: InstallerEvent) {
        match event {
            InstallerEvent::Progress { pct, text } => {
                if let Some(wv) = &self.webview {
                    let script = format!("window.updateProgress({}, {:?});", pct, text);
                    let _ = wv.evaluate_script(&script);
                }
            }
            InstallerEvent::InstallComplete => {
                if let Some(wv) = &self.webview {
                    let _ = wv.evaluate_script("window.installComplete();");
                }
            }
            InstallerEvent::UninstallComplete => {
                if let Some(wv) = &self.webview {
                    let _ = wv.evaluate_script("window.uninstallComplete();");
                }
            }
            InstallerEvent::Exit => {
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }
    }
}

fn run_installer_worker(desktop: bool, start_menu: bool, proxy: EventLoopProxy<InstallerEvent>) {
    let install_dir = get_install_directory();

    // 1. Directory provisioning
    let _ = proxy.send_event(InstallerEvent::Progress {
        pct: 15,
        text: "Provisioning destination directory...".to_string(),
    });
    std::thread::sleep(std::time::Duration::from_millis(200));
    if let Err(e) = std::fs::create_dir_all(&install_dir) {
        let _ = proxy.send_event(InstallerEvent::Progress {
            pct: 20,
            text: format!("Error creating install directory: {}", e),
        });
        return;
    }

    // 2. Binary extraction
    let _ = proxy.send_event(InstallerEvent::Progress {
        pct: 45,
        text: "Extracting Evergreen Browser application binaries...".to_string(),
    });
    let target_exe = install_dir.join(EXE_NAME);
    if let Err(e) = std::fs::write(&target_exe, BROWSER_PAYLOAD) {
        let _ = proxy.send_event(InstallerEvent::Progress {
            pct: 50,
            text: format!("Error extracting binary payload: {}", e),
        });
        return;
    }

    // Deploy standalone uninstaller
    let uninstall_exe = install_dir.join("uninstall.exe");
    if let Ok(cur_exe) = std::env::current_exe() {
        let _ = std::fs::copy(&cur_exe, &uninstall_exe);
    }
    std::thread::sleep(std::time::Duration::from_millis(300));

    // 3. Shortcuts creation
    let _ = proxy.send_event(InstallerEvent::Progress {
        pct: 75,
        text: "Configuring shortcuts and shell integration...".to_string(),
    });
    if desktop {
        if let Some(desktop_lnk) = get_desktop_shortcut_path() {
            let _ = create_shortcut(&target_exe, &desktop_lnk, &target_exe);
        }
    }
    if start_menu {
        if let Some(start_lnk) = get_start_menu_shortcut_path() {
            let _ = create_shortcut(&target_exe, &start_lnk, &target_exe);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(250));

    // 4. Register uninstall entry
    let _ = proxy.send_event(InstallerEvent::Progress {
        pct: 90,
        text: "Registering application and finishing setup...".to_string(),
    });
    let _ = register_uninstall_entry(&install_dir, &uninstall_exe);
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 5. Complete
    let _ = proxy.send_event(InstallerEvent::Progress {
        pct: 100,
        text: "Installation complete!".to_string(),
    });
    let _ = proxy.send_event(InstallerEvent::InstallComplete);
}

fn run_uninstaller_worker(remove_data: bool, proxy: EventLoopProxy<InstallerEvent>) {
    let install_dir = get_install_directory();

    // 0. Terminate any running browser processes before file removal
    #[cfg(target_os = "windows")]
    {
        let mut kill_cmd = Command::new("taskkill");
        kill_cmd.args(["/F", "/IM", EXE_NAME]);
        kill_cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = kill_cmd.output();
    }

    // 1. Remove shortcuts
    if let Some(desktop_lnk) = get_desktop_shortcut_path() {
        let _ = std::fs::remove_file(desktop_lnk);
    }
    if let Some(start_lnk) = get_start_menu_shortcut_path() {
        let _ = std::fs::remove_file(start_lnk);
    }

    // 2. Remove registry entry
    unregister_uninstall_entry();

    // 3. Optionally remove user profile data
    if remove_data {
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let user_data_dir = PathBuf::from(local_appdata).join("EvergreenBrowser");
            let _ = std::fs::remove_dir_all(user_data_dir);
        }
    }

    // 4. Remove installation binaries
    let target_exe = install_dir.join(EXE_NAME);
    let _ = std::fs::remove_file(target_exe);

    // 5. Schedule deferred cleanup of install_dir to remove uninstall.exe after this process exits
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let cmd = format!(
            "timeout /t 2 /nobreak >nul & if exist \"{}\" rmdir /s /q \"{}\"",
            install_dir.display(),
            install_dir.display()
        );
        let _ = Command::new("cmd")
            .args(["/c", &cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::fs::remove_dir_all(&install_dir);
    }

    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = proxy.send_event(InstallerEvent::UninstallComplete);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cur_exe_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let is_uninstall =
        args.iter().any(|a| a == "--uninstall") || cur_exe_name.contains("uninstall");

    // Preflight WebView2 Runtime inspection
    if evergreen_core::env::detect_webview2_runtime().is_none() {
        #[cfg(target_os = "windows")]
        {
            use windows::core::w;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
            unsafe {
                let _ = MessageBoxW(
                    None,
                    w!("Evergreen Browser requires Microsoft Edge WebView2 Evergreen Runtime.\n\nPlease install it from: https://developer.microsoft.com/microsoft-edge/webview2/"),
                    w!("Evergreen Browser Setup"),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        eprintln!("Microsoft WebView2 Runtime is required but was not found.");
        std::process::exit(1);
    }

    let event_loop = EventLoop::<InstallerEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    let mut app = InstallerApp {
        is_uninstall,
        proxy,
        window: None,
        webview: None,
        is_installing: Arc::new(AtomicBool::new(false)),
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}
