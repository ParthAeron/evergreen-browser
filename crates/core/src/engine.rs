use crate::tabs::TabId;
use serde::{Deserialize, Serialize};

/// Basic information about the rendering engine discovered on the host system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    pub name: String,
    pub version: String,
    pub executable_path: Option<String>,
    pub is_hardware_accelerated: bool,
    pub sandbox_enabled: bool,
}

/// Generic interface implemented by any rendering backend (e.g. WebView2, or alternative future engines).
///
/// This trait ensures the core browser logic remains completely decoupled from specific COM APIs or webview libraries.
pub trait EngineHost {
    /// Return engine name and runtime version string
    fn info(&self) -> EngineInfo;

    /// Create and attach a child webview for a given tab
    fn create_tab_webview(&mut self, tab_id: TabId, url: &str, incognito: bool) -> Result<(), Box<dyn std::error::Error>>;

    /// Show and focus a tab's webview
    fn show_tab(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Hide a tab's webview
    fn hide_tab(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Suspend an inactive tab's webview to free memory
    fn suspend_tab(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Resume a suspended tab's webview
    fn resume_tab(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Close and destroy a tab's webview
    fn destroy_tab(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Navigate the specified tab
    fn navigate(&mut self, tab_id: TabId, url: &str) -> Result<(), Box<dyn std::error::Error>>;

    /// Go back in navigation history
    fn go_back(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Go forward in navigation history
    fn go_forward(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Reload current tab
    fn reload(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Stop loading
    fn stop(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Open DevTools for the active tab
    fn open_devtools(&mut self, tab_id: TabId) -> Result<(), Box<dyn std::error::Error>>;

    /// Clear all browsing data (cache, cookies, indexedDB)
    fn clear_browsing_data(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
