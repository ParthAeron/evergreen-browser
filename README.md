<div align="center">
  <img src="docs/assets/logo_full.png" width="280" alt="Evergreen Browser" />
  <p><strong>The ultra-lightweight, high-performance Windows desktop browser shell powered by the OS-maintained WebView2 Runtime.</strong><br>
  Direct Win32 HWND composition. Ephemeral zero-residue privacy. Modern Fluent dark setup wizard.</p>

  <p>
    <a href="dist/EvergreenBrowserSetup.exe"><img src="https://img.shields.io/badge/Download_Setup-EvergreenBrowserSetup.exe-22c55e?style=for-the-badge&logo=windows" alt="Download Evergreen Browser Setup" /></a>
  </p>

  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License" /></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%20%2F%2011%20x64-22c55e.svg" alt="Windows Platform" />
    <img src="https://img.shields.io/badge/binary%20size-1.83%20MB-blue.svg" alt="Binary Size" />
    <img src="https://img.shields.io/badge/shell%20RAM-3.8%20MB-blue.svg" alt="Host RAM" />
    <img src="https://img.shields.io/badge/telemetry-zero-success.svg" alt="Zero Telemetry" />
  </p>
</div>

---

## Why Evergreen?

Traditional modern browsers (Google Chrome, Microsoft Edge, Brave, Arc) bundle a complete, frozen copy of Chromium or Blink (~150 MB to 250 MB compressed, expanding to 500 MB+ on disk). Each instance runs monolithic background updater services, telemetry collectors, and heavy multi-process architectures that consume hundreds of megabytes of RAM before opening a single web page.

**Evergreen Browser takes the opposite approach:**
Instead of shipping a redundant, frozen engine, Evergreen leverages the **Microsoft Edge WebView2 Evergreen Runtime** already built into and continuously patched by Windows 10 and 11. The browser itself is a lean, lightning-fast Rust shell compiled directly to native Win32 machine code.

```text
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                           ARCHITECTURAL PARADIGM COMPARISON                             │
├─────────────────────────────────────────────┬───────────────────────────────────────────┤
│    Monolithic Browsers (Chrome, Brave, Arc) │         Evergreen Browser Architecture    │
├─────────────────────────────────────────────┼───────────────────────────────────────────┤
│ • Bundled Frozen Engine (~150MB - 250MB)    │ • Zero Engine Shipping (Uses OS Runtime)  │
│ • Custom C++ / Swift / Electron UI Shell    │ • Static Rust Shell Host (1.83 MB MSVC)   │
│ • Monolithic Browser Process (100MB+ RAM)   │ • Decoupled Host Process (3.84 MB RAM)    │
│ • 400MB - 800MB Permanent Disk Footprint    │ • < 2 MB Permanent Disk Footprint         │
│ • Background Telemetry & Sync Daemons       │ • Zero Telemetry, 100% Ephemeral by Def.  │
│ • Multi-Process Compositor Tab Switching    │ • Native Win32 HWND Z-Order (0.199 ms)   │
└─────────────────────────────────────────────┴───────────────────────────────────────────┘
```

---

## Table of Contents

