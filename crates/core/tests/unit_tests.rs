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
    assert_eq!(
        manager.pop_last_closed(),
        Some("https://second.com".to_string())
    );
    assert_eq!(
        manager.pop_last_closed(),
        Some("https://first.com".to_string())
    );
    assert_eq!(manager.pop_last_closed(), None);
}

#[test]
fn test_ipc_serde_ui_to_host_all_variants() {
    let variants = vec![
        UiToHostMessage::ChromeReady,
        UiToHostMessage::CreateTab {
            url: Some("https://test.com".to_string()),
        },
        UiToHostMessage::CreateTab { url: None },
        UiToHostMessage::OpenNewWindow,
        UiToHostMessage::SwitchTab { id: TabId(42) },
        UiToHostMessage::CloseTab { id: TabId(7) },
        UiToHostMessage::Navigate {
            url: "https://nav.com".to_string(),
        },
        UiToHostMessage::GoBack,
        UiToHostMessage::GoForward,
        UiToHostMessage::Reload,
        UiToHostMessage::ReloadTab { tab_id: TabId(7) },
        UiToHostMessage::Stop,
        UiToHostMessage::OpenDevTools,
        UiToHostMessage::OpenMenu { x: 100.0, y: 50.0 },
        UiToHostMessage::MenuToggled { open: true },
        UiToHostMessage::ToggleMenuPanel,
        UiToHostMessage::ToggleSecurityPanel,
        UiToHostMessage::CloseSidebar,
        UiToHostMessage::OpenCertificateDialog {
            host: "github.com".to_string(),
        },
        UiToHostMessage::PageNavigated {
            url: "https://test.com".to_string(),
            title: "Test".to_string(),
        },
        UiToHostMessage::ReorderTab {
            from_index: 0,
            to_index: 2,
        },
        UiToHostMessage::DetachTabToNewWindow {
            tab_id: TabId(1),
            screen_x: Some(100.0),
            screen_y: Some(100.0),
        },
        UiToHostMessage::OpenFindInPage,
        UiToHostMessage::SetZoom { factor: 1.25 },
        UiToHostMessage::ZoomIn,
        UiToHostMessage::ZoomOut,
        UiToHostMessage::ZoomReset,
        UiToHostMessage::FindInPage {
            query: "rust".to_string(),
            forward: true,
        },
        UiToHostMessage::CloseFindInPage,
        UiToHostMessage::OpenDownloads,
        UiToHostMessage::ToggleDownloadsSidebar,
        UiToHostMessage::DownloadConfirm {
            download_id: 1,
            accept: true,
            save_path: Some("C:\\test.bin".to_string()),
        },
        UiToHostMessage::CancelDownload { download_id: 1 },
        UiToHostMessage::OpenPermissionPrompt,
        UiToHostMessage::ClosePermissionPrompt,
        UiToHostMessage::PermissionResponse {
            permission_id: 10,
            allow: true,
        },
        UiToHostMessage::TriggerLinkPreview {
            url: "https://preview.com".to_string(),
            peek: false,
        },
        UiToHostMessage::FindResult {
            current: 1,
            total: 5,
        },
        UiToHostMessage::OpenSettings,
        UiToHostMessage::SetSearchEngine {
            engine: "google".to_string(),
        },
        UiToHostMessage::SaveSettings {
            settings_json: "{}".to_string(),
        },
        UiToHostMessage::RunEngineUpdate,
        UiToHostMessage::RunForkUpdate,
        UiToHostMessage::BypassCertificateError {
            tab_id: TabId(1),
            host: "badssl.com".to_string(),
            url: "https://badssl.com/".to_string(),
        },
    ];

    for msg in variants {
        let serialized = serde_json::to_string(&msg).expect("Failed to serialize UiToHostMessage");
        let deserialized: UiToHostMessage =
            serde_json::from_str(&serialized).expect("Failed to deserialize UiToHostMessage");
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
        HostToUiMessage::TabCrashed { tab_id: TabId(1) },
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
        HostToUiMessage::ZoomSync { factor: 1.25 },
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
        let deserialized: HostToUiMessage =
            serde_json::from_str(&serialized).expect("Failed to deserialize HostToUiMessage");
        assert_eq!(msg, deserialized);
    }
}

