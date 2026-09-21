# Extending Evergreen Browser with Plugins

Evergreen Browser provides a modular plugin system through the `BrowserPlugin` trait in `crates/core/src/plugins.rs`. Plugins can contribute custom toolbar buttons, sidebar panels, content scripts, and IPC message handlers without requiring changes to the core tab manager.

---

## 1. The `BrowserPlugin` Trait

```rust
pub trait BrowserPlugin: Send + Sync {
    /// Returns metadata describing plugin identity and defaults.
    fn metadata(&self) -> PluginMetadata;

    /// Evaluates whether the plugin is currently active based on user settings.
    fn is_enabled(&self, settings: &Settings) -> bool;

    /// Optional HTML snippet injected into the chrome navigation bar.
    fn toolbar_item_html(&self) -> Option<&'static str> { None }

    /// Optional sidebar registration: (mode_identifier, panel_html).
    fn sidepanel_view(&self) -> Option<(&'static str, &'static str)> { None }

    /// Optional JavaScript content script injected into web pages.
    fn content_script(&self) -> Option<&'static str> { None }

    /// Handles custom typed IPC actions sent from web views.
    fn handle_ipc(&self, action: &str, payload: &serde_json::Value) -> Option<serde_json::Value> { None }
}
```

---

## 2. Working Examples

### Example 1: Custom Toolbar Button

This plugin adds a custom action button to the browser's top navigation bar:

```rust
use evergreen_core::plugins::{BrowserPlugin, PluginMetadata};
use evergreen_core::settings::Settings;

pub struct ThemeTogglePlugin;

impl BrowserPlugin for ThemeTogglePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "theme_toggle".to_string(),
            name: "Theme Toggle".to_string(),
            version: "1.0.0".to_string(),
            description: "Adds a quick theme toggle button to the toolbar".to_string(),
            author: "Contributor".to_string(),
            is_core: false,
            enabled_by_default: true,
        }
    }

    fn is_enabled(&self, settings: &Settings) -> bool {
        // Tie enablement to a feature flag or keep always active
        true
    }

    fn toolbar_item_html(&self) -> Option<&'static str> {
        Some(r#"
            <button class="nav-btn" id="btnThemeToggle" title="Toggle Theme" onclick="toggleCustomTheme()">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="5"></circle>
                    <line x1="12" y1="1" x2="12" y2="3"></line>
                    <line x1="12" y1="21" x2="12" y2="23"></line>
                </svg>
            </button>
        "#)
    }
}
```

### Example 2: Scratchpad Sidebar Panel

This plugin registers a custom sidepanel drawer mode:

```rust
use evergreen_core::plugins::{BrowserPlugin, PluginMetadata};
use evergreen_core::settings::Settings;

pub struct NotesSidebarPlugin;

impl BrowserPlugin for NotesSidebarPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "notes_sidebar".to_string(),
            name: "Scratchpad Notes".to_string(),
            version: "1.0.0".to_string(),
            description: "Adds a simple scratchpad in the sidebar".to_string(),
            author: "Contributor".to_string(),
            is_core: false,
            enabled_by_default: true,
        }
    }

    fn is_enabled(&self, _settings: &Settings) -> bool {
        true
    }

    fn sidepanel_view(&self) -> Option<(&'static str, &'static str)> {
        Some((
            "notes",
            r#"
            <div style="padding: 16px; color: #f1f5f9; font-family: sans-serif;">
                <h3 style="margin-top:0; font-size:14px;">Scratchpad</h3>
                <textarea style="width:100%; height:300px; background:#1e1e2c; color:#fff; border:1px solid #333; border-radius:6px; padding:8px;"></textarea>
            </div>
            "#,
        ))
    }
}
```

### Example 3: Content Script Injection

This plugin injects JavaScript into all loaded web pages:

```rust
use evergreen_core::plugins::{BrowserPlugin, PluginMetadata};
use evergreen_core::settings::Settings;

pub struct ContentScriptPlugin;

impl BrowserPlugin for ContentScriptPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "custom_styles".to_string(),
            name: "Page Tweaks".to_string(),
            version: "1.0.0".to_string(),
            description: "Injects page customization scripts".to_string(),
            author: "Contributor".to_string(),
            is_core: false,
            enabled_by_default: true,
        }
    }

    fn is_enabled(&self, _settings: &Settings) -> bool {
        true
    }

    fn content_script(&self) -> Option<&'static str> {
        Some(r#"
            console.log('[Evergreen Plugin] Content script initialized on:', window.location.href);
        "#)
    }
}
```

---

## 3. Registering Plugins

Register plugins with the `PluginRegistry` during application startup in `crates/app/src/main.rs`:

```rust
let mut plugins = PluginRegistry::new();
plugins.register(ThemeTogglePlugin);
plugins.register(NotesSidebarPlugin);
plugins.register(ContentScriptPlugin);
```

On startup, the shell queries `plugins.enabled_plugins(&settings)` to collect toolbar HTML, content scripts, and sidepanel views.

