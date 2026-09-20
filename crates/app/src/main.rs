//! Evergreen Browser — Main Application Entrypoint

mod chrome;
mod window;
mod accelerator;
mod settings_ui;
mod home_ui;

use chrome::{create_bounds, normalize_url, CHROME_HEIGHT, EMBEDDED_CHROME_HTML};
use evergreen_core::env::{detect_webview2_runtime, is_process_elevated};
use evergreen_core::ipc::{HostToUiMessage, UiToHostMessage};
use evergreen_core::tabs::{TabId, TabManager};
use evergreen_core::updater::run_local_command;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};
use wry::{WebView, WebViewBuilder};

#[derive(Debug)]
enum BrowserEvent {
    Ipc(UiToHostMessage),
    TabTitleChanged(TabId, String),
    TabNavigated(TabId, String),
    CommandDone(String, bool, String),
    Shortcut(String),
}

struct BrowserApp {
    window: Option<Arc<Window>>,
    proxy: EventLoopProxy<BrowserEvent>,
    tab_manager: TabManager,
    chrome_webview: Option<WebView>,
    tabs: HashMap<TabId, WebView>,
    runtime_version: String,
    modifiers: ModifiersState,
    window_width: f64,
    window_height: f64,
}

impl BrowserApp {
    fn new(proxy: EventLoopProxy<BrowserEvent>, runtime_version: String) -> Self {
        Self {
            window: None,
            proxy,
            tab_manager: TabManager::new(),
            chrome_webview: None,
            tabs: HashMap::new(),
            runtime_version,
            modifiers: ModifiersState::default(),
            window_width: 1280.0,
            window_height: 800.0,
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn sync_ui_state(&self) {
        if let Some(chrome) = &self.chrome_webview {
            let sync_msg = HostToUiMessage::TabStateSync {
                tabs: self.tab_manager.tabs().to_vec(),
                active_tab_id: self.tab_manager.active_tab().map(|t| t.id),
            };
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&sync_msg).unwrap_or_default()) {
                let script = format!("if (window.__shellUpdate) {{ window.__shellUpdate({}); }}", json);
                let _ = chrome.evaluate_script(&script);
            }
        }
    }

