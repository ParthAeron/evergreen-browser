# Evergreen Browser

> An ultra-lightweight, privacy-respecting Windows browser shell.
> A window, a tab strip, and an address bar — plus a settings modal and real Chromium DevTools — forgetting everything unless you tell it not to.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011%20x64-brightgreen.svg)]()

---

## Highlights

- **Lightweight (< 8 MB binary)**: Zero runtime prerequisites. No .NET Desktop Runtime or bundled Electron/CEF binaries required.
- **Evergreen Engine**: Uses the Microsoft-maintained WebView2 Runtime already present on modern Windows. Automatic platform updates, zero engine maintenance overhead.
- **Hardware-Accelerated & Sandboxed**: Full GPU process acceleration (WebGL, WebGPU, 4K video decode) and strict Chromium process sandboxing.
- **Low Memory Footprint**: Inactive background tabs are suspended (`TrySuspendAsync`) after 5 minutes of inactivity to keep RAM minimal (~600 MB across 10 tabs).
- **Ephemeral by Default**: Session history, cookies, and local cache are cleared automatically on exit unless explicitly added to your persistent sites.
- **DevTools Built-In**: Native undocked Chromium DevTools via `F12`.
- **Themeable & Moddable**: Drop a `mods/theme.css` to restyle the browser chrome. Modular Rust architecture allows swapping rendering engines.

---

## Non-Goals

To maintain speed, privacy, and simplicity, the following features are permanently excluded:
- ❌ No bookmarks or history browser
- ❌ No accounts, cloud sync, or telemetry
- ❌ No extensions or plugin store
- ❌ No AI sidebars, crypto widgets, or shopping assistants
- ❌ No sponsored tiles or feed recommendations

See [docs/NON_GOALS.md](docs/NON_GOALS.md) for details.

---

## Architecture

Evergreen Browser is structured as a cargo workspace with clean separation of concerns:

- `evergreen-core`: Engine-agnostic data models, tab state management, typed IPC schemas, and the `EngineHost` trait.
- `evergreen-engine-webview2`: Thin adapter binding `evergreen-core` to WebView2 via `wry` and raw COM interfaces.
- `evergreen-browser`: The binary shell hosting the `winit` event loop and embedded HTML/CSS/JS chrome webview.

For details on architecture and extending the browser, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Building from Source

### Prerequisites
- Windows 10 (21H2+) or Windows 11 x64
- Rust 1.80+ (MSVC toolchain)
- Visual Studio C++ Build Tools (MSVC `link.exe`)
- WebView2 Runtime (preinstalled on Windows 11 and up-to-date Windows 10)

### Build & Run
```powershell
# Build in release mode
cargo build --release

# Run
cargo run --release

# Run in portable mode (keeps all configuration and temporary cache next to binary)
cargo run --release -- --portable
```

---

## License

Dual-licensed under either of:
- MIT License ([LICENSE](LICENSE))
- Apache License, Version 2.0
