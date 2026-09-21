//! Pluggable Feature Architecture for Evergreen Browser.
//!
//! Provides the [`BrowserPlugin`] trait and [`PluginRegistry`] to allow both
//! built-in feature modules (Find-in-Page, Downloads, Link Preview, Tab Gestures, Zoom, Permissions)
//! and third-party fork developers to extend the browser without modifying core internals.

use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Metadata describing a browser plugin or feature module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub is_core: bool,
    pub enabled_by_default: bool,
}

/// The extensible plugin trait implemented by first-party modules and fork extensions.
pub trait BrowserPlugin: Send + Sync {
    /// Returns metadata describing this plugin.
    fn metadata(&self) -> PluginMetadata;

    /// Checks if this plugin is enabled according to current user settings.
    fn is_enabled(&self, settings: &Settings) -> bool;

    /// Optional HTML markup injected into the chrome navigation bar.
    fn toolbar_item_html(&self) -> Option<&'static str> {
        None
    }

    /// Optional side-panel registration: `(mode_name, template_html)`.
    fn sidepanel_view(&self) -> Option<(&'static str, &'static str)> {
        None
    }

    /// Optional JavaScript content script injected into web page tabs.
    fn content_script(&self) -> Option<&'static str> {
        None
    }

    /// Process an IPC message. Returns `Some(response)` if handled.
    fn handle_ipc(&self, _action: &str, _payload: &serde_json::Value) -> Option<serde_json::Value> {
        None
    }
}

/// Central registry managing active and available browser plugins.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Arc<dyn BrowserPlugin>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin with the registry.
    pub fn register<P: BrowserPlugin + 'static>(&mut self, plugin: P) {
        self.plugins.push(Arc::new(plugin));
    }

    /// Register an Arc-wrapped plugin.
    pub fn register_arc(&mut self, plugin: Arc<dyn BrowserPlugin>) {
        self.plugins.push(plugin);
    }

    /// Return all registered plugins.
    pub fn plugins(&self) -> &[Arc<dyn BrowserPlugin>] {
        &self.plugins
    }

    /// Find a plugin by its unique ID.
    pub fn get(&self, id: &str) -> Option<Arc<dyn BrowserPlugin>> {
        self.plugins.iter().find(|p| p.metadata().id == id).cloned()
    }

    /// Return list of all enabled plugins based on current settings.
    pub fn enabled_plugins(&self, settings: &Settings) -> Vec<Arc<dyn BrowserPlugin>> {
        self.plugins
            .iter()
            .filter(|p| p.is_enabled(settings))
            .cloned()
            .collect()
    }

    /// Aggregate all content scripts from enabled plugins into a combined script.
    pub fn combined_content_scripts(&self, settings: &Settings) -> String {
        let mut script = String::new();
        for p in self.enabled_plugins(settings) {
            if let Some(cs) = p.content_script() {
                script.push_str("\n/* Plugin: ");
                script.push_str(&p.metadata().id);
                script.push_str(" */\n");
                script.push_str(cs);
                script.push('\n');
            }
        }
        script
    }
}

/// A lightweight plugin implementation wrapping metadata and an enablement predicate.
#[derive(Clone)]
pub struct FeaturePlugin {
    pub metadata: PluginMetadata,
    pub is_enabled_fn: fn(&Settings) -> bool,
    pub content_script_fn: Option<fn() -> Option<&'static str>>,
}

impl FeaturePlugin {
    pub fn new(metadata: PluginMetadata, is_enabled_fn: fn(&Settings) -> bool) -> Self {
        Self {
            metadata,
            is_enabled_fn,
            content_script_fn: None,
        }
    }
}

impl BrowserPlugin for FeaturePlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn is_enabled(&self, settings: &Settings) -> bool {
        (self.is_enabled_fn)(settings)
    }

    fn content_script(&self) -> Option<&'static str> {
        self.content_script_fn.and_then(|f| f())
    }
}
