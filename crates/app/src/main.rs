//! Evergreen Browser — Main Application Entrypoint

mod chrome;
mod window;
mod accelerator;
mod settings_ui;
mod home_ui;
mod sidebar_ui;
mod cert;

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

const NAV_WATCHER_SCRIPT: &str = r#"
(() => {
  let lastUrl = location.href;
  let lastTitle = document.title;
  function notifyNav() {
    const curUrl = location.href;
    const curTitle = document.title;
    if (curUrl !== lastUrl || (curTitle && curTitle !== lastTitle)) {
      lastUrl = curUrl;
      lastTitle = curTitle;
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({
          action: 'PageNavigated',
          payload: { url: curUrl, title: curTitle }
        }));
      }
    }
  }
  window.addEventListener('popstate', () => { setTimeout(notifyNav, 0); setTimeout(notifyNav, 50); });
  window.addEventListener('pageshow', () => { setTimeout(notifyNav, 0); setTimeout(notifyNav, 50); });
  window.addEventListener('hashchange', () => { setTimeout(notifyNav, 0); });
  const origPush = history.pushState;
  history.pushState = function() {
    origPush.apply(this, arguments);
    setTimeout(notifyNav, 0);
  };
  const origReplace = history.replaceState;
  history.replaceState = function() {
    origReplace.apply(this, arguments);
    setTimeout(notifyNav, 0);
  };
  if (window.MutationObserver) {
    const titleEl = document.querySelector('title');
    if (titleEl) {
      new MutationObserver(() => notifyNav()).observe(titleEl, { childList: true, characterData: true, subtree: true });
    }
  }
})();
"#;

#[derive(Debug)]
enum BrowserEvent {
    Ipc(UiToHostMessage),
    TabTitleChanged(TabId, String),
    TabNavigated(TabId, String),
    HistoryChanged(TabId, bool, bool),
    CommandDone(String, bool, String),
    Shortcut(String),
}

struct BrowserApp {
    window: Option<Arc<Window>>,
    proxy: EventLoopProxy<BrowserEvent>,
    tab_manager: TabManager,
    chrome_webview: Option<WebView>,
    sidebar_webview: Option<WebView>,
    is_sidebar_open: bool,
    sidebar_mode: String,
    cert_cache: cert::CertificateCache,
    tabs: HashMap<TabId, WebView>,
    runtime_version: String,
    modifiers: ModifiersState,
    window_width: f64,
    window_height: f64,
    settings: evergreen_core::settings::Settings,
}

fn get_settings_path() -> std::path::PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        std::path::PathBuf::from(appdata).join("evergreen-browser").join("settings.json")
    } else {
        std::path::PathBuf::from("settings.json")
    }
}

