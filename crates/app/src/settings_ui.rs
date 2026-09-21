//! Internal Settings Tab for Evergreen Browser
//! Styled in Direction 01 — Fluent Desktop (WinUI 3 / Edge Dev minimal).

pub const LOGO_BASE64: &str = include_str!("../ui/logo.b64");

pub const SETTINGS_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Settings - Evergreen</title>
  <link rel="icon" type="image/png" href="data:image/png;base64,{{LOGO_BASE64}}">
  <style>
    :root {
      --bg-surface: #181820;
      --bg-card: rgba(255, 255, 255, 0.035);
      --bg-card-hover: rgba(255, 255, 255, 0.05);
      --border-card: rgba(255, 255, 255, 0.07);
      --border-separator: rgba(255, 255, 255, 0.05);
      --text-main: #f0f0f5;
      --text-muted: #9595a8;
      --accent-blue: #4e8cff;
      --accent-blue-hover: #3d7ae8;
      --accent-green: #34d399;
      --accent-danger: #ef4444;
      --font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, -apple-system, sans-serif;
    }

    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }

    body {
      background-color: var(--bg-surface);
      color: var(--text-main);
      font-family: var(--font-family);
      padding: 48px 32px 80px;
      line-height: 1.5;
      user-select: none;
    }

    .container {
      max-width: 760px;
      margin: 0 auto;
    }

    /* Header */
    .header {
      margin-bottom: 32px;
    }

    .breadcrumb {
      font-size: 12px;
      color: var(--text-muted);
      margin-bottom: 8px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .page-title {
      font-size: 26px;
      font-weight: 600;
      letter-spacing: -0.4px;
    }

    /* Section & Cards */
    .section-title {
      font-size: 14px;
      font-weight: 600;
      color: var(--text-muted);
      margin: 28px 0 12px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .card {
      background: var(--bg-card);
      border: 1px solid var(--border-card);
      border-radius: 12px;
      overflow: hidden;
      margin-bottom: 20px;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    }

    .row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 18px 24px;
      border-bottom: 1px solid var(--border-separator);
    }

    .row:last-child {
      border-bottom: none;
    }

    .row-info {
      display: flex;
      flex-direction: column;
      gap: 3px;
    }

    .row-label {
      font-size: 14px;
      font-weight: 500;
      color: var(--text-main);
    }

    .row-desc {
      font-size: 12px;
      color: var(--text-muted);
    }

    .row-action {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .val-badge {
      font-size: 12px;
      color: var(--text-muted);
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid var(--border-card);
      padding: 4px 10px;
      border-radius: 8px;
    }

    .val-badge.active {
      color: var(--accent-green);
      border-color: rgba(52, 211, 153, 0.3);
      background: rgba(52, 211, 153, 0.08);
      font-weight: 500;
    }

    /* Buttons */
    .btn {
      background: var(--accent-blue);
      color: #fff;
      border: none;
      border-radius: 8px;
      padding: 7px 16px;
      font-size: 13px;
      font-family: inherit;
      font-weight: 500;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      transition: background 0.15s ease;
    }

    .btn:hover {
      background: var(--accent-blue-hover);
    }

    .btn-secondary {
      background: rgba(255, 255, 255, 0.08);
      color: var(--text-main);
      border: 1px solid var(--border-card);
    }

    .btn-secondary:hover {
      background: rgba(255, 255, 255, 0.12);
    }

    /* Status Banner */
    .status-banner {
      display: none;
      padding: 12px 18px;
      border-radius: 8px;
      font-size: 13px;
      margin-top: 16px;
      align-items: center;
      gap: 8px;
    }

    .status-banner.show {
      display: flex;
    }

    .status-banner.info {
      background: rgba(78, 140, 255, 0.12);
      border: 1px solid rgba(78, 140, 255, 0.3);
      color: #93c5fd;
    }

    .select-box {
      background: rgba(255, 255, 255, 0.08);
      border: 1px solid var(--border);
      color: var(--text-main);
      padding: 6px 14px;
      border-radius: 8px;
      font-size: 13px;
      outline: none;
      cursor: pointer;
      font-family: inherit;
    }

    .select-box option {
      background: #1e1e28;
      color: var(--text-main);
    }

    /* Toggle Switch */
    .switch {
      position: relative;
      display: inline-block;
      width: 40px;
      height: 22px;
    }

    .switch input {
      opacity: 0;
      width: 0;
      height: 0;
    }

    .slider {
      position: absolute;
      cursor: pointer;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background-color: rgba(255, 255, 255, 0.1);
      transition: .2s;
      border-radius: 22px;
      border: 1px solid var(--border-card);
    }

    .slider:before {
      position: absolute;
      content: "";
      height: 14px;
      width: 14px;
      left: 3px;
      bottom: 3px;
      background-color: #fff;
      transition: .2s;
      border-radius: 50%;
    }

    input:checked + .slider {
      background-color: var(--accent-blue);
      border-color: var(--accent-blue);
    }

    input:checked + .slider:before {
      transform: translateX(18px);
    }
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <div class="breadcrumb">System / Configuration</div>
      <h1 class="page-title">Settings</h1>
    </div>

    <div class="section-title">Engine & Runtime</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Rendering Engine</span>
          <span class="row-desc">Microsoft WebView2 Evergreen Runtime</span>
        </div>
        <div class="row-action">
          <span class="val-badge" id="engineVersion">Detecting...</span>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Engine Updates</span>
          <span class="row-desc">Invoke official Microsoft bootstrapper to check for latest runtime updates</span>
        </div>
        <div class="row-action">
          <button class="btn" onclick="sendAction('RunEngineUpdate')">Update Engine Now</button>
        </div>
      </div>
    </div>

    <div class="section-title">Search & Navigation</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Default Search Engine</span>
          <span class="row-desc">Used when searching directly from the address bar or start page</span>
        </div>
        <div class="row-action">
          <select id="searchEngineSelect" class="select-box" onchange="onSearchEngineChange(this.value)">
            <option id="opt_duckduckgo" value="duckduckgo">DuckDuckGo</option>
            <option id="opt_google" value="google">Google</option>
            <option id="opt_bing" value="bing">Bing</option>
            <option id="opt_brave" value="brave">Brave Search</option>
            <option id="opt_ecosia" value="ecosia">Ecosia</option>
          </select>
        </div>
      </div>
    </div>

    <div class="section-title">Downloads</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Ask Where to Save</span>
          <span class="row-desc">Always ask for confirmation and destination folder before starting any download</span>
        </div>
        <div class="row-action">
          <label class="switch">
            <input type="checkbox" id="askWhereToSaveToggle" checked onchange="onToggleChange('askWhereToSave', this.checked)">
            <span class="slider"></span>
          </label>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Show Download Progress</span>
          <span class="row-desc">Display animated circular progress ring in the top toolbar</span>
        </div>
        <div class="row-action">
          <label class="switch">
            <input type="checkbox" id="showProgressToolbarToggle" checked onchange="onToggleChange('showProgressToolbar', this.checked)">
            <span class="slider"></span>
          </label>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Default Location</span>
          <span class="row-desc" id="downloadFolderDesc">{{DEFAULT_DOWNLOAD_FOLDER}}</span>
        </div>
        <div class="row-action">
          <button class="btn btn-secondary" onclick="sendAction('ChangeDownloadFolder')">Browse...</button>
        </div>
      </div>
    </div>

    <div class="section-title">Tabs & Windows</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Tab Drag Reordering</span>
          <span class="row-desc">Reorder tabs smoothly along the tab strip by dragging and dropping</span>
        </div>
        <div class="row-action">
          <label class="switch">
            <input type="checkbox" id="tabReorderingToggle" checked onchange="onToggleChange('tabReordering', this.checked)">
            <span class="slider"></span>
          </label>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Tab Tear-Off to Window</span>
          <span class="row-desc">Detach a tab into an independent browser window when dragged outside the top bar</span>
        </div>
        <div class="row-action">
          <label class="switch">
            <input type="checkbox" id="tabTearoffToggle" checked onchange="onToggleChange('tabTearoff', this.checked)">
            <span class="slider"></span>
          </label>
        </div>
      </div>
    </div>

    <div class="section-title">Appearance & Zoom</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Default Page Zoom</span>
          <span class="row-desc">Set the default scale factor for rendered web content</span>
        </div>
        <div class="row-action">
          <select id="defaultZoomSelect" class="select-box" onchange="onDefaultZoomChange(this.value)">
            <option value="0.75">75%</option>
            <option value="0.9">90%</option>
            <option value="1.0" selected>100% (Default)</option>
            <option value="1.1">110%</option>
            <option value="1.25">125%</option>
            <option value="1.5">150%</option>
          </select>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Show Zoom Indicator</span>
          <span class="row-desc">Display zoom percentage badge on the right side of the omnibox when zoomed</span>
        </div>
        <div class="row-action">
          <label class="switch">
            <input type="checkbox" id="showZoomBadgeToggle" checked onchange="onToggleChange('showZoomBadge', this.checked)">
            <span class="slider"></span>
          </label>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Interactive Link Preview (Peek)</span>
          <span class="row-desc">Preview link destination in a floating preview card on hover or shortcut</span>
        </div>
        <div class="row-action">
          <label class="switch">
            <input type="checkbox" id="enableLinkPreviewToggle" checked onchange="onToggleChange('enableLinkPreview', this.checked)">
            <span class="slider"></span>
          </label>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Bottom Status URL Preview</span>
          <span class="row-desc">Show full target address in bottom-left status tooltip when hovering links</span>
        </div>
        <div class="row-action">
          <label class="switch">
            <input type="checkbox" id="showStatusPreviewToggle" checked onchange="onToggleChange('showStatusPreview', this.checked)">
            <span class="slider"></span>
          </label>
        </div>
      </div>
    </div>

    <div class="section-title">Site Permissions</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Location Access</span>
          <span class="row-desc">Permission policy when sites request device geolocation</span>
        </div>
        <div class="row-action">
          <select id="permLocationSelect" class="select-box" onchange="onPermissionChange('location', this.value)">
            <option value="ask" selected>Always Ask</option>
            <option value="allow">Allow</option>
            <option value="block">Block</option>
          </select>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Camera Access</span>
          <span class="row-desc">Permission policy when sites request video capture</span>
        </div>
        <div class="row-action">
          <select id="permCameraSelect" class="select-box" onchange="onPermissionChange('camera', this.value)">
            <option value="ask" selected>Always Ask</option>
            <option value="allow">Allow</option>
            <option value="block">Block</option>
          </select>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Microphone Access</span>
          <span class="row-desc">Permission policy when sites request audio capture</span>
        </div>
        <div class="row-action">
          <select id="permMicrophoneSelect" class="select-box" onchange="onPermissionChange('microphone', this.value)">
            <option value="ask" selected>Always Ask</option>
            <option value="allow">Allow</option>
            <option value="block">Block</option>
          </select>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Notifications</span>
          <span class="row-desc">Permission policy when sites request web notifications</span>
        </div>
        <div class="row-action">
          <select id="permNotificationsSelect" class="select-box" onchange="onPermissionChange('notifications', this.value)">
            <option value="ask" selected>Always Ask</option>
            <option value="allow">Allow</option>
            <option value="block">Block</option>
          </select>
        </div>
      </div>
    </div>

    <div class="section-title">Privacy & Isolation</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Session Privacy</span>
          <span class="row-desc">All cookies, browsing history, and cache partitions are ephemeral</span>
        </div>
        <div class="row-action">
          <span class="val-badge active">Ephemeral Mode Active</span>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Storage Guarantees</span>
          <span class="row-desc">Zero user data is retained to persistent storage across browser sessions</span>
        </div>
        <div class="row-action">
          <span class="val-badge">Zero Traces</span>
        </div>
      </div>
    </div>

    <div class="section-title">Architecture & Developer</div>
    <div class="card">
      <div class="row">
        <div class="row-info">
          <span class="row-label">Native Shell</span>
          <span class="row-desc">Lightweight Rust + Win32 child HWND architecture (sub-1MB executable)</span>
        </div>
        <div class="row-action">
          <span class="val-badge">64-bit Native</span>
        </div>
      </div>
      <div class="row">
        <div class="row-info">
          <span class="row-label">Local Fork Update</span>
          <span class="row-desc">Trigger a local source compilation to rebuild and verify Evergreen</span>
        </div>
        <div class="row-action">
          <button class="btn btn-secondary" onclick="sendAction('RunForkUpdate')">Run Fork Update</button>
        </div>
      </div>
    </div>

    <div id="statusBanner" class="status-banner info">
      <span id="statusText">Command initiated...</span>
    </div>
  </div>

  <script>
    function sendAction(action) {
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({ action: action, payload: null }));
        showStatus('Initiated ' + action + '...');
      } else {
        console.log('[IPC OUT]', action);
        showStatus('Simulated action: ' + action);
      }
    }

    function showStatus(text) {
      const banner = document.getElementById('statusBanner');
      const textEl = document.getElementById('statusText');
      textEl.textContent = text;
      banner.classList.add('show');
      setTimeout(() => banner.classList.remove('show'), 4000);
    }

    function onSearchEngineChange(engine) {
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({ action: 'SetSearchEngine', payload: { engine: engine } }));
        showStatus('Default search engine set to ' + engine);
      }
    }

    function onToggleChange(setting, value) {
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({
          action: 'SaveSettings',
          payload: { settings_json: JSON.stringify({ [setting]: value }) }
        }));
        showStatus('Updated ' + setting + ' to ' + value);
      }
    }

    function onDefaultZoomChange(zoom) {
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({
          action: 'SaveSettings',
          payload: { settings_json: JSON.stringify({ default_zoom_level: parseFloat(zoom) }) }
        }));
        showStatus('Default zoom set to ' + Math.round(parseFloat(zoom) * 100) + '%');
      }
    }

    function onPermissionChange(perm, value) {
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({
          action: 'SaveSettings',
          payload: { settings_json: JSON.stringify({ permissions: { [perm]: value } }) }
        }));
        showStatus('Permission for ' + perm + ' set to ' + value);
      }
    }

    window.__syncSearchEngine = function(engine) {
      const select = document.getElementById('searchEngineSelect');
      if (select && engine) {
        select.value = engine.toLowerCase();
      }
    };

    window.__syncEngineInfo = function(version) {
      const el = document.getElementById('engineVersion');
      if (el) el.textContent = version || 'v153.0.4234.32 (Active)';
    };

    if (window.__runtimeVersion) {
      window.__syncEngineInfo(window.__runtimeVersion);
    }
  </script>
