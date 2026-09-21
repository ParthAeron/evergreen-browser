<div align="center">
  <img src="docs/assets/logo.png" width="108" height="108" alt="Evergreen Browser Logo" />
  <h1>Evergreen Browser</h1>
  <p><strong>A 1.2 MB native Windows browser shell powered by the OS-maintained WebView2 Runtime.</strong></p>

  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License" /></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%20%2F%2011%20x64-22c55e.svg" alt="Windows Platform" />
    <img src="https://img.shields.io/badge/binary%20size-1.23%20MB-blue.svg" alt="Binary Size" />
    <img src="https://img.shields.io/badge/shell%20RAM-3.8%20MB-blue.svg" alt="Host RAM" />
    <img src="https://img.shields.io/badge/telemetry-zero-success.svg" alt="Zero Telemetry" />
  </p>
</div>

---

## Why Evergreen?

Mainstream desktop browsers have grown into massive application runtimes. A fresh installation often demands 500 MB to 700 MB of disk space, launches dozens of utility processes on startup, runs continuous background telemetry, and consumes hundreds of megabytes of memory before you even open a webpage. Many bundle crypto wallets, shopping toolbars, sponsored new-tab tiles, and forced account sync.

Windows 10 and 11 already ship with a fully updated, hardware-accelerated Chromium rendering engine: the Microsoft Edge WebView2 Runtime. 

Evergreen Browser taps directly into that existing system runtime through a 1.2 MB native Rust shell. You get modern Chromium rendering, WebGL, WebGPU, and 4K video decoding without carrying a bundled engine, without persistent tracking, and without uninvited background services.

---

## Interface

### Clean Start Page
![Evergreen Browser Start Page](docs/assets/screenshot_home.png)

### Settings & Feature Modules
![Evergreen Browser Settings](docs/assets/screenshot_settings.png)

### Strict TLS Warning Interstitial
![Strict TLS Certificate Error Interstitial](docs/assets/screenshot_interstitial.png)

---

## Verified Benchmarks

Measurements taken on Windows 11 x64 (Release build, MSVC toolchain, hardware acceleration enabled).

| Dimension | Evergreen Browser | Google Chrome | Microsoft Edge | Brave | Mozilla Firefox |
|---|:---:|:---:|:---:|:---:|:---:|
| **Executable / Setup Size** | **1.23 MB** | ~110 MB | ~140 MB | ~115 MB | ~65 MB |
| **Installed Disk Footprint** | **< 2 MB** | ~520 MB | ~680 MB | ~560 MB | ~410 MB |
| **Engine Source** | OS WebView2 | Bundled Blink | Bundled Blink | Bundled Blink | Bundled Gecko |
| **Host Shell Private RAM** | **3.84 MB** | ~145 MB | ~160 MB | ~135 MB | ~110 MB |
| **Total Private Memory (2 Tabs)** | **228.53 MB** | ~480 MB | ~510 MB | ~460 MB | ~530 MB |
| **Cold Start to First Paint (TTFP)** | **694 ms** | ~950 ms | ~880 ms | ~1,020 ms | ~1,150 ms |
| **Tab Switching Latency** | **0.199 ms** | ~18 – 35 ms | ~15 – 30 ms | ~18 – 35 ms | ~20 – 40 ms |
| **Idle Shell CPU Usage** | **0.0%** | 0.2 – 0.8% | 0.3 – 0.9% | 0.2 – 0.6% | 0.2 – 0.5% |
| **Startup Background Processes** | **1 host** | 7 – 12 | 9 – 15 | 8 – 14 | 6 – 10 |
| **Outbound Telemetry Pings** | **0 (None)** | Continuous | Continuous | Periodic | Periodic |

Detailed methodology and reproduction steps are documented in [docs/benchmarks.md](docs/benchmarks.md).

---

## Why You Would Want to Use Evergreen

### 1. Ephemeral by Default
Every session runs in isolated memory. When you close the browser window, all browsing history, session cookies, and temporary network caches are immediately discarded. You never have to manually clear your history or delete tracking cookies. If you want specific websites (like your email or developer dashboard) to keep you logged in, add them to your persistent sites list in Settings.

### 2. Low Resource Consumption for Devs & Gamers
The Rust shell idles at 3.8 MB of private RAM and 0.0% CPU. Inactive background tabs call `TrySuspendAsync()` after 5 minutes, freeing unused rendering buffers and halting JavaScript timer loops. Tabs playing audio are automatically exempt. You can leave Evergreen open alongside heavy IDEs, compiler jobs, Discord, or games without fighting for system resources.