impl BrowserApp {
    fn new(proxy: EventLoopProxy<BrowserEvent>, runtime_version: String) -> Self {
        let settings_path = get_settings_path();
        let settings = evergreen_core::settings::Settings::load_from_path(&settings_path).unwrap_or_default();
        Self {
            window: None,
            proxy,
            tab_manager: TabManager::new(),
            chrome_webview: None,
            sidebar_webview: None,
            is_sidebar_open: false,
            sidebar_mode: "menu".to_string(),
            cert_cache: cert::CertificateCache::new(),
            tabs: HashMap::new(),
            runtime_version,
            modifiers: ModifiersState::default(),
            window_width: 1280.0,
            window_height: 800.0,
            settings,
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn handle_focus_omnibox(&self) {
        if let Some(chrome) = &self.chrome_webview {
            #[cfg(target_os = "windows")]
            {
                use wry::WebViewExtWindows;
                use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC;
                let _ = unsafe { chrome.controller().MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC) };
            }
            let _ = chrome.evaluate_script("if (window.__focusOmnibox) { window.__focusOmnibox(); }");
        }
    }

    fn sync_ui_state(&self) {
        if let Some(chrome) = &self.chrome_webview {
            let sync_msg = HostToUiMessage::TabStateSync {
                tabs: self.tab_manager.tabs().to_vec(),
                active_tab_id: self.tab_manager.active_tab().map(|t| t.id),
            };
            if let Ok(json_str) = serde_json::to_string(&sync_msg) {
                let script = format!("if (window.__shellUpdate) {{ window.__shellUpdate({}); }}", json_str);
                let _ = chrome.evaluate_script(&script);
            }
        }
    }

    fn ensure_sidebar_webview(&mut self) -> Result<(), wry::Error> {
        if self.sidebar_webview.is_none() {
            let window = self.window.as_ref().expect("Window must exist for sidebar");
            let content_width = (self.window_width - 320.0).max(1.0);
            let sidebar_bounds = create_bounds(
                content_width,
                CHROME_HEIGHT,
                320.0,
                (self.window_height - CHROME_HEIGHT).max(1.0),
            );
            let proxy_ipc = self.proxy.clone();
            let wv = WebViewBuilder::new()
                .with_bounds(sidebar_bounds)
                .with_html(sidebar_ui::SIDEBAR_HTML.as_str())
                .with_transparent(true)
                .with_background_color((30, 30, 38, 255))
                .with_devtools(true)
                .with_ipc_handler(move |req: wry::http::Request<String>| {
                    if let Ok(msg) = serde_json::from_str::<UiToHostMessage>(req.body()) {
                        let _ = proxy_ipc.send_event(BrowserEvent::Ipc(msg));
                    }
                })
                .build_as_child(window.as_ref())?;

            accelerator::attach_accelerator_keys(&wv, self.proxy.clone());
            self.sidebar_webview = Some(wv);
        }
        Ok(())
    }

    fn update_sidebar_sync(&self) {
        if let Some(sidebar) = &self.sidebar_webview {
            let active = self.tab_manager.active_tab();
            let url = active.map(|t| t.url.clone()).unwrap_or_default();
            let sec_info = self.cert_cache.query_or_default(&url);

            let sync_msg = HostToUiMessage::SidebarStateSync {
                open: self.is_sidebar_open,
                mode: self.sidebar_mode.clone(),
                security_info: Some(Box::new(sec_info)),
            };

            if let Ok(json_str) = serde_json::to_string(&sync_msg) {
                let script = format!("if (window.__sidebarSync) {{ window.__sidebarSync({}); }}", json_str);
                let _ = sidebar.evaluate_script(&script);
            }
        }
    }

    fn open_sidebar(&mut self, mode: &str) {
        self.is_sidebar_open = true;
        self.sidebar_mode = mode.to_string();

        let _ = self.ensure_sidebar_webview();

        let content_width = (self.window_width - 320.0).max(1.0);
        if let Some(sidebar) = &self.sidebar_webview {
            let sidebar_bounds = create_bounds(
                content_width,
                CHROME_HEIGHT,
                320.0,
                (self.window_height - CHROME_HEIGHT).max(1.0),
            );
            let _ = sidebar.set_bounds(sidebar_bounds);
            let _ = sidebar.set_visible(true);
        }

        // Resize active tab webview
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(wv) = self.tabs.get(&active.id) {
                let bounds = create_bounds(0.0, CHROME_HEIGHT, content_width, (self.window_height - CHROME_HEIGHT).max(1.0));
                let _ = wv.set_bounds(bounds);
            }
        }

        self.update_sidebar_sync();
    }