#[test]
fn test_settings_default_and_roundtrip() {
    let settings = Settings::default();
    assert!(settings.privacy.ephemeral_default);
    assert_eq!(settings.performance.sleep_after_secs, 300);
    assert_eq!(settings.search_engine, "duckduckgo");
    assert_eq!(
        settings.search_url_template(),
        "https://duckduckgo.com/?q=%s"
    );

    let serialized = serde_json::to_string_pretty(&settings).expect("Serialization failed");
    let deserialized: Settings = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(settings, deserialized);
}

#[test]
fn test_search_engine_templates() {
    let mut s = Settings {
        search_engine: "google".to_string(),
        ..Default::default()
    };
    assert_eq!(
        s.search_url_template(),
        "https://www.google.com/search?q=%s"
    );
    assert_eq!(s.search_engine_display_name(), "Google");

    s.search_engine = "bing".to_string();
    assert_eq!(s.search_url_template(), "https://www.bing.com/search?q=%s");

    s.search_engine = "brave".to_string();
    assert_eq!(
        s.search_url_template(),
        "https://search.brave.com/search?q=%s"
    );

    s.search_engine = "ecosia".to_string();
    assert_eq!(
        s.search_url_template(),
        "https://www.ecosia.org/search?q=%s"
    );
}

#[test]
fn test_tab_manager_suspension_and_audio() {
    let mut tm = TabManager::new();
    let tab_id = tm.create_tab("https://example.com", 1000);

    tm.set_audio_playing(tab_id, true);
    assert!(tm.tabs()[0].is_audio_playing);

    tm.set_tab_suspended(tab_id, true);
    assert_eq!(
        tm.tabs()[0].status,
        evergreen_core::tabs::TabStatus::Suspended
    );

    tm.set_tab_suspended(tab_id, false);
    assert_eq!(
        tm.tabs()[0].status,
        evergreen_core::tabs::TabStatus::Inactive
    );
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
    assert!(
        !elevated,
        "Process expected to run unelevated in standard environment"
    );
}

#[test]
fn test_settings_update_from_partial_json() {
    let mut s = Settings::default();
    assert!(s.downloads.ask_where_to_save);
    assert_eq!(s.search_engine, "duckduckgo");
    assert!(s.features.enable_find_in_page);

    let partial = serde_json::json!({
        "search_engine": "google",
        "askWhereToSave": false,
        "enable_find_in_page": false
    });
    s.update_from_json(&partial);

    assert_eq!(s.search_engine, "google");
    assert!(!s.downloads.ask_where_to_save);
    assert!(!s.features.enable_find_in_page);
    // Other settings untouched
    assert!(s.features.enable_downloads_manager);
    assert_eq!(s.appearance.default_zoom_level, 1.0);
}

#[test]
fn test_tab_manager_extract_and_insert() {
    let mut tm1 = TabManager::new();
    let id1 = tm1.create_tab("https://window1-tab1.com", 100);
    let id2 = tm1.create_tab("https://window1-tab2.com", 200);

    assert_eq!(tm1.tabs().len(), 2);
    assert_eq!(tm1.active_tab_id(), Some(id2));

    // Extract tab 1
    let extracted = tm1.extract_tab(id1).expect("Failed to extract tab");
    assert_eq!(extracted.url, "https://window1-tab1.com");
    assert_eq!(tm1.tabs().len(), 1);
    assert_eq!(tm1.active_tab_id(), Some(id2));

    // Insert into another tab manager
    let mut tm2 = TabManager::new();
    let id_in_tm2 = tm2.insert_tab(extracted, None);
    assert_eq!(tm2.tabs().len(), 1);
    assert_eq!(tm2.tabs()[0].url, "https://window1-tab1.com");
    assert_eq!(tm2.active_tab_id(), Some(id_in_tm2));
}

