#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Evergreen Browser — Main Application Entrypoint

mod accelerator;
mod cert;
mod chrome;
mod home_ui;
mod settings_ui;
mod sidebar_ui;
mod window;

use chrome::{create_bounds, get_chrome_html, normalize_url, CHROME_HEIGHT};
use evergreen_core::env::{detect_webview2_runtime, is_process_elevated};
use evergreen_core::ipc::{HostToUiMessage, UiToHostMessage};
use evergreen_core::tabs::{TabId, TabManager, TabState};
use evergreen_core::updater::run_local_command;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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

  // Bottom Status Link Preview Bubble
  let previewTooltip = null;
  function ensureTooltip() {
    if (!previewTooltip) {
      previewTooltip = document.createElement('div');
      previewTooltip.id = '__evergreen_status_bubble';
      previewTooltip.style.cssText = 'position:fixed;bottom:8px;left:10px;max-width:550px;padding:4px 10px;background:rgba(24,24,32,0.94);backdrop-filter:blur(8px);border:1px solid rgba(255,255,255,0.12);border-radius:6px;color:#cbd5e1;font-size:11px;font-family:system-ui,-apple-system,sans-serif;z-index:2147483647;pointer-events:none;display:none;box-shadow:0 4px 14px rgba(0,0,0,0.4);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;transition:opacity 0.1s ease;';
      document.documentElement.appendChild(previewTooltip);
    }
    return previewTooltip;
  }

  document.addEventListener('mouseover', (e) => {
    const a = e.target.closest('a');
    if (a && a.href) {
      const tip = ensureTooltip();
      tip.textContent = a.href;
      tip.style.display = 'block';
    }
  });

  document.addEventListener('mouseout', (e) => {
    const a = e.target.closest('a');
    if (a) {
      const tip = ensureTooltip();
      tip.style.display = 'none';
    }
  });
})();
"#;

#[derive(Debug)]
pub(crate) enum BrowserEvent {
    Ipc(WindowId, UiToHostMessage),
    TabTitleChanged(WindowId, TabId, String),
    TabNavigated(WindowId, TabId, String),
    HistoryChanged(WindowId, TabId, bool, bool),
    CommandDone(String, bool, String),
    Shortcut(WindowId, String),
    DownloadProgress {
        download_id: u64,
        filename: String,
        received_bytes: i64,
        total_bytes: i64,
        state: String,
    },
    PermissionPrompt {
        window_id: WindowId,
        permission_id: u64,
        origin: String,
        permission_kind: String,
    },
    TabCrashed(WindowId, TabId, i32),
    ServerCertificateError {
        window_id: WindowId,
        tab_id: TabId,
        request_uri: String,
        error_status: i32,
    },
}

struct WindowContext {
    id: WindowId,
    window: Arc<Window>,
    tab_manager: TabManager,
    chrome_webview: Option<WebView>,
    sidebar_webview: Option<WebView>,
    is_sidebar_open: bool,
    sidebar_mode: String,
    tabs: HashMap<TabId, WebView>,
    tab_persisted: HashMap<TabId, bool>,
    window_width: f64,
    window_height: f64,
    find_bar_open: bool,
    permission_bar_open: bool,
}

impl WindowContext {
    pub fn current_chrome_height(&self) -> f64 {
        if self.find_bar_open || self.permission_bar_open {
            114.0
        } else {
            CHROME_HEIGHT
        }
    }

    pub fn update_layout(&self) {
        let chrome_h = self.current_chrome_height();
        if let Some(chrome) = &self.chrome_webview {
            let chrome_bounds = create_bounds(0.0, 0.0, self.window_width, chrome_h);
            let _ = chrome.set_bounds(chrome_bounds);
        }

        let content_width = if self.is_sidebar_open {
            (self.window_width - 320.0).max(1.0)
        } else {
            self.window_width
        };

        let content_bounds = create_bounds(
            0.0,
            chrome_h,
            content_width,
            (self.window_height - chrome_h).max(1.0),
        );
        for wv in self.tabs.values() {
            let _ = wv.set_bounds(content_bounds);
        }

        if self.is_sidebar_open {
            if let Some(sidebar) = &self.sidebar_webview {
                let sidebar_bounds = create_bounds(
                    content_width,
                    chrome_h,
                    320.0,
                    (self.window_height - chrome_h).max(1.0),
                );
                let _ = sidebar.set_bounds(sidebar_bounds);
            }
        }
    }

    pub fn handle_focus_omnibox(&self) {
        if let Some(chrome) = &self.chrome_webview {
            #[cfg(target_os = "windows")]
            {
                use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC;
                use wry::WebViewExtWindows;
                let _ = unsafe {
                    chrome
                        .controller()
                        .MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC)
                };
            }
            let _ =
                chrome.evaluate_script("if (window.__focusOmnibox) { window.__focusOmnibox(); }");
        }
    }

    pub fn sync_ui_state(&self) {
        if let Some(chrome) = &self.chrome_webview {
            let sync_msg = HostToUiMessage::TabStateSync {
                tabs: self.tab_manager.tabs().to_vec(),
                active_tab_id: self.tab_manager.active_tab().map(|t| t.id),
            };
            if let Ok(json_str) = serde_json::to_string(&sync_msg) {
                let script = format!(
                    "if (window.__shellUpdate) {{ window.__shellUpdate({}); }}",
                    json_str
                );
                let _ = chrome.evaluate_script(&script);
            }
        }
    }

    pub fn sync_zoom(&self, factor: f64) {
        if let Some(chrome) = &self.chrome_webview {
            let _ = chrome.evaluate_script(&format!(
                "if (window.__syncZoom) {{ window.__syncZoom({}); }}",
                factor
            ));
        }
        if let Some(sidebar) = &self.sidebar_webview {
            let _ = sidebar.evaluate_script(&format!(
                "if (window.__syncZoom) {{ window.__syncZoom({}); }}",
                factor
            ));
        }
    }

    pub fn get_active_zoom(&self) -> f64 {
        #[cfg(target_os = "windows")]
        {
            if let Some(active) = self.tab_manager.active_tab() {
                if let Some(wv) = self.tabs.get(&active.id) {
                    use wry::WebViewExtWindows;
                    unsafe {
                        let controller = wv.controller();
                        let mut factor = 1.0;
                        if controller.ZoomFactor(&mut factor).is_ok() {
                            return factor;
                        }
                    }
                }
            }
        }
        1.0
    }

    pub fn set_active_zoom(&self, factor: f64) {
        let clamped = (factor * 100.0).round() / 100.0;
        let clamped = clamped.clamp(0.25, 3.0);
        #[cfg(target_os = "windows")]
        {
            if let Some(active) = self.tab_manager.active_tab() {
                if let Some(wv) = self.tabs.get(&active.id) {
                    use wry::WebViewExtWindows;
                    unsafe {
                        let controller = wv.controller();
                        let _ = controller.SetZoomFactor(clamped);
                    }
                }
            }
        }
        self.sync_zoom(clamped);
    }

    pub fn handle_find_in_page(&self, query: &str, forward: bool) {
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(wv) = self.tabs.get(&active.id) {
                let escaped_query = query
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
                    .replace('\n', " ");
                let script = format!(
                    r#"
                    (() => {{
                        const q = '{escaped_query}';
                        if (!q) return;
                        const found = window.find(q, false, !{forward}, true, false, false, false);
                        const bodyText = document.body ? document.body.innerText : '';
                        let count = 0;
                        if (bodyText && q) {{
                            const re = new RegExp(q.replace(/[.*+?^${{}}()|[\]\\]/g, '\\$&'), 'gi');
                            const matches = bodyText.match(re);
                            count = matches ? matches.length : 0;
                        }}
                        const current = (found && count > 0) ? 1 : (count > 0 ? 1 : 0);
                        if (window.ipc) {{
                            window.ipc.postMessage(JSON.stringify({{
                                action: 'FindResult',
                                payload: {{ current, total: count }}
                            }}));
                        }}
                    }})();
                    "#
                );
                let _ = wv.evaluate_script(&script);
            }
        }
    }

    pub fn handle_close_find_in_page(&self) {
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(wv) = self.tabs.get(&active.id) {
                let _ = wv.evaluate_script(
                    "if (window.getSelection) { window.getSelection().removeAllRanges(); }",
                );
            }
        }
    }

    pub fn ensure_sidebar_webview(
        &mut self,
        proxy: EventLoopProxy<BrowserEvent>,
    ) -> Result<(), wry::Error> {
        if self.sidebar_webview.is_none() {
            let window = self.window.clone();
            let chrome_h = self.current_chrome_height();
            let content_width = (self.window_width - 320.0).max(1.0);
            let sidebar_bounds = create_bounds(
                content_width,
                chrome_h,
                320.0,
                (self.window_height - chrome_h).max(1.0),
            );
            let proxy_ipc = proxy.clone();
            let win_id = self.id;
            let wv = WebViewBuilder::new()
                .with_bounds(sidebar_bounds)
                .with_html(sidebar_ui::SIDEBAR_HTML.as_str())
                .with_transparent(true)
                .with_background_color((30, 30, 38, 255))
                .with_devtools(true)
                .with_ipc_handler(move |req: wry::http::Request<String>| {
                    if let Ok(msg) = serde_json::from_str::<UiToHostMessage>(req.body()) {
                        let _ = proxy_ipc.send_event(BrowserEvent::Ipc(win_id, msg));
                    }
                })
                .build_as_child(window.as_ref())?;

            accelerator::attach_accelerator_keys(&wv, win_id, proxy);
            self.sidebar_webview = Some(wv);
        }
        Ok(())
    }

    pub fn update_sidebar_sync(&self, cert_cache: &cert::CertificateCache) {
        if let Some(sidebar) = &self.sidebar_webview {
            let active = self.tab_manager.active_tab();
            let url = active.map(|t| t.url.clone()).unwrap_or_default();
            let sec_info = cert_cache.query_or_default(&url);

            let sync_msg = HostToUiMessage::SidebarStateSync {
                open: self.is_sidebar_open,
                mode: self.sidebar_mode.clone(),
                security_info: Some(Box::new(sec_info)),
            };

            if let Ok(json_str) = serde_json::to_string(&sync_msg) {
                let script = format!(
                    "if (window.__sidebarSync) {{ window.__sidebarSync({}); }}",
                    json_str
                );
                let _ = sidebar.evaluate_script(&script);
            }
        }
    }

    pub fn open_sidebar(
        &mut self,
        mode: &str,
        cert_cache: &cert::CertificateCache,
        proxy: EventLoopProxy<BrowserEvent>,
    ) {
        self.is_sidebar_open = true;
        self.sidebar_mode = mode.to_string();

        let _ = self.ensure_sidebar_webview(proxy);
        self.update_layout();
        if let Some(sidebar) = &self.sidebar_webview {
            let _ = sidebar.set_visible(true);
        }
        self.update_sidebar_sync(cert_cache);
    }

    pub fn close_sidebar(&mut self) {
        self.is_sidebar_open = false;
        if let Some(sidebar) = &self.sidebar_webview {
            let _ = sidebar.set_visible(false);
        }
        self.update_layout();
    }

    pub fn toggle_sidebar(
        &mut self,
        mode: &str,
        cert_cache: &cert::CertificateCache,
        proxy: EventLoopProxy<BrowserEvent>,
    ) {
        if self.is_sidebar_open && self.sidebar_mode == mode {
            self.close_sidebar();
        } else {
            self.open_sidebar(mode, cert_cache, proxy);
        }
    }

    pub fn switch_to_tab_index(&mut self, idx: usize, cert_cache: &cert::CertificateCache) {
        let tabs = self.tab_manager.tabs();
        if idx < tabs.len() {
            let id = tabs[idx].id;
            self.handle_switch_tab(id, cert_cache);
        }
    }

    pub fn wake_active(&self) {
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(wv) = self.tabs.get(&active.id) {
                accelerator::wake_webview(wv);
            }
        }
        if let Some(chrome) = &self.chrome_webview {
            accelerator::wake_webview(chrome);
        }
        if self.is_sidebar_open {
            if let Some(sidebar) = &self.sidebar_webview {
                accelerator::wake_webview(sidebar);
            }
        }
    }

    pub fn handle_switch_tab(&mut self, target_id: TabId, cert_cache: &cert::CertificateCache) {
        if self
            .tab_manager
            .switch_tab(target_id, BrowserApp::now_secs())
        {
            for (id, wv) in &self.tabs {
                if *id == target_id {
                    let _ = wv.set_visible(true);
                    accelerator::wake_webview(wv);
                    accelerator::focus_webview(wv);
                } else {
                    let _ = wv.set_visible(false);
                }
            }
            self.update_layout();
            self.sync_ui_state();
            self.update_sidebar_sync(cert_cache);
            self.sync_zoom(self.get_active_zoom());
        }
    }
}

