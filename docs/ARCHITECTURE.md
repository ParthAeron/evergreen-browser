# Evergreen Browser Architecture

Evergreen Browser is an ultra-lightweight, privacy-first desktop browser shell for Windows. It provides a window, a tab strip, an address bar, a settings modal, and native Chromium DevTools — forgetting everything when closed.

## 1. Architectural Philosophy

- **Zero Engine Shipping**: Embeds the evergreen, Microsoft-maintained WebView2 Runtime already present on Windows. Auto-updates via Windows/Edge Update with zero engine maintenance burden.
- **Static Native Shell**: Built in Rust (`windows-msvc`), compiling to a single static binary (< 8 MB) with zero runtime dependencies.
- **Engine Swappability**: Pure separation between browser business logic (`core`) and the rendering engine backend (`engine-webview2`).
- **Protected Invariants**: Hardware acceleration (GPU process), Chromium sandbox (least-privilege, unelevated), per-tab process isolation, and per-tab sleep memory reclamation are strictly maintained.

## 2. Workspace Layout

```text
evergreen-browser/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── core/                  # Engine-agnostic models, tab state, settings, IPC protocol
│   │   ├── src/engine.rs      # EngineHost trait and EngineInfo
│   │   ├── src/env.rs         # Runtime & elevation detection
│   │   ├── src/ipc.rs         # Typed IPC schemas (UiToHostMessage, HostToUiMessage)
│   │   ├── src/settings.rs    # JSON settings model & persistence
│   │   ├── src/tabs.rs        # TabManager & lifecycle states
│   │   └── src/updater.rs     # Local shell update runner
│   ├── engine-webview2/       # Concrete WebView2 adapter (implements EngineHost)
│   └── app/                   # Binary: winit event loop, windowing, embedded Chrome webview
│       ├── src/main.rs        # Entrypoint
│       ├── src/window.rs      # Native window setup
│       ├── src/chrome.rs      # Chrome webview controller & IPC routing
│       └── ui/index.html      # Embedded HTML/CSS/JS chrome strip
├── mods/
│   └── theme.css              # User CSS custom properties for theming
└── docs/
    ├── ARCHITECTURE.md        # This document
    └── NON_GOALS.md           # Permanent out-of-scope boundaries
```

## 3. Runtime Model & Process Hierarchy

```text
winit EventLoop (App Process, Non-Elevated)
 ├── Chrome WebView (wry / WebView2)
 │    └── Renders UI strip: tab bar, omnibox, settings
 │    └── Bi-directional IPC via postMessage (JSON)
 └── Content WebViews (1 child webview per open tab)
      ├── Active Tab:   set_bounds(content_rect), set_visible(true)
      └── Inactive Tab: set_visible(false), TrySuspendAsync() after N minutes
```

Chromium spawns:
1. **GPU Process**: Hardware-accelerated compositing, WebGL, WebGPU, video decode.
2. **Renderer Processes**: Site-isolated renderer processes for active tabs.
3. **Audio Service Process**: Managed per-tab with mute and audio indicators.

## 4. Forking & Modding

- **Theme Modification**: Edit or supply `mods/theme.css`. The chrome webview automatically applies custom variables.
- **Swapping the Engine**: Replace `crates/engine-webview2` by implementing the `EngineHost` trait found in `crates/core/src/engine.rs`. No changes are required in `core`.