#[test]
fn test_plugin_registry_and_extensibility() {
    use evergreen_core::plugins::{BrowserPlugin, PluginMetadata, PluginRegistry};

    struct TestAdBlockPlugin;
    impl BrowserPlugin for TestAdBlockPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                id: "adblock_test".to_string(),
                name: "Test AdBlocker".to_string(),
                description: "Test plugin for custom forks".to_string(),
                version: "1.0.0".to_string(),
                author: "ForkDev".to_string(),
                is_core: false,
                enabled_by_default: true,
            }
        }
        fn is_enabled(&self, _settings: &Settings) -> bool {
            true
        }
        fn content_script(&self) -> Option<&'static str> {
            Some("console.log('AdBlock active');")
        }
    }

    let mut registry = PluginRegistry::new();
    registry.register(TestAdBlockPlugin);

    assert_eq!(registry.plugins().len(), 1);
    let meta = registry.plugins()[0].metadata();
    assert_eq!(meta.id, "adblock_test");
    assert_eq!(meta.name, "Test AdBlocker");

    let settings = Settings::default();
    let enabled = registry.enabled_plugins(&settings);
    assert_eq!(enabled.len(), 1);

    let script = registry.combined_content_scripts(&settings);
    assert!(script.contains("AdBlock active"));
}