    fn create_tab_webview(&mut self, tab_id: TabId, url: &str) -> Result<WebView, wry::Error> {
        let window = self.window.as_ref().expect("Window must exist to create tab");
        let bounds = create_bounds(
            0.0,
            CHROME_HEIGHT,
            self.window_width,
            (self.window_height - CHROME_HEIGHT).max(1.0),
        );

        let proxy_title = self.proxy.clone();
        let proxy_nav = self.proxy.clone();
        let proxy_ipc = self.proxy.clone();

        let runtime_ver = self.runtime_version.clone();

        let webview = WebViewBuilder::new()
            .with_bounds(bounds)
            .with_custom_protocol("evergreen".into(), move |_webview_id, request| {
                let host = request.uri().host().unwrap_or("");
                let path = request.uri().path();
                if host == "newtab" || host == "home" || path == "/newtab" || path == "/home" || path == "newtab" || path == "home" {
                    wry::http::Response::builder()
                        .header("Content-Type", "text/html; charset=utf-8")
                        .body(std::borrow::Cow::Borrowed(home_ui::HOME_HTML.as_bytes()))
                        .unwrap()
                } else if host == "settings" || path == "/settings" || path == "settings" {
                    let html = settings_ui::SETTINGS_HTML.replace("Detecting...", &format!("v{} (Active)", runtime_ver));
                    wry::http::Response::builder()
                        .header("Content-Type", "text/html; charset=utf-8")
                        .body(std::borrow::Cow::Owned(html.into_bytes()))
                        .unwrap()
                } else {
                    wry::http::Response::builder()
                        .status(404)
                        .body(std::borrow::Cow::Borrowed(&b"Not Found"[..]))
                        .unwrap()
                }
            })
            .with_url(url)
            .with_incognito(true)
            .with_devtools(true)
            .with_document_title_changed_handler(move |title: String| {
                let _ = proxy_title.send_event(BrowserEvent::TabTitleChanged(tab_id, title));
            })
            .with_navigation_handler(move |nav_url: String| {
                let _ = proxy_nav.send_event(BrowserEvent::TabNavigated(tab_id, nav_url));
                true
            })
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                if let Ok(msg) = serde_json::from_str::<UiToHostMessage>(req.body()) {
                    let _ = proxy_ipc.send_event(BrowserEvent::Ipc(msg));
                }
            })
            .build_as_child(window.as_ref())?;

        accelerator::attach_accelerator_keys(&webview, self.proxy.clone());

        Ok(webview)
    }

    fn handle_create_tab(&mut self, url: Option<String>) {
        let target_url = url.unwrap_or_else(|| "evergreen://newtab".to_string());
        let tab_id = self.tab_manager.create_tab(&target_url, Self::now_secs());

        // Hide other tabs
        for (id, wv) in &self.tabs {
            if *id != tab_id {
                let _ = wv.set_visible(false);
            }
        }

        match self.create_tab_webview(tab_id, &target_url) {
            Ok(wv) => {
                let _ = wv.set_visible(true);
                self.tabs.insert(tab_id, wv);
                self.sync_ui_state();
            }
            Err(e) => eprintln!("Failed to create tab webview: {:?}", e),
        }
    }

    fn handle_open_settings(&mut self) {
        if let Some(existing) = self.tab_manager.tabs().iter().find(|t| t.url == "evergreen://settings") {
            let id = existing.id;
            self.handle_switch_tab(id);
        } else {
            self.handle_create_tab(Some("evergreen://settings".to_string()));
        }
    }

    fn handle_switch_tab(&mut self, target_id: TabId) {
        if self.tab_manager.switch_tab(target_id, Self::now_secs()) {
            for (id, wv) in &self.tabs {
                if *id == target_id {
                    let bounds = create_bounds(
                        0.0,
                        CHROME_HEIGHT,
                        self.window_width,
                        (self.window_height - CHROME_HEIGHT).max(1.0),
                    );
                    let _ = wv.set_bounds(bounds);
                    let _ = wv.set_visible(true);
                } else {
                    let _ = wv.set_visible(false);
                }
            }
            self.sync_ui_state();
        }
    }

    fn handle_close_tab(&mut self, target_id: TabId, event_loop: &ActiveEventLoop) {
        if let Some(wv) = self.tabs.remove(&target_id) {
            let _ = wv.set_visible(false);
            drop(wv);
        }

        match self.tab_manager.close_tab(target_id) {
            Some(next_active_id) => {
                self.handle_switch_tab(next_active_id);
            }
            None => {
                // Closing last tab closes window
                event_loop.exit();
            }
        }
    }

    fn handle_navigate(&mut self, raw_url: &str) {
        if let Some(active) = self.tab_manager.active_tab() {
            let active_id = active.id;
            match normalize_url(raw_url, "https://duckduckgo.com/?q=%s") {
                Ok(target) => {
                    self.tab_manager.update_url(active_id, target.clone());
                    if let Some(wv) = self.tabs.get(&active_id) {
                        let _ = wv.load_url(&target);
                    }
                    self.sync_ui_state();
                }
                Err(err) => eprintln!("Navigation error: {}", err),
            }
        }
    }
}

