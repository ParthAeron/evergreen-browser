use evergreen_core::env::is_process_elevated;
use evergreen_core::ipc::{HostToUiMessage, UiToHostMessage};
use evergreen_core::settings::Settings;
use evergreen_core::tabs::{TabId, TabManager, TabStatus};

#[test]
fn test_tab_manager_create_and_transitions() {
    let mut manager = TabManager::new();

    // Initial state
    assert!(manager.active_tab().is_none());
    assert_eq!(manager.tabs().len(), 0);

    // Create first tab
    let id1 = manager.create_tab("https://example.com", 100);
    assert_eq!(manager.active_tab().unwrap().id, id1);
    assert_eq!(manager.active_tab().unwrap().status, TabStatus::Active);
    assert_eq!(manager.tabs().len(), 1);

    // Create second tab -> id1 becomes Inactive, id2 becomes Active
    let id2 = manager.create_tab("https://rust-lang.org", 200);
    assert_eq!(manager.active_tab().unwrap().id, id2);
    assert_eq!(manager.active_tab().unwrap().status, TabStatus::Active);

    let tab1 = manager.tabs().iter().find(|t| t.id == id1).unwrap();
    assert_eq!(tab1.status, TabStatus::Inactive);
}

#[test]
fn test_tab_manager_switch_tab() {
    let mut manager = TabManager::new();
    let id1 = manager.create_tab("https://tab1.com", 100);
    let id2 = manager.create_tab("https://tab2.com", 200);

    assert_eq!(manager.active_tab().unwrap().id, id2);

    // Switch back to tab 1
    let success = manager.switch_tab(id1, 300);
    assert!(success);
    assert_eq!(manager.active_tab().unwrap().id, id1);
    assert_eq!(manager.active_tab().unwrap().status, TabStatus::Active);

    // Tab 2 should now be Inactive
    let tab2 = manager.tabs().iter().find(|t| t.id == id2).unwrap();
    assert_eq!(tab2.status, TabStatus::Inactive);
}

#[test]
fn test_tab_manager_close_last_tab_returns_none() {
    let mut manager = TabManager::new();
    let id1 = manager.create_tab("https://example.com", 100);

    let next_active = manager.close_tab(id1);
    assert!(next_active.is_none());
    assert!(manager.active_tab().is_none());
    assert_eq!(manager.tabs().len(), 0);
}

#[test]
fn test_tab_manager_pop_last_closed_lifo_and_skips_blank() {
    let mut manager = TabManager::new();

    let id_blank = manager.create_tab("about:blank", 100);
    let id_site1 = manager.create_tab("https://first.com", 101);
    let id_site2 = manager.create_tab("https://second.com", 102);

    // Close in order: blank, site1, site2
    manager.close_tab(id_blank);
    manager.close_tab(id_site1);
    manager.close_tab(id_site2);

    // Pop should return site2 first (LIFO), then site1, then None (about:blank skipped)
    assert_eq!(manager.pop_last_closed(), Some("https://second.com".to_string()));
    assert_eq!(manager.pop_last_closed(), Some("https://first.com".to_string()));
    assert_eq!(manager.pop_last_closed(), None);
}

#[test]
fn test_ipc_serde_ui_to_host_all_variants() {
    let variants = vec![
        UiToHostMessage::ChromeReady,
        UiToHostMessage::CreateTab { url: Some("https://test.com".to_string()) },
        UiToHostMessage::CreateTab { url: None },
        UiToHostMessage::OpenNewWindow,
        UiToHostMessage::SwitchTab { id: TabId(42) },
        UiToHostMessage::CloseTab { id: TabId(7) },
        UiToHostMessage::Navigate { url: "https://nav.com".to_string() },
        UiToHostMessage::GoBack,
        UiToHostMessage::GoForward,
        UiToHostMessage::Reload,
        UiToHostMessage::Stop,
        UiToHostMessage::OpenDevTools,
        UiToHostMessage::OpenMenu { x: 100.0, y: 50.0 },
        UiToHostMessage::OpenSettings,
        UiToHostMessage::SaveSettings { settings_json: "{}".to_string() },
        UiToHostMessage::RunEngineUpdate,
        UiToHostMessage::RunForkUpdate,
    ];

    for msg in variants {
        let serialized = serde_json::to_string(&msg).expect("Failed to serialize UiToHostMessage");
        let deserialized: UiToHostMessage = serde_json::from_str(&serialized)
            .expect("Failed to deserialize UiToHostMessage");
        assert_eq!(msg, deserialized);
    }
}

#[test]
fn test_ipc_serde_host_to_ui_all_variants() {
    let variants = vec![
        HostToUiMessage::TabStateSync {
            tabs: vec![],
            active_tab_id: Some(TabId(1)),
        },
        HostToUiMessage::NavigationUpdated {
            tab_id: TabId(1),
            url: "https://example.com".to_string(),
            can_go_back: true,
            can_go_forward: false,
            is_loading: false,
        },
        HostToUiMessage::CommandOutput {
            command: "update".to_string(),
            success: true,
            output: "Installed successfully".to_string(),
        },
        HostToUiMessage::EngineInfoSync {
            version: "153.0.4234.32".to_string(),
            is_update_available: false,
        },
    ];

    for msg in variants {
        let serialized = serde_json::to_string(&msg).expect("Failed to serialize HostToUiMessage");
        let deserialized: HostToUiMessage = serde_json::from_str(&serialized)
            .expect("Failed to deserialize HostToUiMessage");
        assert_eq!(msg, deserialized);
    }
}

#[test]
fn test_settings_default_and_roundtrip() {
    let settings = Settings::default();
    assert!(settings.privacy.ephemeral_default);
    assert_eq!(settings.performance.sleep_after_secs, 300);

    let serialized = serde_json::to_string_pretty(&settings).expect("Serialization failed");
    let deserialized: Settings = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(settings, deserialized);
}

#[test]
fn test_is_process_elevated_returns_false_unelevated() {
    // Under normal user runs or developer testing, this process is not elevated.
    let elevated = is_process_elevated();
    assert!(!elevated, "Process expected to run unelevated in standard environment");
}