pub(crate) fn extract_host(url: &str) -> Option<String> {
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

fn get_data_directory() -> std::path::PathBuf {
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(&p).to_path_buf())
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let args: Vec<String> = std::env::args().collect();
    evergreen_core::env::resolve_data_directory(&exe_dir, &args)
}

fn get_settings_path() -> std::path::PathBuf {
    get_data_directory().join("settings.json")
}

struct BrowserApp {
    windows: HashMap<WindowId, WindowContext>,
    proxy: EventLoopProxy<BrowserEvent>,
    cert_cache: cert::CertificateCache,
    runtime_version: String,
    modifiers: ModifiersState,
    settings: evergreen_core::settings::Settings,
    plugins: evergreen_core::plugins::PluginRegistry,
    next_tab_id: AtomicU64,
    allowed_cert_hosts: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl BrowserApp {
    fn new(proxy: EventLoopProxy<BrowserEvent>, runtime_version: String) -> Self {
        let settings_path = get_settings_path();
        let settings =
            evergreen_core::settings::Settings::load_from_path(&settings_path).unwrap_or_default();
        let mut plugins = evergreen_core::plugins::PluginRegistry::new();
        plugins.register(evergreen_core::plugins::FeaturePlugin::new(
            evergreen_core::plugins::PluginMetadata {
                id: "find_in_page".to_string(),
                name: "Find in Page".to_string(),
                version: "1.0.0".to_string(),
                description: "Expandable in-page text search tray".to_string(),
                author: "Evergreen Team".to_string(),
                is_core: false,
                enabled_by_default: true,
            },
            |s| s.features.enable_find_in_page,
        ));
        plugins.register(evergreen_core::plugins::FeaturePlugin::new(
            evergreen_core::plugins::PluginMetadata {
                id: "downloads_manager".to_string(),
                name: "Downloads Manager".to_string(),
                version: "1.0.0".to_string(),
                description: "Toolbar progress ring and drawer manager".to_string(),
                author: "Evergreen Team".to_string(),
                is_core: false,
                enabled_by_default: true,
            },
            |s| s.features.enable_downloads_manager,
        ));
        plugins.register(evergreen_core::plugins::FeaturePlugin::new(
            evergreen_core::plugins::PluginMetadata {
                id: "link_preview".to_string(),
                name: "Link Destination Preview (Status Bubble)".to_string(),
                version: "1.0.0".to_string(),
                description: "Bottom-left destination URL capsule when hovering hyperlinks"
                    .to_string(),
                author: "Evergreen Team".to_string(),
                is_core: false,
                enabled_by_default: true,
            },
            |s| s.features.enable_link_preview,
        ));
        plugins.register(evergreen_core::plugins::FeaturePlugin::new(
            evergreen_core::plugins::PluginMetadata {
                id: "tab_gestures".to_string(),
                name: "Tab Gestures & Detach".to_string(),
                version: "1.0.0".to_string(),
                description: "Tab dragging, reordering, and multi-window migration".to_string(),
                author: "Evergreen Team".to_string(),
                is_core: false,
                enabled_by_default: true,
            },
            |s| s.features.enable_tab_gestures,
        ));
        plugins.register(evergreen_core::plugins::FeaturePlugin::new(
            evergreen_core::plugins::PluginMetadata {
                id: "zoom_controls".to_string(),
                name: "Zoom Controls & Badge".to_string(),
                version: "1.0.0".to_string(),
                description: "Custom zoom badge, reset dialog, and scale shortcuts".to_string(),
                author: "Evergreen Team".to_string(),
                is_core: false,
                enabled_by_default: true,
            },
            |s| s.features.enable_zoom_controls,
        ));
        plugins.register(evergreen_core::plugins::FeaturePlugin::new(
            evergreen_core::plugins::PluginMetadata {
                id: "permissions_prompt".to_string(),
                name: "Site Permissions Prompt".to_string(),
                version: "1.0.0".to_string(),
                description: "Async permission deferral and user consent bar".to_string(),
                author: "Evergreen Team".to_string(),
                is_core: false,
                enabled_by_default: true,
            },
            |s| s.features.enable_permissions_prompt,
        ));

        Self {
            windows: HashMap::new(),
            proxy,
            cert_cache: cert::CertificateCache::new(),
            runtime_version,
            modifiers: ModifiersState::default(),
            settings,
            plugins,
            next_tab_id: AtomicU64::new(1),
            allowed_cert_hosts: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }

    pub fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn next_tab_id(&self) -> TabId {
        TabId(self.next_tab_id.fetch_add(1, Ordering::SeqCst))
    }

    fn create_browser_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        initial_url: Option<String>,
    ) -> Option<WindowId> {
        let mut window_attributes = window::default_window_attributes();
        window_attributes = window_attributes.with_visible(false);

        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("Failed to create window: {:?}", e);
                return None;
            }
        };
        let win_id = window.id();

