use serde::{Deserialize, Serialize};

/// Unique identifier for an open browser tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub u64);

/// Current lifecycle status of a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabStatus {
    /// Actively visible in the main viewport
    Active,
    /// In background, rendered but not visible
    Inactive,
    /// Suspended via TrySuspendAsync to reclaim memory
    Suspended,
    /// Encountered a crash/render failure
    Crashed,
}

/// Metadata and state for an individual tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabState {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub favicon_uri: Option<String>,
    pub is_loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_audio_playing: bool,
    pub is_muted: bool,
    pub status: TabStatus,
    pub last_active_timestamp_secs: u64,
}

/// Pure state manager for all tabs inside a single window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabManager {
    tabs: Vec<TabState>,
    active_tab_id: Option<TabId>,
    next_id: u64,
    closed_history: Vec<String>, // URL history of closed tabs for Ctrl+Shift+T (in-memory only)
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_id: None,
            next_id: 1,
            closed_history: Vec::new(),
        }
    }

    pub fn create_tab(&mut self, url: &str, current_time_secs: u64) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;

        let new_tab = TabState {
            id,
            url: url.to_string(),
            title: "New Tab".to_string(),
            favicon_uri: None,
            is_loading: false,
            can_go_back: false,
            can_go_forward: false,
            is_audio_playing: false,
            is_muted: false,
            status: TabStatus::Active,
            last_active_timestamp_secs: current_time_secs,
        };

        // If another tab was active, mark it inactive
        if let Some(prev_active_id) = self.active_tab_id {
            if let Some(prev) = self.tabs.iter_mut().find(|t| t.id == prev_active_id) {
                prev.status = TabStatus::Inactive;
            }
        }

        self.tabs.push(new_tab);
        self.active_tab_id = Some(id);
        id
    }

    pub fn close_tab(&mut self, id: TabId) -> Option<TabId> {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            let removed = self.tabs.remove(pos);
            if !removed.url.is_empty() && removed.url != "about:blank" {
                self.closed_history.push(removed.url);
            }

            if self.active_tab_id == Some(id) {
                if self.tabs.is_empty() {
                    self.active_tab_id = None;
                } else {
                    let next_pos = if pos >= self.tabs.len() {
                        self.tabs.len() - 1
                    } else {
                        pos
                    };
                    let next_id = self.tabs[next_pos].id;
                    self.tabs[next_pos].status = TabStatus::Active;
                    self.active_tab_id = Some(next_id);
                }
            }
        }
        self.active_tab_id
    }

    pub fn switch_tab(&mut self, id: TabId, current_time_secs: u64) -> bool {
        let exists = self.tabs.iter().any(|t| t.id == id);
        if !exists {
            return false;
        }

        for tab in &mut self.tabs {
            if tab.id == id {
                tab.status = TabStatus::Active;
                tab.last_active_timestamp_secs = current_time_secs;
            } else if tab.status == TabStatus::Active {
                tab.status = TabStatus::Inactive;
            }
        }
        self.active_tab_id = Some(id);
        true
    }

    pub fn active_tab(&self) -> Option<&TabState> {
        self.active_tab_id
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
    }

    pub fn tabs(&self) -> &[TabState] {
        &self.tabs
    }

    pub fn update_title(&mut self, id: TabId, title: String) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.title = title;
        }
    }

    pub fn update_url(&mut self, id: TabId, url: String) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.url = url;
        }
    }

    pub fn update_favicon(&mut self, id: TabId, favicon_uri: Option<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.favicon_uri = favicon_uri;
        }
    }

    pub fn pop_last_closed(&mut self) -> Option<String> {
        self.closed_history.pop()
    }
}

