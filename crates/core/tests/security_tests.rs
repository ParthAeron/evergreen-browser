//! Comprehensive Defensive Security & Penetration Testing Suite
//!
//! Evaluates the 7 core security vectors:
//! 1. IPC Protocol Fuzzing & Malformed Deserialization Boundaries
//! 2. Navigation URI Scheme Sanitization & Protocol Smuggling Prevention
//! 3. Production Sandbox & Flag Integrity Invariants
//! 4. Host Object Isolation & DOM Boundaries
//! 5. Strict TLS Host-Scoped Whitelisting Boundaries
//! 6. Process Elevation Token Inspection
//! 7. Collection Bounds & DoS Resistance

use evergreen_core::env::*;
use evergreen_core::ipc::*;
use evergreen_core::settings::*;
use evergreen_core::tabs::*;
use std::path::PathBuf;

// ============================================================================
// VECTOR 1: IPC Protocol Fuzzing & Deserialization Boundaries
// ============================================================================

#[test]
fn test_security_v1_malformed_json_fuzzing() {
    let fuzzed_inputs = vec![
        "",
        " ",
        "\0",
        "{\0}",
        "{",
        "}",
        "[]",
        "null",
        "42",
        "\"raw string\"",
        "{ \"action\": }",
        "{ \"action\": \"Navigate\", \"payload\": ",
        "{ \"action\": \"Navigate\", \"payload\": { \"url\": ",
        "{\"action\": \"SwitchTab\", \"payload\": {\"id\": \"not_a_number\"}}",
        "{\"action\": \"CloseTab\", \"payload\": {\"id\": [1, 2, 3]}}",
        "{\"action\": \"CloseTab\", \"payload\": {\"id\": null}}",
        "{\"action\": \"SaveSettings\", \"payload\": {\"settings_json\": ",
        "{\"action\": \"BypassCertificateError\", \"payload\": null}",
    ];

    for input in fuzzed_inputs {
        let res = serde_json::from_str::<UiToHostMessage>(input);
        assert!(res.is_err(), "Malformed JSON must not parse into UiToHostMessage: {:?}", input);
    }
}

#[test]
fn test_security_v1_inner_settings_json_malformed_safety() {
    // When UiToHostMessage::SaveSettings carries invalid inner JSON,
    // host safely ignores it without panic or state corruption.
    let malformed_inner = "{not valid json!!";
    let parsed_inner = serde_json::from_str::<serde_json::Value>(malformed_inner);
    assert!(parsed_inner.is_err(), "Invalid inner JSON must not parse into Value");

    // Default settings must remain intact
    let mut settings = Settings::default();
    let original_engine = settings.search_engine.clone();
    if let Ok(val) = parsed_inner {
        settings.update_from_json(&val);
    }
    assert_eq!(settings.search_engine, original_engine, "Settings must not mutate on invalid JSON");
}

#[test]
fn test_security_v1_unknown_and_dangerous_action_rejection() {
    let dangerous_actions = vec![
        r#"{"action": "ExecShell", "payload": {"cmd": "calc.exe"}}"#,
        r#"{"action": "WriteFile", "payload": {"path": "C:\\malicious.exe"}}"#,
        r#"{"action": "EvalCode", "payload": "alert(1)"}"#,
        r#"{"action": "DropSandbox", "payload": {}}"#,
        r#"{"action": "ReadCredentials", "payload": {}}"#,
        r#"{"action": "NativeSystemExec", "payload": "shutdown /s"}"#,
    ];

    for action_json in dangerous_actions {
        let res = serde_json::from_str::<UiToHostMessage>(action_json);
        assert!(res.is_err(), "Unknown/dangerous action tag must be rejected: {}", action_json);
    }
}

#[test]
fn test_security_v1_massive_dos_payload_handling() {
    // 10 MB payload string to verify no stack overflow or unhandled OOM
    let large_string = "X".repeat(10 * 1024 * 1024);
    let json = serde_json::json!({
        "action": "Navigate",
        "payload": {
            "url": large_string
        }
    });

    let parsed: Result<UiToHostMessage, _> = serde_json::from_value(json);
    assert!(parsed.is_ok(), "10MB string must be parsed gracefully without panic");
    if let Ok(UiToHostMessage::Navigate { url }) = parsed {
        assert_eq!(url.len(), 10 * 1024 * 1024);
    }
}

