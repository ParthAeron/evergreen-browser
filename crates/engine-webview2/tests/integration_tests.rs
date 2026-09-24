use evergreen_core::engine::EngineHost;
use evergreen_core::env::detect_webview2_runtime;
use evergreen_engine_webview2::WebView2Host;

#[test]
fn test_webview2_host_initialization() {
    let host = WebView2Host::new();
    let info = host.info();

    assert!(!info.name.is_empty(), "Engine name must not be empty");
    assert!(
        info.is_hardware_accelerated,
        "Hardware acceleration must be true"
    );
    assert!(info.sandbox_enabled, "Sandbox must be enabled");

    if let Some(detected_version) = detect_webview2_runtime() {
        assert_eq!(
            info.version, detected_version,
            "Reported version must match detected runtime"
        );
    }
}