</body>
</html>
"#;

use evergreen_core::settings::Settings;

pub fn get_settings_html(runtime_ver: &str, settings: &Settings) -> String {
    let download_dir_str = settings.downloads.default_folder.to_string_lossy();
    let mut html = SETTINGS_TEMPLATE
        .replace("{{LOGO_BASE64}}", LOGO_BASE64.trim())
        .replace("Detecting...", &format!("v{} (Active)", runtime_ver))
        .replace("{{DEFAULT_DOWNLOAD_FOLDER}}", &download_dir_str);

    let engine = settings.search_engine.to_lowercase();
    html = html.replace(&format!("id=\"opt_{}\"", engine), &format!("id=\"opt_{}\" selected", engine));

    if !settings.downloads.ask_where_to_save {
        html = html.replace("id=\"askWhereToSaveToggle\" checked", "id=\"askWhereToSaveToggle\"");
    }
    if !settings.downloads.show_progress_toolbar {
        html = html.replace("id=\"showProgressToolbarToggle\" checked", "id=\"showProgressToolbarToggle\"");
    }
    if !settings.tabs.enable_tab_reordering {
        html = html.replace("id=\"tabReorderingToggle\" checked", "id=\"tabReorderingToggle\"");
    }
    if !settings.tabs.enable_tab_tearoff {
        html = html.replace("id=\"tabTearoffToggle\" checked", "id=\"tabTearoffToggle\"");
    }
    if !settings.appearance.show_zoom_badge {
        html = html.replace("id=\"showZoomBadgeToggle\" checked", "id=\"showZoomBadgeToggle\"");
    }
    if !settings.appearance.enable_link_preview {
        html = html.replace("id=\"enableLinkPreviewToggle\" checked", "id=\"enableLinkPreviewToggle\"");
    }
    if !settings.appearance.show_status_preview {
        html = html.replace("id=\"showStatusPreviewToggle\" checked", "id=\"showStatusPreviewToggle\"");
    }

    html
}