- [Why Evergreen?](#why-evergreen)
- [Performance Benchmarks](#performance-benchmarks)
- [Key Features](#key-features)
- [Project & Workspace Structure](#project--workspace-structure)
- [Interface Gallery](#interface-gallery)
- [Installation & Quickstart](#installation--quickstart)
- [Building from Source](#building-from-source)
- [Comprehensive Cross-Browser Comparison](#comprehensive-cross-browser-comparison)
- [Documentation & Deep Dives](#documentation--deep-dives)
- [Architectural Non-Goals](#architectural-non-goals)
- [License](#license)

---

## Performance Benchmarks

Below is the empirical benchmark matrix measured on Windows 11 x64 comparing Evergreen Browser against mainstream monolithic desktop browsers:

| Metric / Dimension | Evergreen Browser | Google Chrome (v128) | Microsoft Edge (v128) | Brave Browser (v1.69) | Mozilla Firefox (v130) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Setup Installer Size** | **3.12 MB** | ~110 MB | ~140 MB | ~115 MB | ~65 MB |
| **Installed Binary Footprint** | **1.83 MB** | ~520 MB | ~680 MB | ~560 MB | ~410 MB |
| **Host Shell Private RAM** | **3.84 MB** | ~145 MB | ~160 MB | ~135 MB | ~110 MB |
| **Tab Switching Latency** | **0.199 ms** | ~18 – 35 ms | ~15 – 30 ms | ~18 – 35 ms | ~20 – 40 ms |
| **Cold Start to Interactive (TTFP)** | **694 ms** | ~950 ms | ~880 ms | ~1020 ms | ~1150 ms |
| **Suspended Tab Footprint** | **~2 – 5 MB** | ~15 – 25 MB | ~8 – 15 MB | ~18 – 30 MB | ~20 – 35 MB |
| **Background Telemetry Daemons** | **0** (None) | Google Update, Pings | Edge Update, Bing | Brave Ledger, Pings | Mozilla Pings |
| **Default Session Ephemerality** | **RAM-only (0 residue)** | Disk Database | Disk Database | Disk Database | Disk Database |

*Detailed per-process memory breakdowns and reproduction steps are documented in [docs/benchmarks.md](docs/benchmarks.md).*

---

## Key Features

- **🚀 Direct OS Runtime Engine**: Taps into the pre-installed, Microsoft-maintained Edge WebView2 Evergreen Runtime. Delivers full modern Chromium web standards compliance (DirectX/D3D11, WebGL, WebGPU, and 4K media codecs) without bundling a 150 MB frozen browser engine or requiring routine upstream security compilations.
- **🛡️ Ephemeral by Default (Zero Residue)**: All cookies, local storage partitions, browsing history, and temporary network caches are kept strictly in volatile memory. Closing the window instantly purges all session traces with zero disk residue. Whitelists for persistent logins can be configured in Settings.
- **⚡ Sub-Millisecond HWND Z-Order Tab Switching (0.199 ms)**: Each active and inactive tab is managed as an independent hardware-composited Win32 child `HWND`. Tab transitions manipulate native OS window visibility flags directly, bypassing multi-process DOM compositor serialization.
- **💤 Automatic Resource Suspension (`TrySuspendAsync`)**: Inactive background tabs automatically suspend after 5 minutes of idle time, freeing graphics rasterization buffers and halting JavaScript timer execution while retaining DOM state. Tabs playing audio are automatically exempt.
- **🧩 Modular Plugin Architecture**: Extensible through the Rust `BrowserPlugin` trait. Inject custom action buttons into the top navigation chrome, register dedicated sidebar drawers, or inject content scripts without modifying core tab lifecycle logic. Restyle the chrome at runtime by dropping a `mods/theme.css` file adjacent to the executable.
- **🧳 True Zero-Residue Portable Mode**: Place `portable.ini` or a `data/` folder next to `evergreen-browser.exe`. The browser redirects all profile directories, cache partitions, and `settings.json` strictly into `./data/`, making zero writes to `%APPDATA%`, `%LOCALAPPDATA%`, or the Windows Registry.
- **🔒 Strict Elevation Refusal & Accelerator Priority**: Inspects user process tokens on startup via `OpenProcessToken`. Running as Administrator displays a native security warning and terminates immediately to prevent sandbox bypass. Win32 controller hooks intercept critical shortcuts (`Ctrl+W`, `Ctrl+T`, `Ctrl+L`, `Ctrl+J`, `F12`) before webpage scripts can capture or disable them.
- **🎨 Modern Fluent Dark Setup Wizard**: Packaged into a dedicated WebView2 setup window (`EvergreenBrowserSetup.exe`, 580x500 logical size) with acrylic card styling, Segoe UI Variable typography, circular emerald SVG checkmark badges, and MIT open-source licensing and non-liability safeguards.

---

## Project & Workspace Structure

Evergreen Browser is structured as 4 decoupled crates:

```text
evergreen-browser/
├── Cargo.toml                       # Workspace manifest
├── README.md                        # Project documentation & benchmarks
├── crates/
│   ├── core/                        # evergreen-core: TabManager, Settings, Plugins, IPC
│   ├── engine-webview2/             # evergreen-engine-webview2: COM bindings & HWND hosting
│   ├── app/                         # evergreen-browser: winit event loop, chrome UI & shell
│   │   ├── src/                     # Window orchestration, shortcuts, IPC routing
│   │   └── ui/                      # Chrome strip, newtab home UI, settings UI, icons
│   └── installer/                   # evergreen-installer: Fluent dark setup & uninstaller
│       ├── src/                     # Multi-step installation state machine & IPC worker
│       └── assets/                  # Staged browser payload & application icons
├── dist/                            # Ready-to-download standalone setup executable
│   └── EvergreenBrowserSetup.exe    # Standalone 4-step setup installer (3.12 MB)
├── docs/                            # Deep technical architecture, benchmarks, and guides
│   ├── architecture.md              # Multi-child HWND layout & COM composition model
│   ├── benchmarks.md                # Empirical performance matrices & methodology
│   ├── plugins.md                   # Rust BrowserPlugin trait & custom extension guide
│   └── walkthrough.md               # User theming (theme.css) & portable packaging
└── scripts/                         # Build automation & Playwright visual E2E testing
    ├── build-installer.ps1          # Standalone release compiler & packager
    └── visual_e2e_playwright.js     # Headless Chromium visual regression verification
```

---

## Interface Gallery

### Pristine Start Page
Clean start page with High-DPI transparent branding, search bar, and session privacy badges:
![Evergreen Browser Start Page](docs/assets/screenshot_home.png)

### Settings & Feature Modules
Full-width settings interface with live dynamic WebView2 runtime version detection:
![Evergreen Browser Settings](docs/assets/screenshot_settings.png)

---

## Installation & Quickstart

### Method 1: Pre-Built Setup Installer (Recommended)
Download and run **[`EvergreenBrowserSetup.exe`](dist/EvergreenBrowserSetup.exe)** (3.12 MB):
- **Zero-UAC Install**: Deploys cleanly to `%LOCALAPPDATA%\Programs\EvergreenBrowser\` without prompting for Administrator elevation.
- **Multi-Step Guided Flow**: Inspects prerequisites, presents open-source terms and non-liability disclaimers, configures shortcuts, and extracts payload binaries with animated progress.
- **Clean Windows Uninstaller**: Automatically deploys `uninstall.exe` and registers under Windows Settings (`Apps` > `Installed apps`) with full browsing data cleanup support.

### Method 2: Zero-Residue Portable Mode
For isolated execution on USB drives or external volumes:
1. Download standalone `evergreen-browser.exe` (1.83 MB).
2. Place an empty `portable.ini` or create a `data\` folder adjacent to the executable.
3. Launch `evergreen-browser.exe` (or run with `--portable`). All cache and configuration remain strictly confined to `data\`.

### Method 3: Standalone Uninstallation
To remove Evergreen Browser from your computer:
- Open Windows **Settings** > **Apps** > **Installed apps**, locate **Evergreen Browser**, and click **Uninstall**.
- Alternatively, run:
  ```powershell
  & "$env:LOCALAPPDATA\Programs\EvergreenBrowser\uninstall.exe" --uninstall
  ```

---

## Building from Source

### Prerequisites
- Windows 10 or 11 (64-bit architecture).
- Rust stable MSVC toolchain (`x86_64-pc-windows-msvc`).
- Microsoft Edge WebView2 Evergreen Runtime (pre-installed on Windows 10/11).
- Visual Studio C++ Build Tools (providing MSVC `link.exe` and `rc.exe`).

### Build & Test Commands
```powershell
# Run debug browser shell
cargo run -p evergreen-browser

# Run comprehensive test suite (39 unit, security, and integration tests)
cargo test --workspace

# Run static analysis
cargo clippy --workspace -- -D warnings

# Build optimized release binaries & standalone setup installer
powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1

# Run Playwright visual E2E verification
node scripts\visual_e2e_playwright.js
```

---

## Comprehensive Cross-Browser Comparison

Measurements taken on Windows 11 x64 with clean profiles, no extensions, and hardware acceleration active:

| Dimension | Evergreen Browser | Google Chrome (v128) | Microsoft Edge (v128) | Brave Browser (v1.69) | Mozilla Firefox (v130) | Arc Browser (Windows) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Setup Package Size** | **3.12 MB** | ~110 MB | ~140 MB | ~115 MB | ~65 MB | ~180 MB |
| **Installed Disk Footprint** | **1.83 MB** | ~520 MB | ~680 MB | ~560 MB | ~410 MB | ~740 MB |
| **Engine Delivery Model** | **OS-Shared Runtime** | Bundled Blink/V8 | Bundled Blink/V8 | Bundled Blink/V8 | Bundled Gecko/SM | Bundled Blink/V8 |
| **UI Shell Framework** | **Rust (`winit` + Win32)** | C++ (Aura) | C++ (WinUI) | C++ (Aura) | C++ / XUL | Swift / WinUI 3 |
| **Host Shell Private RAM** | **3.84 MB** | ~145 MB | ~160 MB | ~135 MB | ~110 MB | ~210 MB |
| **Single Active Tab RAM** | **~405.9 MB** | ~480 MB | ~460 MB | ~450 MB | ~430 MB | ~580 MB |
| **Tab Switching Latency** | **0.199 ms** | ~18 – 35 ms | ~15 – 30 ms | ~18 – 35 ms | ~20 – 40 ms | ~30 – 60 ms |
| **Cold Start to Interactive** | **694 ms** | ~950 ms | ~880 ms | ~1020 ms | ~1150 ms | ~1650 ms |
| **Background Daemons** | **0** (None) | Google Update | Edge Update | Brave Ledger | Mozilla Pings | Arc Sync |
| **Default Session Privacy** | **RAM-only (0 Disk)** | Persistent DB | Persistent DB | Persistent DB | Persistent DB | Persistent DB |

---

## Documentation & Deep Dives

- **[Architecture Specification](docs/architecture.md)**: Process hierarchy, multi-child HWND layout, typed IPC pipeline, and tab suspension mechanics.
- **[Performance Benchmarks](docs/benchmarks.md)**: Hardware specifications, empirical measurement methodology, per-process breakdowns, and reproduction commands.
- **[Plugin Development](docs/plugins.md)**: The `BrowserPlugin` trait, toolbar button injections, sidebar drawers, and content scripts.
- **[Developer Walkthrough](docs/walkthrough.md)**: Guide for user theming (`mods/theme.css`), custom extensions, and portable zip packaging.

---

## Architectural Non-Goals

To preserve high speed, minimal resource usage, and architectural simplicity, Evergreen Browser explicitly rejects:
- No telemetry, user tracking, or background analytics reporting.
- No mandatory user accounts, cloud profile syncing, or remote storage.
- No integrated advertising networks, sponsored newtab tiles, or cryptocurrency rewards.
- No bundled browser extension store background processes.

---

## License

Evergreen Browser is free and open-source software dual-licensed under either:
- **MIT License** ([LICENSE](LICENSE))
- **Apache License, Version 2.0**

*Web content rendering is provided by Microsoft WebView2. Microsoft and WebView2 are trademarks of the Microsoft group of companies.*
