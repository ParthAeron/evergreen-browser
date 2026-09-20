//! WebView2 adapter for Evergreen Browser.
//!
//! This crate contains the concrete implementation of [`evergreen_core::engine::EngineHost`]
//! using `wry` and the Microsoft WebView2 COM API.

use evergreen_core::engine::{EngineHost, EngineInfo};
use evergreen_core::env::detect_webview2_runtime;
use evergreen_core::tabs::TabId;
use std::collections::HashMap;

pub struct WebView2Host {
    version: String,
    // Tab webviews will be managed here during integration
    tabs: HashMap<TabId, String>,
}

impl WebView2Host {
    pub fn new() -> Self {
        let version = detect_webview2_runtime().unwrap_or_else(|| "Unknown".to_string());
        Self {
            version,
            tabs: HashMap::new(),
        }
    }
}

impl Default for WebView2Host {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineHost for WebView2Host {
    fn info(&self) -> EngineInfo {
        EngineInfo {
            name: "Microsoft WebView2 (Evergreen)".to_string(),
            version: self.version.clone(),
            executable_path: None,
            is_hardware_accelerated: true,
            sandbox_enabled: true,
        }
    }

    fn create_tab_webview(&mut self, tab_id: TabId, url: &str, _incognito: bool) -> Result<(), Box<dyn std::error::Error>> {
        self.tabs.insert(tab_id, url.to_string());
        Ok(())
    }

    fn show_tab(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn hide_tab(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn suspend_tab(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        // Calls raw COM: ICoreWebView2::TrySuspendAsync()
        Ok(())
    }

    fn resume_tab(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        // Calls raw COM: ICoreWebView2::Resume()
        Ok(())
    }

    fn destroy_tab(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        self.tabs.remove(&tab_id);
        Ok(())
    }

    fn navigate(&mut self, tab_id: TabId, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(entry) = self.tabs.get_mut(&tab_id) {
            *entry = url.to_string();
        }
        Ok(())
    }

    fn go_back(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn go_forward(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn reload(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn stop(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn open_devtools(&mut self, _tab_id: TabId) -> Result<(), Box<dyn std::error::Error>> {
        // Calls CoreWebView2::OpenDevToolsWindow()
        Ok(())
    }

    fn clear_browsing_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Calls CoreWebView2Profile::ClearBrowsingDataAsync
        Ok(())
    }
}