    fn close_sidebar(&mut self) {
        self.is_sidebar_open = false;
        if let Some(sidebar) = &self.sidebar_webview {
            let _ = sidebar.set_visible(false);
        }

        // Restore active tab webview to full width
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(wv) = self.tabs.get(&active.id) {
                let bounds = create_bounds(0.0, CHROME_HEIGHT, self.window_width, (self.window_height - CHROME_HEIGHT).max(1.0));
                let _ = wv.set_bounds(bounds);
            }
        }
    }

    fn toggle_sidebar(&mut self, mode: &str) {
        if self.is_sidebar_open && self.sidebar_mode == mode {
            self.close_sidebar();
        } else {
            self.open_sidebar(mode);
        }
    }

    fn create_tab_webview(&mut self, tab_id: TabId, url: &str) -> Result<WebView, wry::Error> {
        let window = self.window.as_ref().expect("Window must exist to create tab");
        let content_width = if self.is_sidebar_open {
            (self.window_width - 320.0).max(1.0)
        } else {
            self.window_width
        };
        let bounds = create_bounds(
            0.0,
            CHROME_HEIGHT,
            content_width,
            (self.window_height - CHROME_HEIGHT).max(1.0),
        );

        let proxy_title = self.proxy.clone();
        let proxy_nav = self.proxy.clone();
        let proxy_ipc = self.proxy.clone();

        let runtime_ver = self.runtime_version.clone();
        let is_newtab = url.starts_with("evergreen://newtab") || url == "about:blank";
        let is_settings = url.starts_with("evergreen://settings");

        let mut builder = WebViewBuilder::new()
            .with_bounds(bounds)
            .with_incognito(true)
            .with_transparent(true)
            .with_background_color((24, 24, 32, 255))
            .with_initialization_script(NAV_WATCHER_SCRIPT)
            .with_devtools(true);

        let search_name = self.settings.search_engine_display_name().to_string();
        let search_url = self.settings.search_url_template().to_string();
        let home_html_content = home_ui::get_home_html(&search_name, &search_url);
        let settings_html_content = settings_ui::get_settings_html(&runtime_ver, &self.settings.search_engine);
        let home_bytes = home_html_content.as_bytes().to_vec();
        let settings_bytes = settings_html_content.as_bytes().to_vec();

        // Custom protocol for internal links
        builder = builder.with_custom_protocol("evergreen".into(), move |_webview_id, request| {
            let host = request.uri().host().unwrap_or("");
            let path = request.uri().path();
            if host == "newtab" || host == "home" || path == "/newtab" || path == "/home" || path == "newtab" || path == "home" {
                wry::http::Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(std::borrow::Cow::Owned(home_bytes.clone()))
                    .unwrap()
            } else if host == "settings" || path == "/settings" || path == "settings" {
                wry::http::Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(std::borrow::Cow::Owned(settings_bytes.clone()))
                    .unwrap()
            } else {
                wry::http::Response::builder()
                    .status(404)
                    .body(std::borrow::Cow::Borrowed(&b"Not Found"[..]))
                    .unwrap()
            }
        });

        // Load internal pages directly via with_html for instantaneous rendering
        if is_newtab {
            builder = builder.with_html(home_html_content);
        } else if is_settings {
            builder = builder.with_html(settings_html_content);
        } else {
            builder = builder.with_url(url);
        }

        let proxy_nav_interceptor = self.proxy.clone();
        let webview = builder
            .with_document_title_changed_handler(move |title: String| {
                let _ = proxy_title.send_event(BrowserEvent::TabTitleChanged(tab_id, title));
            })
            .with_navigation_handler(move |nav_url: String| {
                // Ignore base64 data URLs produced by with_html
                if nav_url.starts_with("data:text/html") {
                    return true;
                }
                if nav_url.starts_with("evergreen://settings") {
                    let _ = proxy_nav_interceptor.send_event(BrowserEvent::Ipc(UiToHostMessage::OpenSettings));
                    return false;
                }
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
        accelerator::attach_navigation_events(&webview, tab_id, self.proxy.clone());

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
                self.update_sidebar_sync();
            }
            Err(e) => eprintln!("Failed to create tab webview: {:?}", e),
        }
    }

    fn handle_open_settings(&mut self) {
        if let Some(existing) = self.tab_manager.tabs().iter().find(|t| t.url.starts_with("evergreen://settings")) {
            let id = existing.id;
            self.handle_switch_tab(id);
        } else {
            self.handle_create_tab(Some("evergreen://settings".to_string()));
        }
    }

    fn handle_switch_tab(&mut self, target_id: TabId) {
        if self.tab_manager.switch_tab(target_id, Self::now_secs()) {
            let content_width = if self.is_sidebar_open {
                (self.window_width - 320.0).max(1.0)
            } else {
                self.window_width
            };
            for (id, wv) in &self.tabs {
                if *id == target_id {
                    let bounds = create_bounds(
                        0.0,
                        CHROME_HEIGHT,
                        content_width,
                        (self.window_height - CHROME_HEIGHT).max(1.0),
                    );
                    let _ = wv.set_bounds(bounds);
                    let _ = wv.set_visible(true);
                } else {
                    let _ = wv.set_visible(false);
                }
            }
            self.sync_ui_state();
            self.update_sidebar_sync();
        }
    }

    fn switch_to_tab_index(&mut self, idx: usize) {
        let tabs = self.tab_manager.tabs();
        if idx < tabs.len() {
            let id = tabs[idx].id;
            self.handle_switch_tab(id);
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
            let search_template = self.settings.search_url_template();
            match normalize_url(raw_url, search_template) {
                Ok(target) => {
                    if target.starts_with("evergreen://settings") {
                        self.handle_open_settings();
                        return;
                    }
                    self.tab_manager.update_url(active_id, target.clone());
                    if let Some(host) = extract_host(&target) {
                        self.tab_manager.update_favicon(active_id, Some(format!("https://icons.duckduckgo.com/ip3/{}.ico", host)));
                    }
                    if let Some(wv) = self.tabs.get(&active_id) {
                        let _ = wv.load_url(&target);
                    }
                    self.sync_ui_state();
                }
                Err(err) => eprintln!("Navigation error: {}", err),
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn show_native_menu(&mut self, logical_x: f64, logical_y: f64) {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreatePopupMenu, AppendMenuW, TrackPopupMenuEx, DestroyMenu,
            MF_STRING, MF_SEPARATOR, TPM_RIGHTALIGN, TPM_TOPALIGN, TPM_RETURNCMD,
        };
        use windows::Win32::Foundation::{HWND, POINT};
        use windows::core::w;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        if let Some(window) = &self.window {
            if let Ok(handle) = window.window_handle() {
                if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                    let hwnd = HWND(win32_handle.hwnd.get() as _);
                    let scale = window.scale_factor();
                    let mut pt = POINT {
                        x: (logical_x * scale) as i32,
                        y: (logical_y * scale) as i32,
                    };
                    unsafe {
                        use windows::Win32::Graphics::Gdi::ClientToScreen;
                        let _ = ClientToScreen(hwnd, &mut pt);

                        if let Ok(hmenu) = CreatePopupMenu() {
                            let _ = AppendMenuW(hmenu, MF_STRING, 1001, w!("New Tab"));
                            let _ = AppendMenuW(hmenu, MF_STRING, 1002, w!("New Window"));
                            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
                            let _ = AppendMenuW(hmenu, MF_STRING, 1003, w!("DevTools"));
                            let _ = AppendMenuW(hmenu, MF_STRING, 1004, w!("Settings"));

                            let cmd = TrackPopupMenuEx(
                                hmenu,
                                (TPM_RIGHTALIGN | TPM_TOPALIGN | TPM_RETURNCMD).0,
                                pt.x,
                                pt.y,
                                hwnd,
                                None,
                            );
                            let _ = DestroyMenu(hmenu);

                            match cmd.0 {
                                1001 => self.handle_create_tab(None),
                                1002 => {
                                    if let Ok(exe) = std::env::current_exe() {
                                        let _ = std::process::Command::new(exe).spawn();
                                    }
                                }
                                1003 => {
                                    if let Some(active) = self.tab_manager.active_tab() {
                                        if let Some(wv) = self.tabs.get(&active.id) {
                                            wv.open_devtools();
                                        }
                                    }
                                }
                                1004 => self.handle_open_settings(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn show_native_menu(&mut self, _x: f64, _y: f64) {}
}

fn extract_host(url: &str) -> Option<String> {
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = after_scheme.split('/').next()?.split(':').next()?;
    if host.contains('.') {
        Some(host.to_string())
    } else {
        None
    }
}

impl ApplicationHandler<BrowserEvent> for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let mut window_attributes = window::default_window_attributes();
            window_attributes = window_attributes.with_visible(false);

            let window = match event_loop.create_window(window_attributes) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    eprintln!("Failed to create main window: {:?}", e);
                    return;
                }
            };
            self.window = Some(window.clone());

            let scale = window.scale_factor();
            let physical_size = window.inner_size();
            let logical = physical_size.to_logical::<f64>(scale);
            if logical.width > 0.0 && logical.height > 0.0 {
                self.window_width = logical.width;
                self.window_height = logical.height;
            }

            #[cfg(target_os = "windows")]
            {
                use windows::Win32::Graphics::Gdi::CreateSolidBrush;
                use windows::Win32::UI::WindowsAndMessaging::{SetClassLongPtrW, GCLP_HBRBACKGROUND};
                use windows::Win32::Foundation::{COLORREF, HWND};
                use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

                if let Ok(handle) = window.window_handle() {
                    if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                        let hwnd = HWND(win32_handle.hwnd.get() as _);
                        let dark_brush = unsafe { CreateSolidBrush(COLORREF(0x00201818)) }; // RGB(24, 24, 32)
                        let _ = unsafe { SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, dark_brush.0 as isize) };
                    }
                }
            }

            // 1. Create Chrome Strip WebView
            let chrome_bounds = create_bounds(0.0, 0.0, self.window_width, CHROME_HEIGHT);
            let proxy_ipc = self.proxy.clone();

            match WebViewBuilder::new()
                .with_bounds(chrome_bounds)
                .with_html(EMBEDDED_CHROME_HTML.as_str())
                .with_transparent(true)
                .with_background_color((24, 24, 32, 255))
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

            // 3. Make window visible now that webviews are initialized (eliminates white flashing)
            window.set_visible(true);
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
                UiToHostMessage::MenuToggled { open } => {
                    if let Some(chrome) = &self.chrome_webview {
                        let height = if open { 260.0 } else { CHROME_HEIGHT };
                        let chrome_bounds = create_bounds(0.0, 0.0, self.window_width, height);
                        let _ = chrome.set_bounds(chrome_bounds);
                    }
                }
                UiToHostMessage::ToggleMenuPanel => {
                    self.toggle_sidebar("menu");
                }
                UiToHostMessage::ToggleSecurityPanel => {
                    self.toggle_sidebar("security");
                }
                UiToHostMessage::CloseSidebar => {
                    self.close_sidebar();
                }
                UiToHostMessage::OpenCertificateDialog { host } => {
                    cert::open_native_certificate_dialog(&host);
                }
                UiToHostMessage::PageNavigated { url, title } => {
                    if let Some(active) = self.tab_manager.active_tab() {
                        let active_id = active.id;
                        if !url.starts_with("data:text/html") {
                            self.tab_manager.update_url(active_id, url.clone());
                            if let Some(host) = extract_host(&url) {
                                self.tab_manager.update_favicon(active_id, Some(format!("https://icons.duckduckgo.com/ip3/{}.ico", host)));
                            }
                        }
                        if !title.is_empty() {
                            self.tab_manager.update_title(active_id, title);
                        }
                        self.sync_ui_state();
                        self.update_sidebar_sync();
                    }
                }
                UiToHostMessage::OpenDevTools => {
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            wv.open_devtools();
                        }
                    }
                }
                UiToHostMessage::OpenMenu { x, y } => {
                    self.show_native_menu(x, y);
                }
                UiToHostMessage::OpenSettings => {
                    self.handle_open_settings();
                }
                UiToHostMessage::SetSearchEngine { engine } => {
                    self.settings.search_engine = engine.clone();
                    let path = get_settings_path();
                    let _ = self.settings.save_to_path(&path);
                    if let Some(chrome) = &self.chrome_webview {
                        let _ = chrome.evaluate_script(&format!(
                            "if (window.__syncSearchEngine) {{ window.__syncSearchEngine('{}'); }}",
                            engine
                        ));
                    }
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
                self.update_sidebar_sync();
            }
            BrowserEvent::TabNavigated(tab_id, url) => {
                if url.starts_with("data:text/html") {
                    return;
                }
                self.tab_manager.update_url(tab_id, url.clone());
                if let Some(host) = extract_host(&url) {
                    self.tab_manager.update_favicon(tab_id, Some(format!("https://icons.duckduckgo.com/ip3/{}.ico", host)));
                }
                self.sync_ui_state();
                self.update_sidebar_sync();
            }
            BrowserEvent::HistoryChanged(tab_id, can_back, can_forward) => {
                self.tab_manager.update_history_state(tab_id, can_back, can_forward);
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
                    "Ctrl+L" | "Ctrl+E" | "Ctrl+K" | "Ctrl+F" => {
                        self.handle_focus_omnibox();
                    }
                    "Ctrl+P" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("window.print()");
                            }
                        }
                    }
                    "Ctrl+Shift+R" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("location.reload(true)");
                            }
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
                    "Ctrl+1" => self.switch_to_tab_index(0),
                    "Ctrl+2" => self.switch_to_tab_index(1),
                    "Ctrl+3" => self.switch_to_tab_index(2),
                    "Ctrl+4" => self.switch_to_tab_index(3),
                    "Ctrl+5" => self.switch_to_tab_index(4),
                    "Ctrl+6" => self.switch_to_tab_index(5),
                    "Ctrl+7" => self.switch_to_tab_index(6),
                    "Ctrl+8" => self.switch_to_tab_index(7),
                    "Ctrl+9" => {
                        let tabs = self.tab_manager.tabs();
                        if !tabs.is_empty() {
                            self.handle_switch_tab(tabs.last().unwrap().id);
                        }
                    }
                    "Ctrl+Plus" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                #[cfg(target_os = "windows")]
                                {
                                    use wry::WebViewExtWindows;
                                    unsafe {
                                        let controller = wv.controller();
                                        let mut factor = 1.0;
                                        if controller.ZoomFactor(&mut factor).is_ok() {
                                            let _ = controller.SetZoomFactor((factor + 0.1).min(3.0));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "Ctrl+Minus" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                #[cfg(target_os = "windows")]
                                {
                                    use wry::WebViewExtWindows;
                                    unsafe {
                                        let controller = wv.controller();
                                        let mut factor = 1.0;
                                        if controller.ZoomFactor(&mut factor).is_ok() {
                                            let _ = controller.SetZoomFactor((factor - 0.1).max(0.25));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "Ctrl+Zero" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                #[cfg(target_os = "windows")]
                                {
                                    use wry::WebViewExtWindows;
                                    unsafe {
                                        let controller = wv.controller();
                                        let _ = controller.SetZoomFactor(1.0);
                                    }
                                }
                            }
                        }
                    }
                    "Escape" => {
                        if self.sidebar_webview.is_some() {
                            self.sidebar_webview = None;
                            if let Some(c) = &self.chrome_webview {
                                let _ = c.evaluate_script("if (window.__syncSidebarState) window.__syncSidebarState(null);");
                            }
                        } else if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("window.stop()");
                            }
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

                    let content_width = if self.is_sidebar_open {
                        (self.window_width - 320.0).max(1.0)
                    } else {
                        self.window_width
                    };

                    // Resize active tab webview
                    if let Some(active) = self.tab_manager.active_tab() {
                        if let Some(wv) = self.tabs.get(&active.id) {
                            let content_bounds = create_bounds(
                                0.0,
                                CHROME_HEIGHT,
                                content_width,
                                (self.window_height - CHROME_HEIGHT).max(1.0),
                            );
                            let _ = wv.set_bounds(content_bounds);
                        }
                    }

                    // Resize sidebar webview if open
                    if self.is_sidebar_open {
                        if let Some(sidebar) = &self.sidebar_webview {
                            let sidebar_bounds = create_bounds(
                                content_width,
                                CHROME_HEIGHT,
                                320.0,
                                (self.window_height - CHROME_HEIGHT).max(1.0),
                            );
                            let _ = sidebar.set_bounds(sidebar_bounds);
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
                    Key::Character(ref s) if (s.eq_ignore_ascii_case("l") || s.eq_ignore_ascii_case("e") || s.eq_ignore_ascii_case("k") || s.eq_ignore_ascii_case("f")) && self.modifiers.control_key() => {
                        self.handle_focus_omnibox();
                    }
                    Key::Character(ref s) if s.eq_ignore_ascii_case("p") && self.modifiers.control_key() => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("window.print()");
                            }
                        }
                    }
                    Key::Character(ref s) if s.eq_ignore_ascii_case("r") && self.modifiers.control_key() => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                if self.modifiers.shift_key() {
                                    let _ = wv.evaluate_script("location.reload(true)");
                                } else {
                                    let _ = wv.reload();
                                }
                            }
                        }
                    }
                    Key::Named(NamedKey::F5) => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                if self.modifiers.control_key() || self.modifiers.shift_key() {
                                    let _ = wv.evaluate_script("location.reload(true)");
                                } else {
                                    let _ = wv.reload();
                                }
                            }
                        }
                    }
                    Key::Named(NamedKey::Escape) => {
                        if self.sidebar_webview.is_some() {
                            self.sidebar_webview = None;
                            if let Some(c) = &self.chrome_webview {
                                let _ = c.evaluate_script("if (window.__syncSidebarState) window.__syncSidebarState(null);");
                            }
                        } else if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("window.stop()");
                            }
                        }
                    }
                    Key::Character(ref s) if self.modifiers.control_key() && s.len() == 1 && s.chars().next().is_some_and(|c| c.is_ascii_digit()) => {
                        let digit = s.chars().next().unwrap().to_digit(10).unwrap();
                        if (1..=8).contains(&digit) {
                            self.switch_to_tab_index((digit - 1) as usize);
                        } else if digit == 9 {
                            let tabs = self.tab_manager.tabs();
                            if !tabs.is_empty() {
                                self.handle_switch_tab(tabs.last().unwrap().id);
                            }
                        }
                    }
                    Key::Character(ref s) if self.modifiers.control_key() && (s == "+" || s == "=") => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                #[cfg(target_os = "windows")]
                                {
                                    use wry::WebViewExtWindows;
                                    unsafe {
                                        let controller = wv.controller();
                                        let mut factor = 1.0;
                                        if controller.ZoomFactor(&mut factor).is_ok() {
                                            let _ = controller.SetZoomFactor((factor + 0.1).min(3.0));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Key::Character(ref s) if self.modifiers.control_key() && (s == "-" || s == "_") => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                #[cfg(target_os = "windows")]
                                {
                                    use wry::WebViewExtWindows;
                                    unsafe {
                                        let controller = wv.controller();
                                        let mut factor = 1.0;
                                        if controller.ZoomFactor(&mut factor).is_ok() {
                                            let _ = controller.SetZoomFactor((factor - 0.1).max(0.25));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Key::Character(ref s) if self.modifiers.control_key() && s == "0" => {
                        if let Some(active) = self.tab_manager.active_tab() {
                            if let Some(wv) = self.tabs.get(&active.id) {
                                #[cfg(target_os = "windows")]
                                {
                                    use wry::WebViewExtWindows;
                                    unsafe {
                                        let controller = wv.controller();
                                        let _ = controller.SetZoomFactor(1.0);
                                    }
                                }
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
