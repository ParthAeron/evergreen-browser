//! Evergreen Browser — Main Application Entrypoint

mod chrome;
mod window;

use evergreen_core::env::detect_webview2_runtime;

fn main() {
    println!("Starting Evergreen Browser...");
    
    // Check runtime preflight
    match detect_webview2_runtime() {
        Some(version) => println!("Detected Evergreen WebView2 Runtime: {}", version),
        None => eprintln!("Warning: Evergreen WebView2 Runtime not detected in standard location. Bootstrapper may be required."),
    }

    println!("Ready for shell event loop initialization.");
}
