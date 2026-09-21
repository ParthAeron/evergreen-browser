# Architecture Overview

Evergreen Browser embeds the host Windows WebView2 Runtime behind a native Rust desktop shell. Instead of shipping a bundled 150MB+ Chromium binary, the browser delegates web engine updates and security patching directly to Microsoft Edge Update on the user's operating system.

---

## 1. Crate Boundaries

The workspace separates business logic, engine bindings, and window management into three tiers:

```text
crates/
├── core/               # Engine-agnostic business logic & extensibility
├── engine-webview2/    # WebView2 COM bindings and engine adapters
└── app/                # Native window event loop, multi-child HWND layout, and UI
```

### `evergreen-core`
Contains all data structures and state machines:
- **`TabManager`**: Pure in-memory tab state collection, active tab transitions, and closed-tab restoration history.
- **`Settings`**: Configuration struct, feature flags, and partial JSON patch logic.
- **`BrowserPlugin`**: Extension trait for custom toolbars, sidebars, content scripts, and IPC message handlers.
- **`UiToHostMessage` & `HostToUiMessage`**: Strongly typed serde enums defining the IPC protocol.
- **`env`**: Zero-residue portable mode path resolution and token elevation checks.

`evergreen-core` does not depend on `wry`, `winit`, or `windows`. It compiles and runs unit tests on any platform.

### `evergreen-engine-webview2`
The sole crate in the workspace referencing COM interfaces and `webview2-com`. It implements the `EngineHost` trait and manages engine-level calls such as memory suspension and hardware composition.

### `evergreen-browser` (`app`)
The native application binary:
- Initializes the `winit` event loop.
- Manages child window handles (`HWND`) for the top navigation bar, sidebars, and active content tabs.
- Routes user keystrokes through controller-level `AcceleratorKeyPressed` hooks.
- Sets PE subsystem headers (`windows_subsystem = "windows"`) to ensure silent GUI execution without console pop-ups.

---

## 2. Multi-Child Window Layout

Evergreen Browser uses native Win32 child windows inside a single top-level `winit` window frame.

```text
Top-Level Window (winit HWND, e.g. 1280x800)
 ├── Chrome Strip Child HWND
 │     └── Bounds: (0, 0, WindowWidth, 76px)
 │     └── Fixed at top, never suspended
 │
 ├── Active Tab Content HWND
 │     └── Bounds: (0, 76, ContentWidth, WindowHeight - 76)
 │     └── Active GPU compositing, visible
 │
 └── Inactive Tab Content HWNDs
       └── Hidden via ShowWindow(SW_HIDE)
       └── Suspended via TrySuspendAsync() after 5 minutes
```

When switching tabs, the host toggles Win32 visibility flags directly. This avoids compositor surface recreation and keeps tab switching latency under 0.2 milliseconds.

When the sidebar opens (Downloads, Security, or Menu), content webviews resize smoothly to `(WindowWidth - 320px)`.

---

## 3. IPC Message Pipeline

All communication between the HTML navigation strip and the Rust host uses typed JSON messages:

1. **User interaction in UI**: JavaScript calls `window.ipc.postMessage(JSON.stringify({ action: "Navigate", payload: { url: "..." } }))`.
2. **Rust deserialization**: The IPC bridge deserializes the string into `UiToHostMessage` using `serde_json`.
3. **State update**: `TabManager` updates internal collections.
4. **DOM sync**: The host evaluates `window.__shellUpdate(...)` with the new tab state snapshot.

Content web pages never have access to `window.ipc`. The message bridge is bound exclusively to the chrome navigation webview.

---

## 4. Tab Sleep & Memory Management

Inactive tabs enter a low-memory state automatically:
- After 5 minutes without user interaction, background tabs invoke `ICoreWebView2::TrySuspendAsync()`.
- The engine sets the target memory usage level to `LOW`, flushing unused render caches and suspending JavaScript timers.
- Tabs playing audio are exempt from suspension.
- Clicking a suspended tab restores execution immediately without reloading the network document.