### 3. True Zero-Residue Portable Mode
Drop an empty `portable.ini` or a `data/` folder next to `evergreen-browser.exe`. The browser redirects all profile data, cache partitions, and `settings.json` directly into `./data/`. It leaves `%APPDATA%`, `%LOCALAPPDATA%`, and the Windows Registry untouched. You can carry your browser setup on a USB flash drive or sync it through private cloud storage across different machines.

### 4. Zero Engine Maintenance Overhead
Forking or maintaining an Electron or CEF browser requires compiling and distributing a 150 MB binary whenever Chromium fixes a zero-day vulnerability. Evergreen uses the Microsoft-serviced WebView2 Evergreen Runtime. Microsoft Edge Update patches the rendering engine in the background automatically, keeping security fixes current without application rebuilds.

### 5. Strict Elevation Refusal
Browsing the web with elevated system privileges creates unnecessary attack surface. Evergreen inspects its process token on startup via `OpenProcessToken`. If launched as Administrator, it displays a native Windows warning dialog and terminates immediately, closing token escalation vectors before web content can initialize.

### 6. Reliable Win32 Hotkey Interception
Web applications frequently capture keydown events to disable browser navigation keys. Evergreen intercepts hotkeys at the Win32 controller level (`AcceleratorKeyPressed`), invoking `SetHandled(true)` so that `Ctrl+W`, `Ctrl+T`, `Ctrl+L`, `Ctrl+Tab`, and `F12` execute immediately regardless of what scripts run on the webpage.

### 7. Custom Theming via User CSS
You can restyle the browser without touching Rust code or rebuilding the executable. Create a `mods/theme.css` file next to `evergreen-browser.exe` to override color schemes, button layouts, font sizes, or tab border styles.

---

## Architecture

Evergreen Browser splits responsibilities into three distinct crates:

```text
crates/
├── core/               # TabManager, Settings, typed IPC schemas, plugin traits
├── engine-webview2/    # WebView2 COM bindings, engine adapters, memory suspension
└── app/                # winit event loop, multi-child HWND layout, and UI shell
```

- **`evergreen-core`**: Pure, engine-agnostic business logic. Manages tab lifecycles, crash restoration snapshots, URL normalization, and portable directory resolution. Has no dependency on GUI frameworks or COM.
- **`evergreen-engine-webview2`**: Connects the abstract engine interface to Microsoft WebView2 COM interfaces, managing low-level controller suspension and composition.
- **`evergreen-browser`**: Coordinates the Win32 window layout. Places the chrome navigation webview in a fixed top band `(0, 0, width, 76px)` and maps each tab to a child HWND below it. Tab switching toggles Win32 visibility flags directly, avoiding surface recreation.

Detailed diagrams and IPC message sequences are documented in [docs/architecture.md](docs/architecture.md).

---

## Building from Source

### Prerequisites
- Windows 10 or 11 (64-bit).
- Rust stable toolchain with MSVC target (`x86_64-pc-windows-msvc`).
- Microsoft Edge WebView2 Runtime (installed with Windows).
- Visual Studio C++ Build Tools (provides `link.exe` and `rc.exe`).

### Build & Run
```powershell
# Run debug build
cargo run -p evergreen-browser

# Build optimized release binary
cargo build --release -p evergreen-browser

# Launch in portable mode
target\release\evergreen-browser.exe --portable
```

---

## Documentation

- **[Architecture Guide](docs/architecture.md)**: Window composition model, multi-child HWND layout, typed IPC pipeline, and tab suspension mechanics.
- **[Performance Benchmarks](docs/benchmarks.md)**: Hardware specifications, methodology, per-process memory breakdowns, and reproduction commands.
- **[Plugin Development](docs/plugins.md)**: The `BrowserPlugin` trait, toolbar button injection, sidebar drawers, and content scripts.
- **[Developer Walkthrough](docs/walkthrough.md)**: Step-by-step guide for theming (`mods/theme.css`), custom plugins, and portable zip packaging.

---

## Non-Goals

- No telemetry, analytics, or background reporting services.
- No mandatory accounts, profile syncing, or remote storage.
- No integrated advertising, sponsored tiles, or crypto widgets.
- No bundled extensions store overhead.

---

## License

Dual-licensed under either:
- MIT License ([LICENSE](LICENSE))
- Apache License, Version 2.0
