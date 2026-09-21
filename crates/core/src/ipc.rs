use crate::tabs::{TabId, TabState};
use serde::{Deserialize, Serialize};

/// Strongly-typed commands sent from Chrome UI (JS) to Rust host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum UiToHostMessage {
    /// Emitted when the chrome UI has finished loading and is ready for initial state
    ChromeReady,
    /// Request creating a new tab
    CreateTab { url: Option<String> },
    /// Request opening a new browser window
    OpenNewWindow,
    /// Switch active tab
    SwitchTab { id: TabId },
    /// Close an existing tab
    CloseTab { id: TabId },
    /// Navigate active tab to URL or search query
    Navigate { url: String },
    /// Browser navigation controls
    GoBack,
    GoForward,
    Reload,
    Stop,
    /// Open DevTools for current tab
    OpenDevTools,
    /// Open the native 3-dot popup menu at given coordinates
    OpenMenu { x: f64, y: f64 },
    /// Notify host that HTML 3-dot menu was opened or closed (for dynamic height expansion)
    MenuToggled { open: bool },
    /// Toggle the WinUI 3 slide-out sidebar in Menu mode
    ToggleMenuPanel,
    /// Toggle the WinUI 3 slide-out sidebar in Security/Certificate mode
    ToggleSecurityPanel,
    /// Request closing the sidebar panel
    CloseSidebar,
    /// Open the native Windows certificate dialog for the given host
    OpenCertificateDialog { host: String },
    /// Real-time client-side lifecycle navigation event (e.g. bfcache, popstate, pushState)
    PageNavigated { url: String, title: String },
    /// Tab manipulation
    /// Tab manipulation
    ReorderTab { from_index: usize, to_index: usize },
    DetachTabToNewWindow {
        tab_id: TabId,
        #[serde(default)]
        screen_x: Option<f64>,
        #[serde(default)]
        screen_y: Option<f64>,
    },
    /// Zoom actions
    SetZoom { factor: f64 },
    ZoomIn,
    ZoomOut,
    ZoomReset,
    /// Find in page
    OpenFindInPage,
    FindInPage { query: String, forward: bool },
    CloseFindInPage,
    /// Downloads
    OpenDownloads,
    ToggleDownloadsSidebar,
    DownloadConfirm { download_id: u64, accept: bool, save_path: Option<String> },
    CancelDownload { download_id: u64 },
    /// Site permissions
    OpenPermissionPrompt,
    ClosePermissionPrompt,
    PermissionResponse { permission_id: u64, allow: bool },
    /// Link preview
    TriggerLinkPreview {
        url: String,
        #[serde(default)]
        peek: bool,
    },
    /// Find in page result reported from active webview
    FindResult { current: usize, total: usize },
    /// Settings actions
    OpenSettings,
    SetSearchEngine { engine: String },
    SaveSettings { settings_json: String },
    RunEngineUpdate,
    RunForkUpdate,
}

/// Security and certificate status for the active origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityInfo {
    pub host: String,
    pub is_secure: bool,
    pub protocol: String,
    pub certificate_status: String,
    pub cipher: String,
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_to: String,
    pub thumbprint: String,
    pub serial_number: String,
    pub signature_algorithm: String,
}

impl Default for SecurityInfo {
    fn default() -> Self {
        Self {
            host: String::new(),
            is_secure: false,
            protocol: "Unknown".to_string(),
            certificate_status: "Unknown".to_string(),
            cipher: "Unknown".to_string(),
            subject: String::new(),
            issuer: String::new(),
            valid_from: String::new(),
            valid_to: String::new(),
            thumbprint: String::new(),
            serial_number: String::new(),
            signature_algorithm: String::new(),
        }
    }
}

/// Strongly-typed state events sent from Rust host to Chrome UI (JS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum HostToUiMessage {
    /// Full tab state update
    TabStateSync {
        tabs: Vec<TabState>,
        active_tab_id: Option<TabId>,
    },
    /// Sidebar panel state sync
    SidebarStateSync {
        open: bool,
        mode: String,
        security_info: Option<Box<SecurityInfo>>,
    },
    /// Active tab URL / navigation update
    NavigationUpdated {
        tab_id: TabId,
        url: String,
        can_go_back: bool,
        can_go_forward: bool,
        is_loading: bool,
    },
    /// Status message or engine update output
    CommandOutput {
        command: String,
        success: bool,
        output: String,
    },
    /// Engine version info
    EngineInfoSync {
        version: String,
        is_update_available: bool,
    },
    /// Default search engine sync
    SearchEngineSync {
        engine: String,
    },
    /// Zoom level sync
    ZoomSync {
        factor: f64,
    },
    /// Find in page search results
    FindResult {
        current: usize,
        total: usize,
    },
    /// Download confirmation prompt
    DownloadPrompt {
        download_id: u64,
        filename: String,
        total_bytes: i64,
    },
    /// Download progress update
    DownloadProgress {
        download_id: u64,
        filename: String,
        received_bytes: i64,
        total_bytes: i64,
        state: String,
    },
    /// Permission prompt request
    PermissionPrompt {
        permission_id: u64,
        origin: String,
        permission_kind: String,
    },
    /// Link preview result
    LinkPreviewReady {
        url: String,
        title: String,
    },
}
