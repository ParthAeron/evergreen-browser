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
    /// Settings actions
    OpenSettings,
    SaveSettings { settings_json: String },
    RunEngineUpdate,
    RunForkUpdate,
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
}