impl ApplicationHandler<BrowserEvent> for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = window::default_window_attributes();

            let window = match event_loop.create_window(window_attributes) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    eprintln!("Failed to create main window: {:?}", e);
                    return;
                }
            };
            self.window = Some(window.clone());

            // 1. Create Chrome Strip WebView
            let chrome_bounds = create_bounds(0.0, 0.0, self.window_width, CHROME_HEIGHT);
            let proxy_ipc = self.proxy.clone();

            match WebViewBuilder::new()
                .with_bounds(chrome_bounds)
                .with_html(EMBEDDED_CHROME_HTML)
                .with_ipc_handler(move |req: wry::http::Request<String>| {
                    if let Ok(msg) = serde_json::from_str::<UiToHostMessage>(req.body()) {
                        let _ = proxy_ipc.send_event(BrowserEvent::Ipc(msg));
                    }
                })
                .build_as_child(window.as_ref())
            {
                Ok(chrome_wv) => {
                    accelerator::attach_accelerator_keys(&chrome_wv, self.proxy.clone());
                    let version_script = format!(
                        "if (window.__syncEngineInfo) {{ window.__syncEngineInfo('{}'); }}",
                        self.runtime_version
                    );
                    let _ = chrome_wv.evaluate_script(&version_script);
                    self.chrome_webview = Some(chrome_wv);
                }
                Err(e) => eprintln!("Failed to create chrome webview: {:?}", e),
            }

            // 2. Create initial active tab with Fluent Home Screen
            self.handle_create_tab(None);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: BrowserEvent) {
        match event {
            BrowserEvent::Ipc(action) => match action {
                UiToHostMessage::ChromeReady => {
                    self.sync_ui_state();
                }
                UiToHostMessage::CreateTab { url } => {
                    self.handle_create_tab(url);
                }
                UiToHostMessage::OpenNewWindow => {
                    if let Ok(exe) = std::env::current_exe() {
                        let _ = std::process::Command::new(exe).spawn();
                    }
                }
                UiToHostMessage::SwitchTab { id } => {
                    self.handle_switch_tab(id);
                }
                UiToHostMessage::CloseTab { id } => {
                    self.handle_close_tab(id, event_loop);
                }
                UiToHostMessage::Navigate { url } => {
                    self.handle_navigate(&url);
                }
                UiToHostMessage::GoBack => {
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            let _ = wv.go_back();
                        }
                    }
                }
                UiToHostMessage::GoForward => {
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            let _ = wv.go_forward();
                        }
                    }
                }
                UiToHostMessage::Reload => {
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            let _ = wv.reload();
                        }
                    }
                }
                UiToHostMessage::Stop => {
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            let _ = wv.evaluate_script("window.stop()");
                        }
                    }
                }
                UiToHostMessage::OpenDevTools => {
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            wv.open_devtools();
                        }
                    }
                }
                UiToHostMessage::OpenSettings => {
                    self.handle_open_settings();
                }
                UiToHostMessage::SaveSettings { .. } => {}
                UiToHostMessage::RunEngineUpdate => {
                    let proxy = self.proxy.clone();
                    std::thread::spawn(move || {
                        let cmd = "powershell -NoProfile -Command \"$u='https://go.microsoft.com/fwlink/p/?LinkId=2124703'; $o=\\\"$env:TEMP\\MicrosoftEdgeWebview2Setup.exe\\\"; Invoke-WebRequest -Uri $u -OutFile $o; Start-Process -FilePath $o -ArgumentList '/silent','/install' -Wait\"";
                        match run_local_command(cmd) {
                            Ok(res) => {
                                let _ = proxy.send_event(BrowserEvent::CommandDone("Engine Update".to_string(), res.success, res.stdout));
                            }
                            Err(e) => {
                                let _ = proxy.send_event(BrowserEvent::CommandDone("Engine Update".to_string(), false, e.to_string()));
                            }
                        }
                    });
                }
                UiToHostMessage::RunForkUpdate => {
                    let proxy = self.proxy.clone();
                    std::thread::spawn(move || {
                        let cmd = "git pull && cargo build --release";
                        match run_local_command(cmd) {
                            Ok(res) => {
                                let _ = proxy.send_event(BrowserEvent::CommandDone("Fork Update".to_string(), res.success, res.stdout));
                            }
                            Err(e) => {
                                let _ = proxy.send_event(BrowserEvent::CommandDone("Fork Update".to_string(), false, e.to_string()));
                            }
                        }
                    });
                }
            },
            BrowserEvent::TabTitleChanged(tab_id, title) => {
                self.tab_manager.update_title(tab_id, title);
                self.sync_ui_state();
            }
            BrowserEvent::TabNavigated(tab_id, url) => {
                self.tab_manager.update_url(tab_id, url);
                self.sync_ui_state();
            }
            BrowserEvent::CommandDone(name, success, output) => {
                println!("[COMMAND RESULT] {}: success={} (output: {})", name, success, output.trim());
            }
            BrowserEvent::Shortcut(shortcut) => {
                match shortcut.as_str() {
                    "Ctrl+T" => self.handle_create_tab(None),
                    "Ctrl+N" => {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).spawn();
                        }
                    }
                    "Ctrl+W" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            let id = active.id;
                            self.handle_close_tab(id, event_loop);
                        }
                    }
                    "Ctrl+L" => {
                        if let Some(chrome) = &self.chrome_webview {
                            let _ = chrome.evaluate_script("if (window.__focusOmnibox) { window.__focusOmnibox(); }");
                        }
                    }
                    "Ctrl+R" | "F5" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.reload();
                            }
                        }
                    }
                    "Alt+Left" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.go_back();
                            }
                        }
                    }
                    "Alt+Right" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.go_forward();
                            }
                        }
                    }
                    "Ctrl+Tab" => {
                        let tabs = self.tab_manager.tabs();
                        if tabs.len() > 1 {
                            let current_idx = tabs.iter().position(|t| Some(t.id) == self.tab_manager.active_tab().map(|a| a.id)).unwrap_or(0);
                            let next_idx = (current_idx + 1) % tabs.len();
                            let next_id = tabs[next_idx].id;
                            self.handle_switch_tab(next_id);
                        }
                    }
                    "Ctrl+Shift+Tab" => {
                        let tabs = self.tab_manager.tabs();
                        if tabs.len() > 1 {
                            let current_idx = tabs.iter().position(|t| Some(t.id) == self.tab_manager.active_tab().map(|a| a.id)).unwrap_or(0);
                            let prev_idx = if current_idx == 0 { tabs.len() - 1 } else { current_idx - 1 };
                            let prev_id = tabs[prev_idx].id;
                            self.handle_switch_tab(prev_id);
                        }
                    }
                    "F12" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                wv.open_devtools();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(window) = &self.window {
                    let scale = window.scale_factor();
                    let logical = physical_size.to_logical::<f64>(scale);
                    self.window_width = logical.width;
                    self.window_height = logical.height;

                    // Resize chrome strip
                    if let Some(chrome) = &self.chrome_webview {
                        let chrome_bounds = create_bounds(0.0, 0.0, self.window_width, CHROME_HEIGHT);
                        let _ = chrome.set_bounds(chrome_bounds);
                    }

                    // Resize active tab webview
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            let content_bounds = create_bounds(
                                0.0,
                                CHROME_HEIGHT,
                                self.window_width,
                                (self.window_height - CHROME_HEIGHT).max(1.0),
                            );
                            let _ = wv.set_bounds(content_bounds);
                        }
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                match event.logical_key {
                    Key::Named(NamedKey::F12) => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                wv.open_devtools();
                            }
                        }
                    }
                    Key::Character(ref s) if s.eq_ignore_ascii_case("t") && self.modifiers.control_key() => {
                        self.handle_create_tab(None);
                    }
                    Key::Character(ref s) if s.eq_ignore_ascii_case("n") && self.modifiers.control_key() => {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).spawn();
                        }
                    }
                    Key::Character(ref s) if s.eq_ignore_ascii_case("w") && self.modifiers.control_key() => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            let id = active.id;
                            self.handle_close_tab(id, event_loop);
                        }
                    }
                    Key::Character(ref s) if s.eq_ignore_ascii_case("l") && self.modifiers.control_key() => {
                        if let Some(chrome) = &self.chrome_webview {
                            let _ = chrome.evaluate_script("if (window.__focusOmnibox) { window.__focusOmnibox(); }");
                        }
                    }
                    Key::Character(ref s) if s.eq_ignore_ascii_case("r") && self.modifiers.control_key() => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.reload();
                            }
                        }
                    }
                    Key::Named(NamedKey::F5) => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.reload();
                            }
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) if self.modifiers.alt_key() => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.go_back();
                            }
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) if self.modifiers.alt_key() => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.go_forward();
                            }
                        }
                    }
                    Key::Named(NamedKey::Tab) if self.modifiers.control_key() => {
                        let tabs = self.tab_manager.tabs();
                        if tabs.len() > 1 {
                            let current_idx = tabs.iter().position(|t| Some(t.id) == self.tab_manager.active_tab().map(|a| a.id)).unwrap_or(0);
                            if self.modifiers.shift_key() {
                                let prev_idx = if current_idx == 0 { tabs.len() - 1 } else { current_idx - 1 };
                                let prev_id = tabs[prev_idx].id;
                                self.handle_switch_tab(prev_id);
                            } else {
                                let next_idx = (current_idx + 1) % tabs.len();
                                let next_id = tabs[next_idx].id;
                                self.handle_switch_tab(next_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Elevation check: enforce non-elevated running invariant
    if is_process_elevated() {
        eprintln!("SECURITY ERROR: Running as Administrator / elevated is strictly prohibited.");
        std::process::exit(1);
    }

    // 2. Preflight runtime check
    let runtime_version = match detect_webview2_runtime() {
        Some(v) => v,
        None => {
            eprintln!("Microsoft WebView2 Runtime is required but was not found.");
            std::process::exit(1);
        }
    };
    println!("Detected Evergreen WebView2 Runtime: {}", runtime_version);

    // 3. Launch event loop
    let event_loop = EventLoop::<BrowserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = BrowserApp::new(proxy, runtime_version);
    event_loop.run_app(&mut app)?;

    Ok(())
}
