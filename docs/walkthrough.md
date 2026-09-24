# Developer Walkthrough

This guide walks through building, customizing, and packaging Evergreen Browser on Windows.

---

## 1. Prerequisites

Before building from source, verify your environment has:
- Windows 10 or 11 (64-bit).
- Rust stable toolchain with the MSVC target (`rustup default stable-x86_64-pc-windows-msvc`).
- Microsoft Edge WebView2 Runtime (installed by default on Windows 10 20H2+ and Windows 11).
- Windows 10/11 SDK (specifically `rc.exe` for icon compilation, included with Visual Studio C++ Build Tools).

---

## 2. Building the Project

Clone the repository and build using Cargo:

### Debug Build
```powershell
cargo run -p evergreen-browser
```
This starts the browser directly with debug symbols enabled.

### Release Build
```powershell
cargo build --release -p evergreen-browser
```
The compiled executable is written to `target/release/evergreen-browser.exe`. The release binary is stripped and optimized, producing an executable around 1.83 MB.

---

## 3. Customizing the Browser Shell

Evergreen Browser supports custom user mods via CSS and JavaScript. You do not need to recompile the Rust binary to restyle the chrome navigation bar.

### Custom Themes (`mods/theme.css`)
Place a `mods/` folder in the same directory as `evergreen-browser.exe` (or in the repository root when running with Cargo):

```
evergreen-browser/
├── mods/
│   ├── theme.css
│   └── script.js
└── evergreen-browser.exe
```

Inside `mods/theme.css`, override CSS custom properties or style UI components:

```css
/* Example: Custom Nord Theme */
:root {
  --bg-chrome: #2e3440;
  --bg-surface: #3b4252;
  --bg-input: #434c5e;
  --text-primary: #eceff4;
  --text-secondary: #d8dee9;
  --accent-color: #88c0d0;
  --accent-hover: #81a1c1;
  --border-color: rgba(255, 255, 255, 0.08);
}

.tab.active {
  border-bottom: 2px solid var(--accent-color);
}
```

### Custom Client Scripts (`mods/script.js`)
Scripts in `mods/script.js` load into the navigation bar context on startup. You can register custom shortcut handlers or adjust UI behaviors:

```javascript
console.log("Custom Evergreen mod script initialized.");
```

---

## 4. Registering a Custom Rust Plugin

To add deeper features like custom IPC handlers or sidepanel drawers:

1. Open `crates/core/src/plugins.rs`.
2. Define your plugin struct and implement the `BrowserPlugin` trait:

```rust
use evergreen_core::plugins::{BrowserPlugin, PluginMetadata};
use evergreen_core::settings::Settings;

pub struct QuickNotesPlugin;

impl BrowserPlugin for QuickNotesPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "quick_notes".to_string(),
            name: "Quick Notes".to_string(),
            version: "0.1.0".to_string(),
            description: "Personal scratchpad in the sidebar".to_string(),
            author: "Forker".to_string(),
            is_core: false,
            enabled_by_default: true,
        }
    }

    fn is_enabled(&self, _settings: &Settings) -> bool {
        true
    }

    fn toolbar_item_html(&self) -> Option<&'static str> {
        Some(r#"<button class="nav-btn" onclick="toggleSidebar('notes')" title="Quick Notes">📝</button>"#)
    }

    fn sidepanel_view(&self) -> Option<(&'static str, &'static str)> {
        Some(("notes", r#"<div style="padding:16px;"><h3>Notes</h3><textarea style="width:100%;height:300px;background:#1e1e1e;color:#fff;border:1px solid #333;border-radius:6px;padding:8px;"></textarea></div>"#))
    }
}
```

3. Register your plugin in `PluginRegistry::default()` inside `crates/core/src/plugins.rs`:

```rust
pub fn default() -> Self {
    let mut reg = Self::new();
    reg.register(Box::new(QuickNotesPlugin));
    reg
}
```

Rebuild with `cargo run -p evergreen-browser` to see your new toolbar button and sidepanel.

---

## 5. Portable Packaging and Distribution

Evergreen Browser supports true zero-residue portable deployment:

### Directory Structure for Portable Distribution
Create a release folder containing:
```
EvergreenPortable/
├── evergreen-browser.exe
├── portable.ini (or data/ folder)
└── mods/
    └── theme.css
```

When an empty file named `portable.ini` or an existing directory named `data/` exists beside `evergreen-browser.exe`, the browser:
- Directs all profile data, cache, cookies, and `settings.json` into `./data/`.
- Leaves `%APPDATA%`, `%LOCALAPPDATA%`, and the Windows Registry untouched.
- Can run from a USB drive or cloud-synced folder across multiple machines.

### Creating a Distribution Archive
From PowerShell:
```powershell
mkdir dist\EvergreenPortable
copy target\release\evergreen-browser.exe dist\EvergreenPortable\
New-Item -ItemType File dist\EvergreenPortable\portable.ini
Compress-Archive -Path dist\EvergreenPortable\* -DestinationPath dist\EvergreenBrowser-x64-portable.zip
```
The resulting ZIP file is approximately 1.8 MB and ready for immediate use on modern 64-bit Windows systems.

---

## 6. Building the Windows Installer (`EvergreenBrowserSetup.exe`)

For standard desktop distribution, Evergreen Browser provides an automated installer build pipeline that produces `EvergreenBrowserSetup.exe`:

### Build Command
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1
```

### What this produces:
- **`dist/EvergreenBrowserSetup.exe` (~3.14 MB)**: Standalone installer that embeds the release browser binary and high-DPI icon resources.
  - Installs to `%LOCALAPPDATA%\Programs\EvergreenBrowser\` without requiring UAC elevation.
  - Generates crisp Start Menu and Desktop shortcuts.
  - Registers in Windows Settings (`Apps` > `Installed apps`) with full uninstaller support (`--uninstall`).
  - Launches the browser immediately upon completion.
- **`dist/evergreen-browser.exe` (~1.83 MB)**: Standalone binary for portable usage.

### Regenerating High-DPI Icon Assets
If you modify `crates/app/ui/logo.png`, regenerate the 7-layer multi-resolution icon suite (16x16 to 256x256) and RGBA buffers via:
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\generate-high-dpi-icons.ps1
```