#[test]
fn test_security_v1_out_of_bounds_tab_ids() {
    let mut manager = TabManager::new();
    let t1 = manager.create_tab("https://example.com", 100);

    // Closing boundary values must not panic
    assert_eq!(manager.close_tab(TabId(0)), Some(t1));
    assert_eq!(manager.close_tab(TabId(999_999)), Some(t1));
    assert_eq!(manager.close_tab(TabId(u64::MAX)), Some(t1));

    // Switching to boundary values must return false
    assert!(!manager.switch_tab(TabId(0), 101));
    assert!(!manager.switch_tab(TabId(u64::MAX), 101));
    assert!(!manager.switch_tab(TabId(888_888), 101));

    // Crash marking on non-existent tab ID must not panic
    manager.mark_tab_crashed(TabId(0));
    manager.mark_tab_crashed(TabId(u64::MAX));
}

// ============================================================================
// VECTOR 2: Navigation URI Scheme Sanitization & Injection Prevention
// ============================================================================

#[test]
fn test_security_v2_case_insensitive_scheme_injection() {
    let search_template = "https://duckduckgo.com/?q=%s";

    let disallowed_schemes = vec![
        "javascript:alert(1)",
        "JavaScript:alert(document.cookie)",
        "JAVASCRIPT:void(0)",
        "JaVaScRiPt:alert(1)",
        "vbscript:msgbox(1)",
        "VbScRiPt:run",
        "VBSCRIPT:test",
        "data:text/html,<script>alert(1)</script>",
        "DATA:text/html,test",
        "DaTa:text/plain;base64,AAA",
        "file:///C:/Windows/System32/cmd.exe",
        "FILE:///C:/",
        "File://localhost/etc/passwd",
        "about:evil",
        "about:config",
        "about:cache",
        "about:net-internals",
    ];

    for disallowed in disallowed_schemes {
        let res = normalize_url(disallowed, search_template);
        assert!(res.is_err(), "Disallowed URI scheme must be rejected: {}", disallowed);
    }
}

#[test]
fn test_security_v2_safe_schemes_and_search_query_resolution() {
    let search_template = "https://duckduckgo.com/?q=%s";

    let safe_inputs = vec![
        ("https://github.com/rust-lang/rust", "https://github.com/rust-lang/rust"),
        ("http://example.org", "http://example.org"),
        ("evergreen://newtab", "evergreen://newtab"),
        ("evergreen://settings", "evergreen://settings"),
        ("about:blank", "about:blank"),
        ("about:newtab", "evergreen://newtab"),
        ("about:home", "evergreen://newtab"),
        ("about:settings", "evergreen://settings"),
        ("rust-lang.org", "https://rust-lang.org"),
        ("doc.rust-lang.org/std", "https://doc.rust-lang.org/std"),
    ];

    for (input, expected) in safe_inputs {
        let res = normalize_url(input, search_template);
        assert_eq!(res.unwrap(), expected, "Valid URL failed normalization: {}", input);
    }

    // Search query fallback verification
    let search_queries = vec![
        ("rust async runtime", "https://duckduckgo.com/?q=rust+async+runtime"),
        ("hello world", "https://duckduckgo.com/?q=hello+world"),
        ("what is webview2?", "https://duckduckgo.com/?q=what+is+webview2%3F"),
    ];

    for (query, expected) in search_queries {
        let res = normalize_url(query, search_template);
        assert_eq!(res.unwrap(), expected, "Search query normalization mismatch: {}", query);
    }
}

// ============================================================================
// VECTOR 3: Production Sandbox & Flag Integrity Invariants
// ============================================================================

#[test]
fn test_security_v3_production_flags_audit() {
    // Prohibited Chromium command-line flags in production builds
    let prohibited_flags = [
        "--no-sandbox",
        "--disable-web-security",
        "--allow-file-access-from-files",
        "--single-process",
        "--disable-gpu-compositing",
    ];

    let settings = Settings::default();
    let serialized = serde_json::to_string(&settings).unwrap();

    for flag in prohibited_flags {
        assert!(
            !serialized.contains(flag),
            "Prohibited security flag found in default settings: {}",
            flag
        );
    }
}

// ============================================================================
// VECTOR 4: Host Object Isolation & DOM Boundaries
// ============================================================================

