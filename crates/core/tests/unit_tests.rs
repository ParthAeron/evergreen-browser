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
        UiToHostMessage::MenuToggled { open: true },
        UiToHostMessage::ToggleMenuPanel,
        UiToHostMessage::ToggleSecurityPanel,
        UiToHostMessage::CloseSidebar,
        UiToHostMessage::OpenCertificateDialog { host: "github.com".to_string() },
        UiToHostMessage::PageNavigated { url: "https://test.com".to_string(), title: "Test".to_string() },
        UiToHostMessage::ReorderTab { from_index: 0, to_index: 2 },
        UiToHostMessage::DetachTabToNewWindow { tab_id: TabId(1) },
        UiToHostMessage::SetZoom { factor: 1.25 },
        UiToHostMessage::ZoomIn,
        UiToHostMessage::ZoomOut,
        UiToHostMessage::ZoomReset,
        UiToHostMessage::FindInPage { query: "rust".to_string(), forward: true },
        UiToHostMessage::CloseFindInPage,
        UiToHostMessage::OpenDownloads,
        UiToHostMessage::DownloadConfirm { download_id: 1, accept: true, save_path: Some("C:\\test.bin".to_string()) },
        UiToHostMessage::CancelDownload { download_id: 1 },
        UiToHostMessage::PermissionResponse { permission_id: 10, allow: true },
        UiToHostMessage::TriggerLinkPreview { url: "https://preview.com".to_string(), peek: false },
        UiToHostMessage::FindResult { current: 1, total: 5 },
        UiToHostMessage::OpenSettings,
        UiToHostMessage::SetSearchEngine { engine: "google".to_string() },
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
        HostToUiMessage::SidebarStateSync {
            open: true,
            mode: "menu".to_string(),
            security_info: Some(Box::new(evergreen_core::ipc::SecurityInfo {
                host: "github.com".to_string(),
                is_secure: true,
                protocol: "TLS 1.3".to_string(),
                certificate_status: "Valid".to_string(),
                cipher: "256-bit encryption (AES-GCM)".to_string(),
                subject: "CN=github.com".to_string(),
                issuer: "CN=Sectigo".to_string(),
                valid_from: "2026-01-01".to_string(),
                valid_to: "2027-01-01".to_string(),
                thumbprint: "D3B63E...".to_string(),
                serial_number: "00A5...".to_string(),
                signature_algorithm: "sha256ECDSA".to_string(),
            })),
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
        HostToUiMessage::SearchEngineSync {
            engine: "google".to_string(),
        },
        HostToUiMessage::ZoomSync {
            factor: 1.25,
        },
        HostToUiMessage::FindResult {
            current: 1,
            total: 5,
        },
        HostToUiMessage::DownloadPrompt {
            download_id: 42,
            filename: "installer.exe".to_string(),
            total_bytes: 1048576,
        },
        HostToUiMessage::DownloadProgress {
            download_id: 42,
            filename: "installer.exe".to_string(),
            received_bytes: 524288,
            total_bytes: 1048576,
            state: "InProgress".to_string(),
        },
        HostToUiMessage::PermissionPrompt {
            permission_id: 99,
            origin: "https://meet.google.com".to_string(),
            permission_kind: "Camera".to_string(),
        },
        HostToUiMessage::LinkPreviewReady {
            url: "https://news.ycombinator.com".to_string(),
            title: "Hacker News".to_string(),
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
    assert_eq!(settings.search_engine, "duckduckgo");
    assert_eq!(settings.search_url_template(), "https://duckduckgo.com/?q=%s");

    let serialized = serde_json::to_string_pretty(&settings).expect("Serialization failed");
    let deserialized: Settings = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(settings, deserialized);
}

#[test]
fn test_search_engine_templates() {
    let mut s = Settings::default();
    s.search_engine = "google".to_string();
    assert_eq!(s.search_url_template(), "https://www.google.com/search?q=%s");
    assert_eq!(s.search_engine_display_name(), "Google");

    s.search_engine = "bing".to_string();
    assert_eq!(s.search_url_template(), "https://www.bing.com/search?q=%s");

    s.search_engine = "brave".to_string();
    assert_eq!(s.search_url_template(), "https://search.brave.com/search?q=%s");

    s.search_engine = "ecosia".to_string();
    assert_eq!(s.search_url_template(), "https://www.ecosia.org/search?q=%s");
}

#[test]
fn test_tab_manager_suspension_and_audio() {
    let mut tm = TabManager::new();
    let tab_id = tm.create_tab("https://example.com", 1000);

    tm.set_audio_playing(tab_id, true);
    assert!(tm.tabs()[0].is_audio_playing);

    tm.set_tab_suspended(tab_id, true);
    assert_eq!(tm.tabs()[0].status, evergreen_core::tabs::TabStatus::Suspended);

    tm.set_tab_suspended(tab_id, false);
    assert_eq!(tm.tabs()[0].status, evergreen_core::tabs::TabStatus::Inactive);
}

#[test]
fn test_tab_manager_reorder() {
    let mut tm = TabManager::new();
    let id1 = tm.create_tab("https://tab1.com", 100);
    let id2 = tm.create_tab("https://tab2.com", 200);
    let id3 = tm.create_tab("https://tab3.com", 300);

    assert_eq!(tm.tabs()[0].id, id1);
    assert_eq!(tm.tabs()[1].id, id2);
    assert_eq!(tm.tabs()[2].id, id3);

    // Reorder: move tab at index 0 to index 2
    let reordered = tm.reorder_tab(0, 2);
    assert!(reordered);
    assert_eq!(tm.tabs()[0].id, id2);
    assert_eq!(tm.tabs()[1].id, id3);
    assert_eq!(tm.tabs()[2].id, id1);

    // Reorder back: move tab at index 2 to index 0
    let reordered2 = tm.reorder_tab(2, 0);
    assert!(reordered2);
    assert_eq!(tm.tabs()[0].id, id1);
    assert_eq!(tm.tabs()[1].id, id2);
    assert_eq!(tm.tabs()[2].id, id3);

    // Out of bounds returns false
    assert!(!tm.reorder_tab(0, 99));
    assert!(!tm.reorder_tab(99, 0));
    assert!(!tm.reorder_tab(1, 1));
}

#[test]
fn test_is_process_elevated_returns_false_unelevated() {
    // Under normal user runs or developer testing, this process is not elevated.
    let elevated = is_process_elevated();
    assert!(!elevated, "Process expected to run unelevated in standard environment");
}

