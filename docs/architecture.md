# Architecture Overview

Evergreen Browser embeds the host Windows WebView2 Runtime behind a native Rust desktop shell. Instead of shipping a bundled 150 MB+ Chromium binary, the browser delegates web engine updates and security patching directly to Microsoft Edge Update on the user's operating system.

---

## 1. Crate Boundaries & Dependencies

The workspace separates business logic, engine bindings, and window management into three decoupled tiers:

```mermaid
graph TD
    subgraph UI ["User Interface Layer"]
        Chrome["HTML/CSS/JS Chrome Strip<br>(Tabs, Omnibox, Actions)"]
        Sidebar["WinUI 3 Sidepanels<br>(Downloads, Menu, Cert)"]
    end

    subgraph AppTier ["Application Tier: evergreen-browser"]
        EventLoop["winit EventLoop & Window Manager"]
        Win32Hooks["Win32 Accelerator & Hotkey Interceptors"]
        ResComp["Native Win32 PE Resources & Icon"]
    end

    subgraph CoreTier ["Core Logic Tier: evergreen-core"]
        TabMgr["TabManager State Machine"]
        SettingsMod["Settings & Feature Flags"]
        PluginReg["PluginRegistry & BrowserPlugin Trait"]
        IpcSchemas["UiToHost & HostToUi IPC Protocols"]
        EnvLogic["Portable Mode & Token Elevation Checks"]
    end

    subgraph EngineTier ["Engine Tier: evergreen-engine-webview2"]
        EngineHost["EngineHost Trait Implementation"]
        COMAdapter["WebView2 COM Controller & Composition"]
        MemSuspend["ICoreWebView2::TrySuspendAsync() Adapter"]
    end

    subgraph OS ["Operating System Layer"]
        WebView2["Microsoft Edge WebView2 Runtime (Evergreen)"]
        BlinkGPU["DirectX/D3D11 Compositor & Sandboxed Blink"]
    end

    Chrome -->|window.ipc| EventLoop
    Sidebar -->|window.ipc| EventLoop
    EventLoop --> CoreTier
    EventLoop --> EngineTier
    EngineTier --> OS
```

### `evergreen-core`
Contains all data structures, collections, and state machines:
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

Evergreen Browser uses native Win32 child windows inside a single top-level `winit` window frame:

```mermaid
graph TD
    TopWin["Top-Level Native Window Frame (winit HWND, e.g. 1280x800)"]
    
    TopWin --> ChromeHWND["Chrome Navigation Strip Child HWND<br>Bounds: (0, 0, WindowWidth, 76px)<br>Fixed at top · Never suspended"]
    TopWin --> ActiveTabHWND["Active Tab Content HWND<br>Bounds: (0, 76, ContentWidth, WindowHeight - 76)<br>Visible · Active DirectComposition GPU pipeline"]
    TopWin --> InactiveTab1["Inactive Tab 1 HWND<br>ShowWindow(SW_HIDE)<br>Low Memory Target"]
    TopWin --> InactiveTabN["Inactive Tab N HWND<br>ShowWindow(SW_HIDE)<br>TrySuspendAsync() after 5m"]
    TopWin -.->|When Toggled| SidebarHWND["Sidebar Drawer HWND<br>Bounds: (WindowWidth - 320, 76, 320, WindowHeight - 76)"]

    style TopWin fill:#1e3a5f,stroke:#4e8cff,stroke-width:2px,color:#fff
    style ChromeHWND fill:#1b3830,stroke:#34d399,stroke-width:2px,color:#fff
    style ActiveTabHWND fill:#22222e,stroke:#4e8cff,color:#fff
    style InactiveTab1 fill:#16161d,stroke:#555,color:#aaa
    style InactiveTabN fill:#16161d,stroke:#555,color:#aaa
    style SidebarHWND fill:#2b2b3b,stroke:#8b949e,color:#fff
```

When switching tabs, the host toggles Win32 visibility flags directly (`ShowWindow(SW_SHOW)` / `ShowWindow(SW_HIDE)`). This avoids compositor surface recreation and keeps tab switching latency under 0.2 milliseconds.

When a sidebar opens (Downloads, Security, or Menu), the active content webview resizes smoothly to `(WindowWidth - 320px)`.

---

## 3. IPC Message Pipeline

All communication between the HTML navigation strip and the Rust host uses typed JSON messages:

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant ChromeUI as Chrome Navigation Webview
    participant Host as Rust Host Process (evergreen-browser)
    participant TabMgr as TabManager (evergreen-core)
    participant ContentWV as Content Webview (WebView2)

    User->>ChromeUI: Enters URL and presses Enter
    ChromeUI->>Host: window.ipc.postMessage({"action":"Navigate","payload":{"url":"..."}})
    Note over Host: Deserializes JSON into UiToHostMessage::Navigate
    Host->>TabMgr: update_url(active_tab_id, target_url)
    Host->>ContentWV: controller.load_url(target_url)
    Host->>ChromeUI: evaluate_script(window.__shellUpdate(tabState))
    Note over ChromeUI: DOM renders updated tab title and lock state
    ContentWV-->>User: Renders web page with hardware acceleration
```

Content web pages never have access to `window.ipc`. The message bridge is bound exclusively to the local chrome navigation webview.

---

## 4. Tab Sleep & Memory Lifecycle

Inactive tabs enter low-memory states automatically to prevent background resource hogging:

```mermaid
stateDiagram-v2
    [*] --> Active: User opens tab
    Active --> Inactive: User switches to another tab
    Inactive --> Active: User clicks tab / presses shortcut

    Inactive --> Suspending: Inactive for 300 seconds (5 min)
    note right of Suspending: Audio playing tabs are exempt
    
    Suspending --> Suspended: Call ICoreWebView2::TrySuspendAsync()
    note right of Suspended: Target memory level set to LOW<br>Render caches released<br>JavaScript timers halted

    Suspended --> Active: User clicks tab
    note right of Active: State restored immediately<br>No network document reload
```

- Inactive background tabs invoke `ICoreWebView2::TrySuspendAsync()` after 5 minutes without user interaction.
- The engine sets the target memory usage level to `LOW`, flushing unused render caches and suspending JavaScript timers.
- Tabs playing audio are exempt from suspension.
- Clicking a suspended tab restores execution immediately without reloading the network document.