        let scale = window.scale_factor();
        let physical_size = window.inner_size();
        let logical = physical_size.to_logical::<f64>(scale);
        let win_width = if logical.width > 0.0 {
            logical.width
        } else {
            1280.0
        };
        let win_height = if logical.height > 0.0 {
            logical.height
        } else {
            800.0
        };

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::{COLORREF, HWND};
            use windows::Win32::Graphics::Gdi::CreateSolidBrush;
            use windows::Win32::UI::WindowsAndMessaging::{SetClassLongPtrW, GCLP_HBRBACKGROUND};
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

            #[link(name = "dwmapi")]
            extern "system" {
                fn DwmSetWindowAttribute(
                    hwnd: isize,
                    dwAttribute: u32,
                    pvAttribute: *const std::ffi::c_void,
                    cbAttribute: u32,
                ) -> i32;
            }

            if let Ok(handle) = window.window_handle() {
                if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                    let hwnd = HWND(win32_handle.hwnd.get() as _);
                    let dark_brush = unsafe { CreateSolidBrush(COLORREF(0x00201818)) }; // RGB(24, 24, 32)
                    let _ = unsafe {
                        SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, dark_brush.0 as isize)
                    };

                    let dark_mode: i32 = 1;
                    let _ = unsafe {
                        DwmSetWindowAttribute(hwnd.0 as _, 20, &dark_mode as *const _ as _, 4)
                    };
                    let caption_color: u32 = 0x00201818;
                    let _ = unsafe {
                        DwmSetWindowAttribute(hwnd.0 as _, 35, &caption_color as *const _ as _, 4)
                    };
                }
            }
        }

        let mut win_ctx = WindowContext {
            id: win_id,
            window: window.clone(),
            tab_manager: TabManager::new(),
            chrome_webview: None,
            sidebar_webview: None,
            is_sidebar_open: false,
            sidebar_mode: "menu".to_string(),
            tabs: HashMap::new(),
            tab_persisted: HashMap::new(),
            window_width: win_width,
            window_height: win_height,
            find_bar_open: false,
            permission_bar_open: false,
        };

        // Create Chrome Strip WebView
        let chrome_bounds = create_bounds(0.0, 0.0, win_width, CHROME_HEIGHT);
        let proxy_ipc = self.proxy.clone();

        let chrome_html = get_chrome_html(&self.settings.search_engine);

        match WebViewBuilder::new()
            .with_bounds(chrome_bounds)
            .with_html(&chrome_html)
            .with_transparent(true)
            .with_background_color((24, 24, 32, 255))
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                if let Ok(msg) = serde_json::from_str::<UiToHostMessage>(req.body()) {
                    let _ = proxy_ipc.send_event(BrowserEvent::Ipc(win_id, msg));
                }
            })
            .build_as_child(window.as_ref())
        {
            Ok(chrome_wv) => {
                accelerator::attach_accelerator_keys(&chrome_wv, win_id, self.proxy.clone());
                let version_script = format!(
                    "if (window.__syncEngineInfo) {{ window.__syncEngineInfo('{}'); }}",
                    self.runtime_version
                );
                let _ = chrome_wv.evaluate_script(&version_script);
                let engine_script = format!(
                    "if (window.__syncSearchEngine) {{ window.__syncSearchEngine('{}'); }}",
                    self.settings.search_engine
                );
                let _ = chrome_wv.evaluate_script(&engine_script);
                win_ctx.chrome_webview = Some(chrome_wv);
            }
            Err(e) => eprintln!("Failed to create chrome webview: {:?}", e),
        }

        // Sidebar webview is lazily instantiated on demand in open_sidebar()
        self.windows.insert(win_id, win_ctx);

        let target_url = initial_url.or_else(|| {
            std::env::args()
                .nth(1)
                .filter(|a| !a.is_empty() && !a.starts_with('-'))
        });
        self.handle_create_tab(win_id, target_url);

        window.set_visible(true);
        Some(win_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_tab_webview(
        &self,
        win_id: WindowId,
        tab_id: TabId,
        url: &str,
        window: &Window,
        win_width: f64,
        win_height: f64,
        is_sidebar_open: bool,
        chrome_h: f64,
    ) -> Result<WebView, wry::Error> {
        let content_width = if is_sidebar_open {
            (win_width - 320.0).max(1.0)
        } else {
            win_width
        };
        let bounds = create_bounds(
            0.0,
            chrome_h,
            content_width,
            (win_height - chrome_h).max(1.0),
        );

        let proxy_title = self.proxy.clone();
        let proxy_nav = self.proxy.clone();
        let proxy_ipc = self.proxy.clone();

        let runtime_ver = self.runtime_version.clone();
        let is_newtab = url.starts_with("evergreen://newtab") || url == "about:blank";
        let is_settings = url.starts_with("evergreen://settings");

        let plugin_scripts = self.plugins.combined_content_scripts(&self.settings);
        let init_script = if plugin_scripts.is_empty() {
            NAV_WATCHER_SCRIPT.to_string()
        } else {
            format!("{}\n{}", NAV_WATCHER_SCRIPT, plugin_scripts)
        };

        let is_persistent = self.settings.is_site_persistent(url);
        let settings_nav = self.settings.clone();

        let mut builder = WebViewBuilder::new()
            .with_bounds(bounds)
            .with_incognito(!is_persistent)
            .with_transparent(true)
            .with_background_color((24, 24, 32, 255))
            .with_initialization_script(&init_script)
            .with_devtools(true);

        let search_name = self.settings.search_engine_display_name().to_string();
        let search_url = self.settings.search_url_template().to_string();
        let home_html_content = home_ui::get_home_html(&search_name, &search_url);
        let home_bytes = home_html_content.as_bytes().to_vec();
        let fallback_runtime_ver = runtime_ver.clone();
        let settings_clone = self.settings.clone();

        // Custom protocol for internal links
        builder = builder.with_custom_protocol("evergreen".into(), move |_webview_id, request| {
            let host = request.uri().host().unwrap_or("");
            let path = request.uri().path();
            if host == "newtab"
                || host == "home"
                || path == "/newtab"
                || path == "/home"
                || path == "newtab"
                || path == "home"
            {
                wry::http::Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(std::borrow::Cow::Owned(home_bytes.clone()))
                    .unwrap()
            } else if host == "settings" || path == "/settings" || path == "settings" {
                let active_ver = evergreen_core::env::detect_webview2_runtime()
                    .unwrap_or_else(|| fallback_runtime_ver.clone());
                let dynamic_settings_html =
                    settings_ui::get_settings_html(&active_ver, &settings_clone);
                wry::http::Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(std::borrow::Cow::Owned(dynamic_settings_html.into_bytes()))
                    .unwrap()
            } else {
                wry::http::Response::builder()
                    .status(404)
                    .body(std::borrow::Cow::Borrowed(&b"Not Found"[..]))
                    .unwrap()
            }
        });

        if is_newtab {
            builder = builder.with_html(home_html_content);
        } else if is_settings {
            let active_ver = evergreen_core::env::detect_webview2_runtime()
                .unwrap_or_else(|| runtime_ver.clone());
            builder =
                builder.with_html(settings_ui::get_settings_html(&active_ver, &self.settings));
        } else {
            builder = builder.with_url(url);
        }

        let proxy_nav_interceptor = self.proxy.clone();
        let webview = builder
            .with_document_title_changed_handler(move |title: String| {
                let _ =
                    proxy_title.send_event(BrowserEvent::TabTitleChanged(win_id, tab_id, title));
            })
            .with_navigation_handler(move |nav_url: String| {
                if nav_url.starts_with("data:text/html") {
                    return true;
                }
                if nav_url.starts_with("evergreen://settings") {
                    let _ = proxy_nav_interceptor
                        .send_event(BrowserEvent::Ipc(win_id, UiToHostMessage::OpenSettings));
                    return false;
                }
                if !is_persistent && settings_nav.is_site_persistent(&nav_url) {
                    let _ = proxy_nav_interceptor.send_event(BrowserEvent::Ipc(
                        win_id,
                        UiToHostMessage::Navigate { url: nav_url },
                    ));
                    return false;
                }
                let _ = proxy_nav.send_event(BrowserEvent::TabNavigated(win_id, tab_id, nav_url));
                true
            })
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                if let Ok(msg) = serde_json::from_str::<UiToHostMessage>(req.body()) {
                    let _ = proxy_ipc.send_event(BrowserEvent::Ipc(win_id, msg));
                }
            })
            .build_as_child(window)?;

        accelerator::attach_accelerator_keys(&webview, win_id, self.proxy.clone());
        accelerator::attach_navigation_events(
            &webview,
            win_id,
            tab_id,
            self.proxy.clone(),
            self.allowed_cert_hosts.clone(),
        );

        Ok(webview)
    }

    fn handle_create_tab(&mut self, win_id: WindowId, url: Option<String>) {
        let tab_id = self.next_tab_id();
        let target_url = url.unwrap_or_else(|| "evergreen://newtab".to_string());

        if let Some(win_ctx) = self.windows.get_mut(&win_id) {
            let tab_state = TabState {
                id: tab_id,
                url: target_url.clone(),
                title: if target_url == "evergreen://settings" {
                    "Settings".to_string()
                } else {
                    "New Tab".to_string()
                },
                favicon_uri: None,
                is_loading: false,
                can_go_back: false,
                can_go_forward: false,
                is_audio_playing: false,
                is_muted: false,
                status: evergreen_core::tabs::TabStatus::Active,
                last_active_timestamp_secs: Self::now_secs(),
            };
            let actual_id = win_ctx.tab_manager.insert_tab(tab_state, None);

            // Hide other tabs in this window
            for (id, wv) in &win_ctx.tabs {
                if *id != actual_id {
                    let _ = wv.set_visible(false);
                }
            }

            let window = win_ctx.window.clone();
            let win_width = win_ctx.window_width;
            let win_height = win_ctx.window_height;
            let is_sidebar_open = win_ctx.is_sidebar_open;
            let chrome_h = win_ctx.current_chrome_height();

            match self.create_tab_webview(
                win_id,
                actual_id,
                &target_url,
                window.as_ref(),
                win_width,
                win_height,
                is_sidebar_open,
                chrome_h,
            ) {
                Ok(wv) => {
                    let _ = wv.set_visible(true);
                    if let Some(win_ctx) = self.windows.get_mut(&win_id) {
                        let is_persistent = self.settings.is_site_persistent(&target_url);
                        win_ctx.tabs.insert(actual_id, wv);
                        win_ctx.tab_persisted.insert(actual_id, is_persistent);
                        win_ctx.sync_ui_state();
                        win_ctx.update_sidebar_sync(&self.cert_cache);
                        let default_zoom = self.settings.appearance.default_zoom_level;
                        if (default_zoom - 1.0).abs() > f64::EPSILON {
                            win_ctx.set_active_zoom(default_zoom);
                        } else {
                            win_ctx.sync_zoom(1.0);
                        }
                    }
                }
                Err(e) => eprintln!("Failed to create tab webview: {:?}", e),
            }
        }
    }

    fn handle_open_settings(&mut self, win_id: WindowId) {
        if let Some(win_ctx) = self.windows.get_mut(&win_id) {
            win_ctx.close_sidebar();
            if let Some(existing) = win_ctx
                .tab_manager
                .tabs()
                .iter()
                .find(|t| t.url.starts_with("evergreen://settings"))
            {
                let id = existing.id;
                win_ctx.handle_switch_tab(id, &self.cert_cache);
                return;
            }
        }
        self.handle_create_tab(win_id, Some("evergreen://settings".to_string()));
    }

    fn handle_close_tab(
        &mut self,
        win_id: WindowId,
        target_id: TabId,
        event_loop: &ActiveEventLoop,
    ) {
        let should_close_window = if let Some(win_ctx) = self.windows.get_mut(&win_id) {
            if let Some(wv) = win_ctx.tabs.remove(&target_id) {
                let _ = wv.set_visible(false);
                drop(wv);
            }
            win_ctx.tab_persisted.remove(&target_id);

            match win_ctx.tab_manager.close_tab(target_id) {
                Some(next_active_id) => {
                    win_ctx.handle_switch_tab(next_active_id, &self.cert_cache);
                    false
                }
                None => true,
            }
        } else {
            false
        };

        if should_close_window {
            self.windows.remove(&win_id);
            if self.windows.is_empty() {
                event_loop.exit();
            }
        }
    }

    fn handle_detach_tab(
        &mut self,
        source_win_id: WindowId,
        tab_id: TabId,
        screen_x: Option<f64>,
        screen_y: Option<f64>,
        event_loop: &ActiveEventLoop,
    ) {
        let tab_state_opt = if let Some(win_ctx) = self.windows.get_mut(&source_win_id) {
            let extracted = win_ctx.tab_manager.extract_tab(tab_id);
            if let Some(wv) = win_ctx.tabs.remove(&tab_id) {
                let _ = wv.set_visible(false);
                drop(wv);
            }
            win_ctx.tab_persisted.remove(&tab_id);
            extracted
        } else {
            None
        };

        if let Some(tab_state) = tab_state_opt {
            let mut target_win_id = None;
            if let (Some(sx), Some(sy)) = (screen_x, screen_y) {
                for (w_id, w_ctx) in &self.windows {
                    if *w_id != source_win_id {
                        if let (Ok(pos), size) =
                            (w_ctx.window.outer_position(), w_ctx.window.outer_size())
                        {
                            let scale = w_ctx.window.scale_factor();
                            let l_pos = pos.to_logical::<f64>(scale);
                            let l_size = size.to_logical::<f64>(scale);
                            if sx >= l_pos.x
                                && sx <= l_pos.x + l_size.width
                                && sy >= l_pos.y
                                && sy <= l_pos.y + l_size.height
                            {
                                target_win_id = Some(*w_id);
                                break;
                            }
                        }
                    }
                }
            }

            if let Some(target_id) = target_win_id {
                self.handle_create_tab(target_id, Some(tab_state.url));
            } else if let Some(new_win_id) =
                self.create_browser_window(event_loop, Some(tab_state.url))
            {
                if let (Some(sx), Some(sy)) = (screen_x, screen_y) {
                    if let Some(new_win) = self.windows.get(&new_win_id) {
                        new_win
                            .window
                            .set_outer_position(winit::dpi::LogicalPosition::new(
                                (sx - 200.0).max(0.0),
                                (sy - 20.0).max(0.0),
                            ));
                    }
                }
            }

            let should_close_source = if let Some(win_ctx) = self.windows.get_mut(&source_win_id) {
                if win_ctx.tab_manager.tabs().is_empty() {
                    true
                } else {
                    if let Some(next_active) = win_ctx.tab_manager.active_tab_id() {
                        win_ctx.handle_switch_tab(next_active, &self.cert_cache);
                    }
                    win_ctx.sync_ui_state();
                    false
                }
            } else {
                false
            };

            if should_close_source {
                self.windows.remove(&source_win_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
        }
    }

    fn handle_navigate(&mut self, win_id: WindowId, raw_url: &str) {
        let search_template = self.settings.search_url_template();
        match normalize_url(raw_url, search_template) {
            Ok(target) => {
                if target.starts_with("evergreen://settings") {
                    self.handle_open_settings(win_id);
                    return;
                }
                let desired_persistent = self.settings.is_site_persistent(&target);

                // Scope 1: inspect active tab and determine if partition transition is required
                let tab_info = if let Some(win_ctx) = self.windows.get_mut(&win_id) {
                    if let Some(active) = win_ctx.tab_manager.active_tab() {
                        let active_id = active.id;
                        let current_persistent = win_ctx
                            .tab_persisted
                            .get(&active_id)
                            .copied()
                            .unwrap_or(false);

                        win_ctx.tab_manager.update_url(active_id, target.clone());
                        if let Some(host) = extract_host(&target) {
                            win_ctx.tab_manager.update_favicon(
                                active_id,
                                Some(format!("https://icons.duckduckgo.com/ip3/{}.ico", host)),
                            );
                        }

                        if current_persistent == desired_persistent {
                            if let Some(wv) = win_ctx.tabs.get(&active_id) {
                                let _ = wv.load_url(&target);
                            }
                            win_ctx.sync_ui_state();
                            None
                        } else {
                            Some((
                                active_id,
                                win_ctx.window.clone(),
                                win_ctx.window_width,
                                win_ctx.window_height,
                                win_ctx.is_sidebar_open,
                                win_ctx.current_chrome_height(),
                            ))
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Scope 2: partition transition (ephemeral <-> persistent): recreate webview
                if let Some((active_id, window, win_width, win_height, is_sidebar_open, chrome_h)) =
                    tab_info
                {
                    match self.create_tab_webview(
                        win_id,
                        active_id,
                        &target,
                        window.as_ref(),
                        win_width,
                        win_height,
                        is_sidebar_open,
                        chrome_h,
                    ) {
                        Ok(new_wv) => {
                            let _ = new_wv.set_visible(true);
                            if let Some(win_ctx) = self.windows.get_mut(&win_id) {
                                win_ctx.tabs.insert(active_id, new_wv);
                                win_ctx.tab_persisted.insert(active_id, desired_persistent);
                                win_ctx.sync_ui_state();
                                win_ctx.update_sidebar_sync(&self.cert_cache);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to recreate tab on persistence transition: {:?}", e)
                        }
                    }
                }
            }
            Err(err) => eprintln!("Navigation error: {}", err),
        }
    }

    #[cfg(target_os = "windows")]
    fn show_native_menu(
        &mut self,
        win_id: WindowId,
        logical_x: f64,
        logical_y: f64,
        event_loop: &ActiveEventLoop,
    ) {
        use windows::core::w;
        use windows::Win32::Foundation::{HWND, POINT};
        use windows::Win32::UI::WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, DestroyMenu, TrackPopupMenuEx, MF_SEPARATOR, MF_STRING,
            TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_TOPALIGN,
        };
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        if let Some(win_ctx) = self.windows.get(&win_id) {
            if let Ok(handle) = win_ctx.window.window_handle() {
                if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                    let hwnd = HWND(win32_handle.hwnd.get() as _);
                    let scale = win_ctx.window.scale_factor();
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
                                1001 => self.handle_create_tab(win_id, None),
                                1002 => {
                                    self.create_browser_window(event_loop, None);
                                }
                                1003 => {
                                    if let Some(win) = self.windows.get(&win_id) {
                                        if let Some(active) = win.tab_manager.active_tab() {
                                            if let Some(wv) = win.tabs.get(&active.id) {
                                                wv.open_devtools();
                                            }
                                        }
                                    }
                                }
                                1004 => self.handle_open_settings(win_id),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn show_native_menu(
        &mut self,
        _win_id: WindowId,
        _x: f64,
        _y: f64,
        _event_loop: &ActiveEventLoop,
    ) {
    }
}

impl ApplicationHandler<BrowserEvent> for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.windows.is_empty() {
            self.create_browser_window(event_loop, None);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: BrowserEvent) {
        match event {
            BrowserEvent::Ipc(win_id, action) => match action {
                UiToHostMessage::ChromeReady => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.sync_ui_state();
                    }
                }
                UiToHostMessage::CreateTab { url } => {
                    self.handle_create_tab(win_id, url);
                }
                UiToHostMessage::OpenNewWindow => {
                    self.create_browser_window(event_loop, None);
                }
                UiToHostMessage::SwitchTab { id } => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.handle_switch_tab(id, &self.cert_cache);
                    }
                }
                UiToHostMessage::CloseTab { id } => {
                    self.handle_close_tab(win_id, id, event_loop);
                }
                UiToHostMessage::Navigate { url } => {
                    self.handle_navigate(win_id, &url);
                }
                UiToHostMessage::GoBack => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.go_back();
                            }
                        }
                    }
                }
                UiToHostMessage::GoForward => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.go_forward();
                            }
                        }
                    }
                }
                UiToHostMessage::Reload => {
                    let reload_nav = if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            let desired = self.settings.is_site_persistent(&active.url);
                            let current =
                                win.tab_persisted.get(&active.id).copied().unwrap_or(false);
                            if current != desired {
                                Some(active.url.clone())
                            } else {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    let _ = wv.reload();
                                }
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(url) = reload_nav {
                        self.handle_navigate(win_id, &url);
                    }
                }
                UiToHostMessage::Stop => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("window.stop()");
                            }
                        }
                    }
                }
                UiToHostMessage::MenuToggled { open } => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(chrome) = &win.chrome_webview {
                            let height = if open {
                                260.0
                            } else {
                                win.current_chrome_height()
                            };
                            let chrome_bounds = create_bounds(0.0, 0.0, win.window_width, height);
                            let _ = chrome.set_bounds(chrome_bounds);
                        }
                    }
                }
                UiToHostMessage::ToggleMenuPanel => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.toggle_sidebar("menu", &self.cert_cache, self.proxy.clone());
                    }
                }
                UiToHostMessage::ToggleSecurityPanel => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.toggle_sidebar("security", &self.cert_cache, self.proxy.clone());
                    }
                }
                UiToHostMessage::ToggleDownloadsSidebar => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.toggle_sidebar("downloads", &self.cert_cache, self.proxy.clone());
                    }
                }
                UiToHostMessage::CloseSidebar => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.close_sidebar();
                    }
                }
                UiToHostMessage::OpenCertificateDialog { host } => {
                    cert::open_native_certificate_dialog(&host);
                }
                UiToHostMessage::PageNavigated { url, title } => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            let active_id = active.id;
                            if !url.starts_with("data:text/html") {
                                win.tab_manager.update_url(active_id, url.clone());
                                if let Some(host) = extract_host(&url) {
                                    win.tab_manager.update_favicon(
                                        active_id,
                                        Some(format!(
                                            "https://icons.duckduckgo.com/ip3/{}.ico",
                                            host
                                        )),
                                    );
                                }
                            }
                            if !title.is_empty() {
                                win.tab_manager.update_title(active_id, title);
                            }
                            win.sync_ui_state();
                            win.update_sidebar_sync(&self.cert_cache);
                        }
                    }
                }
                UiToHostMessage::OpenDevTools => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                wv.open_devtools();
                            }
                        }
                    }
                }
                UiToHostMessage::OpenMenu { x, y } => {
                    self.show_native_menu(win_id, x, y, event_loop);
                }
                UiToHostMessage::OpenSettings => {
                    self.handle_open_settings(win_id);
                }
                UiToHostMessage::SetSearchEngine { engine } => {
                    self.settings.search_engine = engine.clone();
                    let path = get_settings_path();
                    let _ = self.settings.save_to_path(&path);
                    for win in self.windows.values() {
                        if let Some(chrome) = &win.chrome_webview {
                            let _ = chrome.evaluate_script(&format!(
                                "if (window.__syncSearchEngine) {{ window.__syncSearchEngine('{}'); }}",
                                engine
                            ));
                        }
                    }
                }
                UiToHostMessage::ReorderTab {
                    from_index,
                    to_index,
                } => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.tab_manager.reorder_tab(from_index, to_index);
                        win.sync_ui_state();
                    }
                }
                UiToHostMessage::DetachTabToNewWindow {
                    tab_id,
                    screen_x,
                    screen_y,
                } => {
                    self.handle_detach_tab(win_id, tab_id, screen_x, screen_y, event_loop);
                }
                UiToHostMessage::SetZoom { factor } => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.set_active_zoom(factor);
                    }
                }
                UiToHostMessage::ZoomIn => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.set_active_zoom(win.get_active_zoom() + 0.1);
                    }
                }
                UiToHostMessage::ZoomOut => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.set_active_zoom(win.get_active_zoom() - 0.1);
                    }
                }
                UiToHostMessage::ZoomReset => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.set_active_zoom(1.0);
                    }
                }
                UiToHostMessage::OpenFindInPage => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.find_bar_open = true;
                        win.update_layout();
                    }
                }
                UiToHostMessage::FindInPage { query, forward } => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.handle_find_in_page(&query, forward);
                    }
                }
                UiToHostMessage::CloseFindInPage => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.find_bar_open = false;
                        win.handle_close_find_in_page();
                        win.update_layout();
                    }
                }
                UiToHostMessage::FindResult { current, total } => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(chrome) = &win.chrome_webview {
                            let _ = chrome.evaluate_script(&format!(
                                "if (window.__syncFindResult) {{ window.__syncFindResult({{ current: {}, total: {} }}); }}",
                                current, total
                            ));
                        }
                    }
                }
                UiToHostMessage::OpenDownloads => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.toggle_sidebar("downloads", &self.cert_cache, self.proxy.clone());
                    }
                }
                UiToHostMessage::DownloadConfirm { .. } => {}
                UiToHostMessage::CancelDownload { .. } => {}
                UiToHostMessage::OpenPermissionPrompt => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.permission_bar_open = true;
                        win.update_layout();
                    }
                }
                UiToHostMessage::ClosePermissionPrompt => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.permission_bar_open = false;
                        win.update_layout();
                    }
                }
                UiToHostMessage::PermissionResponse {
                    permission_id,
                    allow,
                } => {
                    accelerator::resolve_permission(permission_id, allow);
                }
                UiToHostMessage::TriggerLinkPreview { url, .. } => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(chrome) = &win.chrome_webview {
                            let escaped = url.replace('\\', "\\\\").replace('\'', "\\'");
                            let _ = chrome.evaluate_script(&format!(
                                "if (window.__showStatusPreview) {{ window.__showStatusPreview('{}'); }}",
                                escaped
                            ));
                        }
                    }
                }
                UiToHostMessage::SaveSettings { settings_json } => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&settings_json) {
                        self.settings.update_from_json(&val);
                        let path = get_settings_path();
                        let _ = self.settings.save_to_path(&path);
                        for win in self.windows.values() {
                            if let Some(chrome) = &win.chrome_webview {
                                let _ = chrome.evaluate_script(&format!(
                                    "if (window.__syncSearchEngine) {{ window.__syncSearchEngine('{}'); }}",
                                    self.settings.search_engine
                                ));
                            }
                        }
                    }
                }
                UiToHostMessage::RunEngineUpdate => {
                    let proxy = self.proxy.clone();
                    std::thread::spawn(move || {
                        let cmd = "powershell -NoProfile -Command \"$u='https://go.microsoft.com/fwlink/p/?LinkId=2124703'; $o=\\\"$env:TEMP\\MicrosoftEdgeWebview2Setup.exe\\\"; Invoke-WebRequest -Uri $u -OutFile $o; Start-Process -FilePath $o -ArgumentList '/silent','/install' -Wait\"";
                        match run_local_command(cmd) {
                            Ok(res) => {
                                let _ = proxy.send_event(BrowserEvent::CommandDone(
                                    "Engine Update".to_string(),
                                    res.success,
                                    res.stdout,
                                ));
                            }
                            Err(e) => {
                                let _ = proxy.send_event(BrowserEvent::CommandDone(
                                    "Engine Update".to_string(),
                                    false,
                                    e.to_string(),
                                ));
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
                                let _ = proxy.send_event(BrowserEvent::CommandDone(
                                    "Fork Update".to_string(),
                                    res.success,
                                    res.stdout,
                                ));
                            }
                            Err(e) => {
                                let _ = proxy.send_event(BrowserEvent::CommandDone(
                                    "Fork Update".to_string(),
                                    false,
                                    e.to_string(),
                                ));
                            }
                        }
                    });
                }
                UiToHostMessage::ReloadTab { tab_id } => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        if let Some(tab) = win
                            .tab_manager
                            .tabs_mut()
                            .iter_mut()
                            .find(|t| t.id == tab_id)
                        {
                            if tab.status == evergreen_core::tabs::TabStatus::Crashed {
                                tab.status = evergreen_core::tabs::TabStatus::Active;
                            }
                        }
                        if let Some(wv) = win.tabs.get(&tab_id) {
                            let _ = wv.reload();
                        }
                        win.sync_ui_state();
                    }
                }
                UiToHostMessage::BypassCertificateError { tab_id, host, url } => {
                    eprintln!("[TLS BYPASS INITIATED] Bypassing certificate error for host: {} on Tab {:?}", host, tab_id);
                    if let Ok(mut set) = self.allowed_cert_hosts.lock() {
                        set.insert(host.clone());
                    }
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(wv) = win.tabs.get(&tab_id) {
                            let _ = wv.load_url(&url);
                        }
                    }
                }
            },
            BrowserEvent::TabTitleChanged(win_id, tab_id, title) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.tab_manager.update_title(tab_id, title);
                    win.sync_ui_state();
                    win.update_sidebar_sync(&self.cert_cache);
                }
            }
            BrowserEvent::TabNavigated(win_id, tab_id, url) => {
                if url.starts_with("data:text/html") || url == "about:blank" || url == "aboutblank"
                {
                    return;
                }
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.tab_manager.update_url(tab_id, url.clone());
                    if let Some(host) = extract_host(&url) {
                        win.tab_manager.update_favicon(
                            tab_id,
                            Some(format!("https://icons.duckduckgo.com/ip3/{}.ico", host)),
                        );
                    }
                    win.sync_ui_state();
                    win.update_sidebar_sync(&self.cert_cache);
                }
            }
            BrowserEvent::HistoryChanged(win_id, tab_id, can_back, can_forward) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.tab_manager
                        .update_history_state(tab_id, can_back, can_forward);
                    win.sync_ui_state();
                }
            }
            BrowserEvent::CommandDone(name, success, output) => {
                println!(
                    "[COMMAND RESULT] {}: success={} (output: {})",
                    name,
                    success,
                    output.trim()
                );
            }
            BrowserEvent::DownloadProgress {
                download_id,
                filename,
                received_bytes,
                total_bytes,
                state,
            } => {
                let json = serde_json::json!({
                    "download_id": download_id,
                    "filename": filename,
                    "received_bytes": received_bytes,
                    "total_bytes": total_bytes,
                    "state": state,
                });
                let script_chrome = format!(
                    "if (window.__downloadProgress) {{ window.__downloadProgress({}); }}",
                    json
                );
                let script_sidebar = format!(
                    "if (window.__syncDownloads) {{ window.__syncDownloads({}); }}",
                    json
                );
                for win in self.windows.values() {
                    if let Some(chrome) = &win.chrome_webview {
                        let _ = chrome.evaluate_script(&script_chrome);
                    }
                    if let Some(sidebar) = &win.sidebar_webview {
                        let _ = sidebar.evaluate_script(&script_sidebar);
                    }
                }
            }
            BrowserEvent::PermissionPrompt {
                window_id,
                permission_id,
                origin,
                permission_kind,
            } => {
                if let Some(win) = self.windows.get(&window_id) {
                    if let Some(chrome) = &win.chrome_webview {
                        let json = serde_json::json!({
                            "permission_id": permission_id,
                            "origin": origin,
                            "permission_kind": permission_kind,
                        });
                        let _ = chrome.evaluate_script(&format!(
                            "if (window.__permissionPrompt) {{ window.__permissionPrompt({}); }}",
                            json
                        ));
                    }
                }
            }
            BrowserEvent::TabCrashed(win_id, tab_id, kind) => {
                eprintln!(
                    "[TAB CRASH CONTAINMENT] Window {:?}, Tab {:?}, kind={}",
                    win_id, tab_id, kind
                );
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.tab_manager.mark_tab_crashed(tab_id);
                    win.sync_ui_state();

                    // If the entire browser engine process exited (kind == 0), reconstruct state from snapshot
                    if kind == 0 {
                        eprintln!(
                            "[ENGINE PROCESS RECONSTRUCTION] Reconstructing tabs from snapshot"
                        );
                        let snapshot = win.tab_manager.snapshot();
                        for tab in snapshot {
                            if let Some(wv) = win.tabs.get(&tab.id) {
                                let _ = wv.load_url(&tab.url);
                            }
                        }
                    } else if let Some(wv) = win.tabs.get(&tab_id) {
                        // Render per-tab recovery banner directly inside the crashed tab's webview
                        let crash_banner_html = format!(
                            r#"data:text/html,<!DOCTYPE html><html><head><meta charset="utf-8"><title>Tab Stopped Responding</title><style>body{{margin:0;padding:0;background:%23181820;color:%23f1f5f9;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;}}.box{{text-align:center;max-width:480px;padding:32px;background:%231f1f2a;border:1px solid rgba(255,255,255,0.08);border-radius:12px;box-shadow:0 8px 32px rgba(0,0,0,0.5);}}.icon{{font-size:40px;margin-bottom:12px;}}h1{{font-size:20px;font-weight:600;margin:0 0 8px 0;color:%23f87171;}}p{{font-size:13px;color:%2394a3b8;line-height:1.5;margin:0 0 24px 0;}}button{{background:%233b82f6;color:white;border:none;border-radius:6px;padding:10px 20px;font-size:13px;font-weight:500;cursor:pointer;}}button:hover{{background:%232563eb;}}</style></head><body><div class="box"><div class="icon">⚠️</div><h1>Page Stopped Responding</h1><p>This web page or renderer process encountered a problem and had to be halted. Other tabs and your browser session remain safe.</p><button onclick="if(window.ipc){{window.ipc.postMessage(JSON.stringify({{action:'ReloadTab',payload:{{tab_id:{}}}}}));}}else{{location.reload();}}">Reload Page</button></div></body></html>"#,
                            tab_id.0
                        );
                        let _ = wv.load_url(&crash_banner_html);
                    }
                }
            }
            BrowserEvent::ServerCertificateError {
                window_id,
                tab_id,
                request_uri,
                error_status,
            } => {
                let host = extract_host(&request_uri).unwrap_or_else(|| request_uri.clone());
                eprintln!(
                    "[STRICT TLS] Certificate error {} for {} on Tab {:?}",
                    error_status, request_uri, tab_id
                );
                if let Some(win) = self.windows.get(&window_id) {
                    if let Some(wv) = win.tabs.get(&tab_id) {
                        let warning_html = format!(
                            r#"data:text/html,<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>Security Warning: Untrusted Certificate</title><style>:root{{color-scheme:dark;--bg-gradient:radial-gradient(circle at 50% 25%,%2322222e 0%,%2316161d 100%);--surface-card:rgba(255,255,255,0.04);--border-subtle:rgba(255,255,255,0.08);--text-main:%23f0f0f5;--text-muted:%239595a8;--accent:%234e8cff;--accent-hover:%233b76e1;--danger:%23ef4444;--font-family:"Segoe UI Variable Text","Segoe UI",system-ui,-apple-system,sans-serif;}}*{{box-sizing:border-box;margin:0;padding:0;}}body{{background:var(--bg-gradient);color:var(--text-main);font-family:var(--font-family);min-height:100vh;display:flex;align-items:center;justify-content:center;padding:32px 24px;user-select:none;}}.card{{background:var(--surface-card);backdrop-filter:blur(16px);border:1px solid var(--border-subtle);border-radius:16px;box-shadow:0 16px 40px rgba(0,0,0,0.5);padding:40px 36px;max-width:540px;width:100%;text-align:center;}}.icon-badge{{width:64px;height:64px;margin:0 auto 20px;border-radius:50%;background:rgba(239,68,68,0.12);border:1px solid rgba(239,68,68,0.3);display:flex;align-items:center;justify-content:center;}}.icon-badge svg{{width:32px;height:32px;stroke:var(--danger);stroke-width:2;stroke-linecap:round;stroke-linejoin:round;fill:none;}}h1{{font-size:22px;font-weight:600;letter-spacing:-0.3px;margin-bottom:12px;color:var(--text-main);}}p.desc{{font-size:13px;color:var(--text-muted);line-height:1.6;margin-bottom:20px;}}p.desc strong{{color:var(--text-main);word-break:break-all;}}.uri-container{{margin-bottom:24px;padding:10px 14px;background:rgba(0,0,0,0.3);border:1px solid var(--border-subtle);border-radius:8px;display:flex;align-items:center;justify-content:space-between;gap:12px;font-size:12px;font-family:monospace;}}.uri-text{{color:%23cbd5e1;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;flex:1;text-align:left;}}.error-code{{color:%23f87171;background:rgba(239,68,68,0.15);padding:2px 8px;border-radius:4px;font-size:11px;flex-shrink:0;}}.button-group{{display:flex;align-items:center;justify-content:center;gap:12px;}}button{{border:none;border-radius:8px;padding:10px 22px;font-size:13px;font-weight:500;cursor:pointer;font-family:inherit;transition:background 0.12s ease;}}.btn-safety{{background:var(--accent);color:%23ffffff;font-weight:600;box-shadow:0 4px 14px rgba(78,140,255,0.3);}}.btn-safety:hover{{background:var(--accent-hover);}}.btn-advanced{{background:rgba(255,255,255,0.06);color:var(--text-muted);border:1px solid var(--border-subtle);}}.btn-advanced:hover{{background:rgba(255,255,255,0.1);color:var(--text-main);}}.advanced-drawer{{display:none;margin-top:24px;padding:18px;background:rgba(0,0,0,0.35);border:1px solid rgba(255,255,255,0.06);border-radius:10px;text-align:left;}}.advanced-drawer p{{font-size:12px;color:var(--text-muted);line-height:1.6;margin-bottom:14px;}}.btn-proceed{{background:transparent;color:%23f87171;padding:6px 12px;border:1px solid rgba(239,68,68,0.3);border-radius:6px;font-size:12px;display:inline-block;}}.btn-proceed:hover{{background:rgba(239,68,68,0.12);color:%23fca5a5;}}</style></head><body><div class="card"><div class="icon-badge"><svg viewBox="0 0 24 24"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg></div><h1>Your connection is not private</h1><p class="desc">Evergreen Browser prevented connection to <strong>{host}</strong> because its security certificate is untrusted, self-signed, or expired. Attackers might be trying to steal your information.</p><div class="uri-container"><span class="uri-text">{request_uri}</span><span class="error-code">NET::ERR_CERT_INVALID</span></div><div class="button-group"><button class="btn-safety" id="safetyBtn" onclick="goBack()">Go Back to Safety</button><button class="btn-advanced" id="advancedBtn" onclick="toggleAdvanced()">Advanced ▾</button></div><div class="advanced-drawer" id="advancedDrawer"><p>This server could not prove that it is <strong>{host}</strong>; its security certificate is not trusted by your computer's operating system. This may be caused by a misconfiguration or an attacker intercepting your connection.</p><button class="btn-proceed" id="proceedBtn" onclick="bypass()">Proceed to {host} (unsafe)</button></div></div><script>function goBack(){{if(window.history.length>1){{window.history.back();}}else{{location.href='evergreen://newtab';}}}}function toggleAdvanced(){{const d=document.getElementById('advancedDrawer');const b=document.getElementById('advancedBtn');const h=(d.style.display==='none'||!d.style.display);d.style.display=h?'block':'none';b.textContent=h?'Advanced ▴':'Advanced ▾';}}function bypass(){{if(window.ipc){{window.ipc.postMessage(JSON.stringify({{action:'BypassCertificateError',payload:{{tab_id:{tab_id},host:'{host}',url:'{request_uri}'}}}}));}}else{{location.reload();}}}}</script></body></html>"#,
                            host = host,
                            request_uri = request_uri,
                            tab_id = tab_id.0
                        );
                        let _ = wv.load_url(&warning_html);
                    }
                }
            }
            BrowserEvent::Shortcut(win_id, shortcut) => match shortcut.as_str() {
                "Ctrl+T" => self.handle_create_tab(win_id, None),
                "Ctrl+Shift+T" => {
                    let target_url = if let Some(win) = self.windows.get_mut(&win_id) {
                        win.tab_manager.pop_last_closed()
                    } else {
                        None
                    };
                    if let Some(url) = target_url {
                        self.handle_create_tab(win_id, Some(url));
                    }
                }
                "Ctrl+N" => {
                    self.create_browser_window(event_loop, None);
                }
                "Ctrl+Shift+N" => {
                    self.create_browser_window(event_loop, None);
                }
                "Ctrl+W" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            let id = active.id;
                            self.handle_close_tab(win_id, id, event_loop);
                        }
                    }
                }
                "Ctrl+L" | "Ctrl+E" | "Ctrl+K" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.handle_focus_omnibox();
                    }
                }
                "Ctrl+F" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(chrome) = &win.chrome_webview {
                            let _ = chrome.evaluate_script(
                                "if (window.__toggleFindBar) { window.__toggleFindBar(); }",
                            );
                        }
                    }
                }
                "Ctrl+J" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.toggle_sidebar("downloads", &self.cert_cache, self.proxy.clone());
                    }
                }
                "Ctrl+P" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("window.print()");
                            }
                        }
                    }
                }
                "Ctrl+Shift+R" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("location.reload(true)");
                            }
                        }
                    }
                }
                "Ctrl+R" | "F5" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.reload();
                            }
                        }
                    }
                }
                "Alt+Left" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.go_back();
                            }
                        }
                    }
                }
                "Alt+Right" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.go_forward();
                            }
                        }
                    }
                }
                "Ctrl+Tab" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        let tabs = win.tab_manager.tabs();
                        if tabs.len() > 1 {
                            let current_idx = tabs
                                .iter()
                                .position(|t| {
                                    Some(t.id) == win.tab_manager.active_tab().map(|a| a.id)
                                })
                                .unwrap_or(0);
                            let next_idx = (current_idx + 1) % tabs.len();
                            let next_id = tabs[next_idx].id;
                            win.handle_switch_tab(next_id, &self.cert_cache);
                        }
                    }
                }
                "Ctrl+Shift+Tab" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        let tabs = win.tab_manager.tabs();
                        if tabs.len() > 1 {
                            let current_idx = tabs
                                .iter()
                                .position(|t| {
                                    Some(t.id) == win.tab_manager.active_tab().map(|a| a.id)
                                })
                                .unwrap_or(0);
                            let prev_idx = if current_idx == 0 {
                                tabs.len() - 1
                            } else {
                                current_idx - 1
                            };
                            let prev_id = tabs[prev_idx].id;
                            win.handle_switch_tab(prev_id, &self.cert_cache);
                        }
                    }
                }
                "Ctrl+1" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(0, &self.cert_cache);
                    }
                }
                "Ctrl+2" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(1, &self.cert_cache);
                    }
                }
                "Ctrl+3" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(2, &self.cert_cache);
                    }
                }
                "Ctrl+4" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(3, &self.cert_cache);
                    }
                }
                "Ctrl+5" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(4, &self.cert_cache);
                    }
                }
                "Ctrl+6" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(5, &self.cert_cache);
                    }
                }
                "Ctrl+7" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(6, &self.cert_cache);
                    }
                }
                "Ctrl+8" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        win.switch_to_tab_index(7, &self.cert_cache);
                    }
                }
                "Ctrl+9" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        let tabs = win.tab_manager.tabs();
                        if !tabs.is_empty() {
                            let last_id = tabs.last().unwrap().id;
                            win.handle_switch_tab(last_id, &self.cert_cache);
                        }
                    }
                }
                "Ctrl+Plus" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.set_active_zoom(win.get_active_zoom() + 0.1);
                    }
                }
                "Ctrl+Minus" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.set_active_zoom(win.get_active_zoom() - 0.1);
                    }
                }
                "Ctrl+Zero" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        win.set_active_zoom(1.0);
                    }
                }
                "Escape" => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        if win.is_sidebar_open {
                            win.close_sidebar();
                            if let Some(c) = &win.chrome_webview {
                                let _ = c.evaluate_script("if (window.__syncSidebarState) window.__syncSidebarState(null);");
                            }
                        } else if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                let _ = wv.evaluate_script("window.stop()");
                            }
                        }
                    }
                }
                "F12" => {
                    if let Some(win) = self.windows.get(&win_id) {
                        if let Some(active) = win.tab_manager.active_tab() {
                            if let Some(wv) = win.tabs.get(&active.id) {
                                wv.open_devtools();
                            }
                        }
                    }
                }
                _ => {}
            },
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(win_ctx) = self.windows.get_mut(&window_id) {
                    let scale = win_ctx.window.scale_factor();
                    let logical = physical_size.to_logical::<f64>(scale);
                    win_ctx.window_width = logical.width;
                    win_ctx.window_height = logical.height;
                    win_ctx.update_layout();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::Focused(true) => {
                if let Some(win_ctx) = self.windows.get(&window_id) {
                    win_ctx.wake_active();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                match event.logical_key {
                    Key::Named(NamedKey::F12) => {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(active) = win.tab_manager.active_tab() {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    wv.open_devtools();
                                }
                            }
                        }
                    }
                    Key::Character(ref s)
                        if s.eq_ignore_ascii_case("t") && self.modifiers.control_key() =>
                    {
                        if self.modifiers.shift_key() {
                            let target_url = if let Some(win) = self.windows.get_mut(&window_id) {
                                win.tab_manager.pop_last_closed()
                            } else {
                                None
                            };
                            if let Some(url) = target_url {
                                self.handle_create_tab(window_id, Some(url));
                            }
                        } else {
                            self.handle_create_tab(window_id, None);
                        }
                    }
                    Key::Character(ref s)
                        if s.eq_ignore_ascii_case("n") && self.modifiers.control_key() =>
                    {
                        self.create_browser_window(event_loop, None);
                    }
                    Key::Character(ref s)
                        if s.eq_ignore_ascii_case("w") && self.modifiers.control_key() =>
                    {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(active) = win.tab_manager.active_tab() {
                                let id = active.id;
                                self.handle_close_tab(window_id, id, event_loop);
                            }
                        }
                    }
                    Key::Character(ref s)
                        if (s.eq_ignore_ascii_case("l")
                            || s.eq_ignore_ascii_case("e")
                            || s.eq_ignore_ascii_case("k"))
                            && self.modifiers.control_key() =>
                    {
                        if let Some(win) = self.windows.get(&window_id) {
                            win.handle_focus_omnibox();
                        }
                    }
                    Key::Character(ref s)
                        if s.eq_ignore_ascii_case("f") && self.modifiers.control_key() =>
                    {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(chrome) = &win.chrome_webview {
                                let _ = chrome.evaluate_script(
                                    "if (window.__toggleFindBar) { window.__toggleFindBar(); }",
                                );
                            }
                        }
                    }
                    Key::Character(ref s)
                        if s.eq_ignore_ascii_case("j") && self.modifiers.control_key() =>
                    {
                        if let Some(win) = self.windows.get_mut(&window_id) {
                            win.toggle_sidebar("downloads", &self.cert_cache, self.proxy.clone());
                        }
                    }
                    Key::Character(ref s)
                        if s.eq_ignore_ascii_case("p") && self.modifiers.control_key() =>
                    {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(active) = win.tab_manager.active_tab() {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    let _ = wv.evaluate_script("window.print()");
                                }
                            }
                        }
                    }
                    Key::Character(ref s)
                        if s.eq_ignore_ascii_case("r") && self.modifiers.control_key() =>
                    {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(active) = win.tab_manager.active_tab() {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    if self.modifiers.shift_key() {
                                        let _ = wv.evaluate_script("location.reload(true)");
                                    } else {
                                        let _ = wv.reload();
                                    }
                                }
                            }
                        }
                    }
                    Key::Named(NamedKey::F5) => {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(active) = win.tab_manager.active_tab() {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    if self.modifiers.control_key() || self.modifiers.shift_key() {
                                        let _ = wv.evaluate_script("location.reload(true)");
                                    } else {
                                        let _ = wv.reload();
                                    }
                                }
                            }
                        }
                    }
                    Key::Named(NamedKey::Escape) => {
                        if let Some(win) = self.windows.get_mut(&window_id) {
                            if win.sidebar_webview.is_some() {
                                win.sidebar_webview = None;
                                win.is_sidebar_open = false;
                                win.update_layout();
                                if let Some(c) = &win.chrome_webview {
                                    let _ = c.evaluate_script("if (window.__syncSidebarState) window.__syncSidebarState(null);");
                                }
                            } else if let Some(active) = win.tab_manager.active_tab() {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    let _ = wv.evaluate_script("window.stop()");
                                }
                            }
                        }
                    }
                    Key::Character(ref s)
                        if self.modifiers.control_key()
                            && s.len() == 1
                            && s.chars().next().is_some_and(|c| c.is_ascii_digit()) =>
                    {
                        let digit = s.chars().next().unwrap().to_digit(10).unwrap();
                        if let Some(win) = self.windows.get_mut(&window_id) {
                            if (1..=8).contains(&digit) {
                                win.switch_to_tab_index((digit - 1) as usize, &self.cert_cache);
                            } else if digit == 9 {
                                let tabs = win.tab_manager.tabs();
                                if !tabs.is_empty() {
                                    let last_id = tabs.last().unwrap().id;
                                    win.handle_switch_tab(last_id, &self.cert_cache);
                                }
                            }
                        }
                    }
                    Key::Character(ref s)
                        if self.modifiers.control_key() && (s == "+" || s == "=") =>
                    {
                        if let Some(win) = self.windows.get(&window_id) {
                            win.set_active_zoom(win.get_active_zoom() + 0.1);
                        }
                    }
                    Key::Character(ref s)
                        if self.modifiers.control_key() && (s == "-" || s == "_") =>
                    {
                        if let Some(win) = self.windows.get(&window_id) {
                            win.set_active_zoom(win.get_active_zoom() - 0.1);
                        }
                    }
                    Key::Character(ref s) if self.modifiers.control_key() && s == "0" => {
                        if let Some(win) = self.windows.get(&window_id) {
                            win.set_active_zoom(1.0);
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) if self.modifiers.alt_key() => {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(active) = win.tab_manager.active_tab() {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    let _ = wv.go_back();
                                }
                            }
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) if self.modifiers.alt_key() => {
                        if let Some(win) = self.windows.get(&window_id) {
                            if let Some(active) = win.tab_manager.active_tab() {
                                if let Some(wv) = win.tabs.get(&active.id) {
                                    let _ = wv.go_forward();
                                }
                            }
                        }
                    }
                    Key::Named(NamedKey::Tab) if self.modifiers.control_key() => {
                        if let Some(win) = self.windows.get_mut(&window_id) {
                            let tabs = win.tab_manager.tabs();
                            if tabs.len() > 1 {
                                let current_idx = tabs
                                    .iter()
                                    .position(|t| {
                                        Some(t.id) == win.tab_manager.active_tab().map(|a| a.id)
                                    })
                                    .unwrap_or(0);
                                if self.modifiers.shift_key() {
                                    let prev_idx = if current_idx == 0 {
                                        tabs.len() - 1
                                    } else {
                                        current_idx - 1
                                    };
                                    let prev_id = tabs[prev_idx].id;
                                    win.handle_switch_tab(prev_id, &self.cert_cache);
                                } else {
                                    let next_idx = (current_idx + 1) % tabs.len();
                                    let next_id = tabs[next_idx].id;
                                    win.handle_switch_tab(next_id, &self.cert_cache);
                                }
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
        #[cfg(target_os = "windows")]
        {
            use windows::core::w;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
            unsafe {
                let _ = MessageBoxW(
                    None,
                    w!("Running Evergreen Browser with Administrator privileges is prohibited for security reasons.\n\nPlease launch the browser as a standard user."),
                    w!("Evergreen Browser - Security Warning"),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        eprintln!("SECURITY ERROR: Running as Administrator / elevated is strictly prohibited.");
        std::process::exit(1);
    }

    // 2. Portable mode / Data directory initialization
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or(&p).to_path_buf())
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let args: Vec<String> = std::env::args().collect();

    // If invoked with --uninstall, delegate to uninstall.exe and exit immediately
    if args.iter().any(|a| a == "--uninstall") {
        let uninst_exe = exe_dir.join("uninstall.exe");
        if uninst_exe.exists() {
            let mut cmd = std::process::Command::new(&uninst_exe);
            cmd.arg("--uninstall");
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            let _ = cmd.spawn();
        }
        return Ok(());
    }

    let data_dir = evergreen_core::env::resolve_data_directory(&exe_dir, &args);
    let _ = std::fs::create_dir_all(&data_dir);

    // Direct WebView2 runtime to store all cache, state, and profiles inside data_dir
    let wv2_data_dir = data_dir.join("webview2_data");
    let _ = std::fs::create_dir_all(&wv2_data_dir);
    std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &wv2_data_dir);

    if evergreen_core::env::is_portable_mode(&exe_dir, &args) {
        println!(
            "[PORTABLE MODE] Operating with zero system residue at: {:?}",
            data_dir
        );
    }

    // 3. Preflight runtime check
    let runtime_version = match detect_webview2_runtime() {
        Some(v) => v,
        None => {
            eprintln!("Microsoft WebView2 Runtime is required but was not found.");
            std::process::exit(1);
        }
    };
    println!("Detected Evergreen WebView2 Runtime: {}", runtime_version);

    // 4. Launch event loop
    let event_loop = EventLoop::<BrowserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = BrowserApp::new(proxy, runtime_version);
    event_loop.run_app(&mut app)?;

    Ok(())
}
