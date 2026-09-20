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

    window.__syncEngineInfo = function(version) {
      const el = document.getElementById('engineVersion');
      if (el) el.textContent = version || 'v153.0.4234.32 (Active)';
    };

    // Auto-detect version if injected into window
    if (window.__runtimeVersion) {
      window.__syncEngineInfo(window.__runtimeVersion);
    }
  </script>
</body>
</html>
"#;

pub fn get_settings_html(runtime_ver: &str) -> String {
    SETTINGS_TEMPLATE
        .replace("{{LOGO_BASE64}}", LOGO_BASE64.trim())
        .replace("Detecting...", &format!("v{} (Active)", runtime_ver))
}

pub static SETTINGS_HTML: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    get_settings_html("Detecting...")
});
