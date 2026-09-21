use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Application settings persisted to `%APPDATA%\<app>\settings.json` or local directory in portable mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Engine settings
    pub engine: EngineSettings,
    /// Privacy and ephemerality settings
    pub privacy: PrivacySettings,
    /// Performance and tab-sleep settings
    pub performance: PerformanceSettings,
    /// Appearance and UI theming
    pub appearance: AppearanceSettings,
    /// Default search engine ("duckduckgo", "google", "bing", "brave", "ecosia")
    pub search_engine: String,
    /// Download behavior
    pub downloads: DownloadSettings,
    /// Tab behavior and dragging
    pub tabs: TabSettings,
    /// Site permissions
    pub permissions: PermissionSettings,
    /// App update commands
    pub updates: UpdateSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSettings {
    /// Command to run when the user triggers "Update engine now"
    pub update_command: String,
    /// Automatically check for engine updates
    pub auto_check: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// If true, tabs run in ephemeral/private mode by default (no history or persistent cookies)
    pub ephemeral_default: bool,
    /// Domains explicitly allowed to retain cookies/session
    pub persistent_sites: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceSettings {
    /// Duration of inactivity in seconds before suspending a background tab (default: 300s = 5m)
    pub sleep_after_secs: u64,
    /// Target memory level for suspended tabs ("low", "normal")
    pub suspended_memory_target: String,
    /// Enable background tab CPU throttling
    pub background_throttling: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceSettings {
    /// Theme preference: "system", "light", "dark"
    pub theme: String,
    /// UI density: "comfortable", "compact"
    pub density: String,
    /// Default page zoom level (e.g. 1.0 = 100%)
    pub default_zoom_level: f64,
    /// Show zoom indicator badge on address bar when zoomed
    pub show_zoom_badge: bool,
    /// Enable interactive link preview card (Peek)
    pub enable_link_preview: bool,
    /// Show destination URL in bottom status bar on hover
    pub show_status_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadSettings {
    /// Prompt for save destination on each download
    pub ask_where_to_save: bool,
    /// Show download progress ring in toolbar
    pub show_progress_toolbar: bool,
    /// Default download folder
    pub default_folder: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabSettings {
    /// Enable tab drag-and-drop horizontal reordering
    pub enable_tab_reordering: bool,
    /// Enable detaching tab into a new window when dragged outside the tab strip
    pub enable_tab_tearoff: bool,
    /// Confirm before closing multiple tabs
    pub warn_on_close_tabs: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionSettings {
    /// Location permission policy: "ask", "allow", "block"
    pub location: String,
    /// Camera permission policy: "ask", "allow", "block"
    pub camera: String,
    /// Microphone permission policy: "ask", "allow", "block"
    pub microphone: String,
    /// Notifications permission policy: "ask", "allow", "block"
    pub notifications: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateSettings {
    /// Check for app updates on startup
    pub auto_check_app_updates: bool,
    /// Local rebuild/update command for developer and fork builds
    pub fork_update_command: String,
}

impl Default for Settings {
    fn default() -> Self {
        let default_download_dir = dirs_fallback_downloads();
        Self {
            engine: EngineSettings {
                update_command: "powershell -NoProfile -Command \"$u='https://go.microsoft.com/fwlink/p/?LinkId=2124703'; $o=\\\"$env:TEMP\\MicrosoftEdgeWebview2Setup.exe\\\"; Invoke-WebRequest -Uri $u -OutFile $o; Start-Process -FilePath $o -ArgumentList '/silent','/install' -Wait\"".to_string(),
                auto_check: true,
            },
            privacy: PrivacySettings {
                ephemeral_default: true,
                persistent_sites: Vec::new(),
            },
            performance: PerformanceSettings {
                sleep_after_secs: 300,
                suspended_memory_target: "low".to_string(),
                background_throttling: true,
            },
            appearance: AppearanceSettings {
                theme: "system".to_string(),
                density: "comfortable".to_string(),
                default_zoom_level: 1.0,
                show_zoom_badge: true,
                enable_link_preview: true,
                show_status_preview: true,
            },
            search_engine: "duckduckgo".to_string(),
            downloads: DownloadSettings {
                ask_where_to_save: true,
                show_progress_toolbar: true,
                default_folder: default_download_dir,
            },
            tabs: TabSettings {
                enable_tab_reordering: true,
                enable_tab_tearoff: true,
                warn_on_close_tabs: true,
            },
            permissions: PermissionSettings {
                location: "ask".to_string(),
                camera: "ask".to_string(),
                microphone: "ask".to_string(),
                notifications: "ask".to_string(),
            },
            updates: UpdateSettings {
                auto_check_app_updates: false,
                fork_update_command: "git pull && cargo build --release".to_string(),
            },
        }
    }
}

impl Settings {
    pub fn search_url_template(&self) -> &'static str {
        match self.search_engine.to_lowercase().as_str() {
            "google" => "https://www.google.com/search?q=%s",
            "bing" => "https://www.bing.com/search?q=%s",
            "brave" => "https://search.brave.com/search?q=%s",
            "ecosia" => "https://www.ecosia.org/search?q=%s",
            _ => "https://duckduckgo.com/?q=%s",
        }
    }

    pub fn search_engine_display_name(&self) -> &'static str {
        match self.search_engine.to_lowercase().as_str() {
            "google" => "Google",
            "bing" => "Bing",
            "brave" => "Brave",
            "ecosia" => "Ecosia",
            _ => "DuckDuckGo",
        }
    }
}

fn dirs_fallback_downloads() -> PathBuf {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile).join("Downloads")
    } else {
        PathBuf::from("Downloads")
    }
}

impl Settings {
    pub fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            let default_settings = Self::default();
            default_settings.save_to_path(path)?;
            return Ok(default_settings);
        }
        let content = std::fs::read_to_string(path)?;
        let settings = serde_json::from_str(&content)?;
        Ok(settings)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