#[test]
fn test_portable_mode_detection_and_data_directory() {
    use evergreen_core::env::{is_portable_mode, resolve_data_directory};

    let temp_dir = std::env::temp_dir().join(format!(
        "evergreen_test_portable_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Default: false without args
    assert!(!is_portable_mode(&temp_dir, &[]));
    assert!(!is_portable_mode(&temp_dir, &["--other-flag".to_string()]));

    // Portable flag in args
    assert!(is_portable_mode(&temp_dir, &["--portable".to_string()]));
    let dir = resolve_data_directory(&temp_dir, &["--portable".to_string()]);
    assert_eq!(dir, temp_dir.join("user_data"));

    // portable.lock file
    let lock_file = temp_dir.join("portable.lock");
    std::fs::write(&lock_file, "").unwrap();
    assert!(is_portable_mode(&temp_dir, &[]));
    let dir_lock = resolve_data_directory(&temp_dir, &[]);
    assert_eq!(dir_lock, temp_dir.join("user_data"));
    let _ = std::fs::remove_file(&lock_file);

    // portable.ini file
    let ini_file = temp_dir.join("portable.ini");
    std::fs::write(&ini_file, "").unwrap();
    assert!(is_portable_mode(&temp_dir, &[]));
    let _ = std::fs::remove_file(&ini_file);

    // user_data directory
    let user_data_dir = temp_dir.join("user_data");
    std::fs::create_dir_all(&user_data_dir).unwrap();
    assert!(is_portable_mode(&temp_dir, &[]));
    assert_eq!(
        resolve_data_directory(&temp_dir, &[]),
        temp_dir.join("user_data")
    );
    let _ = std::fs::remove_dir_all(&user_data_dir);

    // data directory
    let data_dir = temp_dir.join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    assert!(is_portable_mode(&temp_dir, &[]));
    assert_eq!(
        resolve_data_directory(&temp_dir, &[]),
        temp_dir.join("data")
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_tab_manager_crash_and_snapshot_restore() {
    let mut tm = TabManager::new();
    let id1 = tm.create_tab("https://alpha.example", 100);
    let _id2 = tm.create_tab("https://beta.example", 200);

    assert_eq!(
        tm.tabs()[0].status,
        evergreen_core::tabs::TabStatus::Inactive
    );
    assert_eq!(tm.tabs()[1].status, evergreen_core::tabs::TabStatus::Active);

    // Mark tab 1 crashed
    tm.mark_tab_crashed(id1);
    assert_eq!(
        tm.tabs()[0].status,
        evergreen_core::tabs::TabStatus::Crashed
    );

    // Snapshot
    let snapshot = tm.snapshot();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].url, "https://alpha.example");
    assert_eq!(snapshot[0].status, evergreen_core::tabs::TabStatus::Crashed);
    assert_eq!(snapshot[1].url, "https://beta.example");

    // Restore into a fresh TabManager
    let mut tm_restored = TabManager::new();
    tm_restored.restore_from_snapshot(snapshot);
    assert_eq!(tm_restored.tabs().len(), 2);
    assert_eq!(tm_restored.tabs()[0].url, "https://alpha.example");
    assert_eq!(tm_restored.tabs()[1].url, "https://beta.example");
}

#[test]
fn test_tab_manager_extract_and_insert_tab() {
    let mut tm1 = TabManager::new();
    let id1 = tm1.create_tab("https://tab1.com", 100);
    let _id2 = tm1.create_tab("https://tab2.com", 200);

    // Extract tab 1 from tm1
    let extracted = tm1.extract_tab(id1);
    assert!(extracted.is_some());
    let tab_state = extracted.unwrap();
    assert_eq!(tab_state.url, "https://tab1.com");
    assert_eq!(tm1.tabs().len(), 1);

    // Insert into tm2
    let mut tm2 = TabManager::new();
    let new_id = tm2.insert_tab(tab_state, None);
    assert_eq!(tm2.tabs().len(), 1);
    assert_eq!(tm2.tabs()[0].id, new_id);
    assert_eq!(tm2.tabs()[0].url, "https://tab1.com");
    assert_eq!(tm2.active_tab_id(), Some(new_id));
}

#[test]
fn test_tab_manager_update_url_title_and_favicon() {
    let mut tm = TabManager::new();
    let id = tm.create_tab("https://initial.com", 100);

    tm.update_url(id, "https://updated.com".to_string());
    tm.update_title(id, "Updated Title".to_string());
    tm.update_favicon(id, Some("https://updated.com/favicon.ico".to_string()));

    let tab = tm.tabs().iter().find(|t| t.id == id).unwrap();
    assert_eq!(tab.url, "https://updated.com");
    assert_eq!(tab.title, "Updated Title");
    assert_eq!(
        tab.favicon_uri,
        Some("https://updated.com/favicon.ico".to_string())
    );
}

#[test]
fn test_settings_search_url_template_fallback() {
    let settings = Settings {
        search_engine: "unknown_engine".to_string(),
        ..Default::default()
    };

    assert_eq!(
        settings.search_url_template(),
        "https://duckduckgo.com/?q=%s"
    );
}

#[test]
fn test_settings_feature_flags_json_patching() {
    let mut settings = Settings::default();
    assert!(settings.features.enable_find_in_page);

    // Patch enable_find_in_page to false
    let patch = serde_json::json!({
        "features": {
            "enable_find_in_page": false,
            "enable_downloads_manager": false
        }
    });

    settings.update_from_json(&patch);
    assert!(!settings.features.enable_find_in_page);
    assert!(!settings.features.enable_downloads_manager);
    // Other flags should remain intact
    assert!(settings.features.enable_link_preview);
}

#[test]
fn test_updater_run_local_command() {
    use evergreen_core::updater::run_local_command;

    #[cfg(windows)]
    let res = run_local_command("cmd /C echo evergreen_test_ok");
    #[cfg(not(windows))]
    let res = run_local_command("echo evergreen_test_ok");

    assert!(res.is_ok());
    let output = res.unwrap();
    assert!(output.success);
    assert!(output.stdout.contains("evergreen_test_ok"));
}

#[test]
fn test_plugin_registry_feature_plugin_dynamic() {
    use evergreen_core::plugins::{FeaturePlugin, PluginMetadata, PluginRegistry};

    let mut registry = PluginRegistry::new();
    let meta = PluginMetadata {
        id: "zoom_plugin".to_string(),
        name: "Zoom Controls".to_string(),
        version: "1.0.0".to_string(),
        description: "Dynamic zoom".to_string(),
        author: "Evergreen".to_string(),
        is_core: false,
        enabled_by_default: true,
    };

    let plugin = FeaturePlugin::new(meta, |s| s.features.enable_zoom_controls);
    registry.register(plugin);

    let mut settings = Settings::default();
    settings.features.enable_zoom_controls = true;
    assert_eq!(registry.enabled_plugins(&settings).len(), 1);

    settings.features.enable_zoom_controls = false;
    assert_eq!(registry.enabled_plugins(&settings).len(), 0);
}