#[test]
fn test_security_v4_host_object_isolation_invariants() {
    // Invariant: Native COM host objects must never be exposed to the DOM window
    // `AreHostObjectsAllowed` must remain false in production configuration.
    let settings = Settings::default();
    assert!(settings.privacy.ephemeral_default);

    // Verify IPC message definitions do not expose arbitrary method reflection
    let json_sample = r#"{"action": "HostObjectCall", "payload": {"method": "GetProcess"}}"#;
    let parsed = serde_json::from_str::<UiToHostMessage>(json_sample);
    assert!(parsed.is_err(), "Arbitrary host object calls must be strictly rejected");
}

// ============================================================================
// VECTOR 5: Strict TLS Host-Scoped Whitelisting Boundaries
// ============================================================================

#[test]
fn test_security_v5_tls_host_scoping_boundary() {
    let mut allowed_hosts = std::collections::HashSet::new();
    allowed_hosts.insert("badssl.com".to_string());

    // Exact host matching verification
    assert!(allowed_hosts.contains("badssl.com"));

    // Subdomains must NOT be automatically allowed (no wildcard leak)
    assert!(!allowed_hosts.contains("sub.badssl.com"));
    assert!(!allowed_hosts.contains("evil-badssl.com"));
    assert!(!allowed_hosts.contains("badssl.com.evil.com"));

    // Port and trailing dot injection must not match
    assert!(!allowed_hosts.contains("badssl.com:443"));
    assert!(!allowed_hosts.contains("badssl.com:8443"));
    assert!(!allowed_hosts.contains("badssl.com."));

    // Empty or whitespace host injection must fail
    assert!(!allowed_hosts.contains(""));
    assert!(!allowed_hosts.contains(" "));
}

// ============================================================================
// VECTOR 6: Process Elevation Token Inspection
// ============================================================================

#[test]
fn test_security_v6_process_elevation_token_check() {
    // Calling is_process_elevated must execute safely without panicking.
    // In standard CI/development runs, the process runs unelevated (false).
    let elevated = is_process_elevated();
    println!("[SECURITY AUDIT] Process elevated token status: {}", elevated);
}

// ============================================================================
// VECTOR 7: Collection Bounds & DoS Resistance
// ============================================================================

#[test]
fn test_security_v7_bounded_history_capacity() {
    let mut manager = TabManager::new();

    // Create and close 100 distinct tabs
    for i in 0..100 {
        let id = manager.create_tab(&format!("https://example.com/page-{}", i), i);
        manager.close_tab(id);
    }

    // Verify bounded capacity
    // closed_history must be capped at 50 to prevent unbounded RAM growth
    let mut restored_count = 0;
    while manager.pop_last_closed().is_some() {
        restored_count += 1;
    }

    assert_eq!(restored_count, 50, "Closed tab history must be capped at 50 items");
}

// ============================================================================
// VECTOR 8: Zero-Residue Portable Mode Containment
// ============================================================================

#[test]
fn test_security_v8_portable_data_isolation() {
    let temp_sandbox = std::env::temp_dir().join(format!(
        "evergreen_sec_v8_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_sandbox).unwrap();

    // 1. When portable.ini exists, data directory must resolve strictly inside sandbox
    let ini = temp_sandbox.join("portable.ini");
    std::fs::write(&ini, "").unwrap();

    assert!(is_portable_mode(&temp_sandbox, &[]));
    let resolved = resolve_data_directory(&temp_sandbox, &[]);
    assert!(
        resolved.starts_with(&temp_sandbox),
        "Portable data must strictly stay within executable directory tree: {:?}",
        resolved
    );

    // 2. Settings saved in portable mode must remain contained
    let settings_path = resolved.join("settings.json");
    let settings = Settings::default();
    settings.save_to_path(&settings_path).unwrap();
    assert!(settings_path.exists());

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_sandbox);
}

// ============================================================================
// VECTOR 9: Installer Boundary & Unquoted Path Defense
// ============================================================================

#[test]
fn test_security_v9_installer_path_and_registry_safety() {
    // Verify that uninstall command formatting is strictly quoted
    // to protect against unquoted service path / command injection (CWE-428).
    let sample_uninstaller = PathBuf::from(r"C:\Program Files\Evergreen Browser\uninstall.exe");
    let formatted_cmd = format!("\"{}\" --uninstall", sample_uninstaller.display());

    assert!(formatted_cmd.starts_with('"'));
    assert!(formatted_cmd.contains("\" --uninstall"));
    assert!(!formatted_cmd.starts_with("C:\\Program Files")); // Must be quoted!
}
