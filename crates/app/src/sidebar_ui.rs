//! Sidebar / Side-Panel UI for Evergreen Browser
//! Provides modern WinUI 3 Fluent slide-out panel for both:
//! 1) 3-Dot Browser Menu (New Tab, New Window, DevTools, Settings)
//! 2) Site Information & Certificate Inspector (HTTPS status, encryption, ephemeral storage)

use std::sync::LazyLock;

pub const LOGO_BASE64: &str = include_str!("../ui/logo.b64");

pub const SIDEBAR_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Evergreen Panel</title>
  <link rel="icon" type="image/png" href="data:image/png;base64,{{LOGO_BASE64}}">
  <style>
    :root {
      --bg-panel: #1e1e26;
      --bg-card: #272733;
      --bg-card-hover: #313142;
      --border-subtle: rgba(255, 255, 255, 0.08);
      --border-focus: #4e8cff;
      --text-main: #f3f3f8;
      --text-muted: #9595a6;
      --accent-green: #34d399;
      --accent-amber: #f59e0b;
      --accent-blue: #4e8cff;
      --danger-color: #ef4444;
      --font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, -apple-system, sans-serif;
    }

    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
      user-select: none;
    }

    body {
      background-color: var(--bg-panel);
      color: var(--text-main);
      font-family: var(--font-family);
      font-size: 13px;
      height: 100vh;
      display: flex;
      flex-direction: column;
      border-left: 1px solid var(--border-subtle);
      overflow-y: auto;
      scrollbar-width: thin;
      scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
    }

    svg {
      stroke: currentColor;
      stroke-width: 2;
      stroke-linecap: round;
      stroke-linejoin: round;
      fill: none;
      flex-shrink: 0;
    }

    /* Panel Header */
    .panel-header {
      height: 52px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 16px;
      border-bottom: 1px solid var(--border-subtle);
      background: rgba(30, 30, 38, 0.95);
      position: sticky;
      top: 0;
      z-index: 10;
    }

    .header-title-group {
      display: flex;
      align-items: center;
      gap: 10px;
    }

    .header-icon {
      width: 20px;
      height: 20px;
      color: var(--accent-blue);
    }

    .header-title {
      font-size: 14px;
      font-weight: 600;
      letter-spacing: -0.2px;
    }

    .btn-close {
      width: 28px;
      height: 28px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: transparent;
      border: none;
      color: var(--text-muted);
      border-radius: 6px;
      cursor: pointer;
      transition: background 0.1s ease, color 0.1s ease;
    }

    .btn-close:hover {
      background: rgba(255, 255, 255, 0.1);
      color: var(--text-main);
    }

    .btn-close svg {
      width: 16px;
      height: 16px;
    }

    /* Mode Navigation Tabs */
    .mode-nav {
      display: flex;
      padding: 8px 16px 0;
      gap: 6px;
      border-bottom: 1px solid var(--border-subtle);
    }

    .mode-tab {
      padding: 6px 12px;
      font-size: 12px;
      font-weight: 500;
      color: var(--text-muted);
      background: transparent;
      border: none;
      border-bottom: 2px solid transparent;
      cursor: pointer;
      transition: color 0.1s ease, border-color 0.1s ease;
    }

    .mode-tab.active {
      color: var(--text-main);
      border-bottom-color: var(--accent-blue);
    }

    .mode-tab:hover:not(.active) {
      color: var(--text-main);
    }

    /* Content Area */
    .panel-content {
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 14px;
      flex: 1;
    }

    .view-section {
      display: none;
      flex-direction: column;
      gap: 14px;
    }

    .view-section.active {
      display: flex;
    }

    /* Card Containers */
    .card {
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      padding: 12px 14px;
      display: flex;
      flex-direction: column;
      gap: 8px;
    }

    .card-title {
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: var(--text-muted);
      font-weight: 600;
    }

    /* Action List Items (Menu Mode) */
    .action-list {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }

    .action-item {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 9px 12px;
      border-radius: 6px;
      color: var(--text-main);
      cursor: pointer;
      transition: background 0.12s ease;
    }

    .action-item:hover {
      background: var(--bg-card-hover);
    }

    .action-item svg {
      width: 16px;
      height: 16px;
      color: var(--text-muted);
    }

    .action-item:hover svg {
      color: var(--text-main);
    }

    .action-label {
      flex: 1;
      font-size: 13px;
    }

    .action-shortcut {
      font-size: 11px;
      color: var(--text-muted);
    }

    .action-divider {
      height: 1px;
      background: var(--border-subtle);
      margin: 4px 0;
    }

    .zoom-action-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 6px 12px;
      height: 36px;
    }

    .zoom-controls {
      display: flex;
      align-items: center;
      gap: 6px;
    }

    .zoom-btn {
      width: 24px;
      height: 24px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: 6px;
      color: var(--text-main);
      cursor: pointer;
      font-size: 13px;
      font-weight: 600;
      transition: background 0.1s ease;
    }

    .zoom-btn:hover {
      background: var(--bg-card-hover);
    }

    .zoom-val {
      font-size: 11px;
      font-weight: 500;
      color: var(--text-muted);
      min-width: 38px;
      text-align: center;
      cursor: pointer;
      padding: 2px 4px;
      border-radius: 4px;
      transition: background 0.1s ease, color 0.1s ease;
    }

    .zoom-val:hover {
      background: var(--bg-card-hover);
      color: var(--text-main);
    }

    /* Security Status Banner */
    .security-status-card {
      padding: 14px;
      border-radius: 8px;
      display: flex;
      flex-direction: column;
      gap: 6px;
    }

    .security-status-card.secure {
      background: rgba(52, 211, 153, 0.1);
      border: 1px solid rgba(52, 211, 153, 0.25);
    }

    .security-status-card.insecure {
      background: rgba(245, 158, 11, 0.1);
      border: 1px solid rgba(245, 158, 11, 0.25);
    }

    .security-status-card.internal {
      background: rgba(78, 140, 255, 0.1);
      border: 1px solid rgba(78, 140, 255, 0.25);
    }

    .status-badge-row {
      display: flex;
      align-items: center;
      gap: 8px;
      font-weight: 600;
      font-size: 13px;
    }

    .security-status-card.secure .status-badge-row { color: var(--accent-green); }
    .security-status-card.insecure .status-badge-row { color: var(--accent-amber); }
    .security-status-card.internal .status-badge-row { color: var(--accent-blue); }

    .status-desc {
      font-size: 12px;
      color: var(--text-muted);
      line-height: 1.4;
    }

    /* Property Row */
    .prop-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 4px 0;
      font-size: 12px;
    }

    .prop-label {
      color: var(--text-muted);
    }

    .prop-value {
      font-weight: 500;
      color: var(--text-main);
      text-align: right;
      max-width: 180px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .btn-cert-details {
      width: 100%;
      margin-top: 10px;
      padding: 8px 12px;
      background: var(--bg-card-hover);
      border: 1px solid var(--border-subtle);
      border-radius: 6px;
      color: var(--text-main);
      font-size: 12px;
      font-family: inherit;
      font-weight: 500;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      cursor: pointer;
      transition: background 0.15s ease, border-color 0.15s ease, transform 0.1s ease;
    }

    .btn-cert-details:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: var(--border-focus);
    }

    .btn-cert-details:active {
      transform: scale(0.98);
    }

    /* Privacy Guarantee Pill */
    .privacy-pill {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      background: rgba(52, 211, 153, 0.1);
      border: 1px solid rgba(52, 211, 153, 0.25);
      color: var(--accent-green);
      font-size: 11px;
      font-weight: 500;
      padding: 4px 10px;
      border-radius: 12px;
      width: fit-content;
    }
  </style>
</head>
<body>
  <!-- Header -->
  <div class="panel-header">
    <div class="header-title-group">
      <svg class="header-icon" id="panelHeaderIcon" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="1.5"/>
        <circle cx="12" cy="5" r="1.5"/>
        <circle cx="12" cy="19" r="1.5"/>
      </svg>
      <span class="header-title" id="panelHeaderTitle">Evergreen Menu</span>
    </div>
    <button class="btn-close" onclick="sendAction('CloseSidebar')" title="Close panel" aria-label="Close">
      <svg viewBox="0 0 24 24"><path d="M18 6L6 18M6 6l12 12"/></svg>
    </button>
  </div>

  <!-- Mode Switcher Tabs -->
  <div class="mode-nav">
    <button class="mode-tab active" id="tabMenu" onclick="switchMode('menu')">Browser Menu</button>
    <button class="mode-tab" id="tabSecurity" onclick="switchMode('security')">Site Information</button>
  </div>

  <div class="panel-content">
    <!-- VIEW 1: Browser Menu Mode -->
    <div class="view-section active" id="viewMenu">
      <div class="card action-list">
        <div class="action-item" onclick="sendAction('CreateTab')">
          <svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>
          <span class="action-label">New tab</span>
          <span class="action-shortcut">Ctrl+T</span>
        </div>
        <div class="action-item" onclick="sendAction('OpenNewWindow')">
          <svg viewBox="0 0 24 24"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18"/></svg>
          <span class="action-label">New window</span>
          <span class="action-shortcut">Ctrl+N</span>
        </div>
        <div class="action-item" onclick="sendAction('Reload')">
          <svg viewBox="0 0 24 24">
            <path d="M21 2v6h-6"/>
            <path d="M3 12a9 9 0 0 1 15-6.7L21 8"/>
            <path d="M3 22v-6h6"/>
            <path d="M21 12a9 9 0 0 1-15 6.7L3 16"/>
          </svg>
          <span class="action-label">Reload page</span>
          <span class="action-shortcut">Ctrl+R</span>
        </div>
        <div class="action-divider"></div>
        <!-- Zoom Controls -->
        <div class="zoom-action-row">
          <span class="action-label">Zoom</span>
          <div class="zoom-controls">
            <button class="zoom-btn" id="sidebarZoomOut" onclick="sendAction('ZoomOut')" title="Zoom Out (Ctrl+-)">−</button>
            <span class="zoom-val" id="sidebarZoomVal" onclick="sendAction('ZoomReset')" title="Reset (Ctrl+0)">100%</span>
            <button class="zoom-btn" id="sidebarZoomIn" onclick="sendAction('ZoomIn')" title="Zoom In (Ctrl++)">+</button>
          </div>
        </div>
        <div class="action-item" onclick="sendAction('OpenFindInPage')">
          <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
          <span class="action-label">Find in page</span>
          <span class="action-shortcut">Ctrl+F</span>
        </div>
        <div class="action-item" onclick="sendAction('OpenDownloads')">
          <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>
          <span class="action-label">Downloads</span>
          <span class="action-shortcut">Ctrl+J</span>
        </div>
        <div class="action-divider"></div>
        <div class="action-item" onclick="sendAction('OpenDevTools')">
          <svg viewBox="0 0 24 24"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
          <span class="action-label">Developer tools</span>
          <span class="action-shortcut">F12</span>
        </div>
        <div class="action-item" onclick="sendAction('OpenSettings')">
          <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
          <span class="action-label">Settings</span>
        </div>
      </div>

      <!-- Ephemeral Mode Status Card -->
      <div class="card">
        <span class="card-title">Session Protection</span>
        <div class="privacy-pill">
          <svg viewBox="0 0 24 24" style="width: 12px; height: 12px;"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
          Ephemeral Mode Active
        </div>
        <p class="status-desc">All cache, cookies, and browsing partitions are isolated in volatile memory and wiped upon exiting.</p>
      </div>
    </div>

    <!-- VIEW 2: Site Security & Certificate Mode -->
    <div class="view-section" id="viewSecurity">
      <!-- Status Banner -->
      <div class="security-status-card secure" id="securityBanner">
        <div class="status-badge-row">
          <svg id="statusIcon" viewBox="0 0 24 24" style="width: 18px; height: 18px;">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          <span id="statusTitle">Connection is secure</span>
        </div>
        <p class="status-desc" id="statusDesc">Your information (for example, passwords or credit card numbers) is private when sent to this site.</p>
      </div>

      <!-- Certificate & Encryption Details -->
      <div class="card">
        <span class="card-title">Certificate & Cryptography</span>
        <div class="prop-row">
          <span class="prop-label">Host</span>
          <span class="prop-value" id="propHost">example.com</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Status</span>
          <span class="prop-value" id="propCert">Valid (System Verified)</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Issued To</span>
          <span class="prop-value" id="propSubject">CN=example.com</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Issued By</span>
          <span class="prop-value" id="propIssuer">DigiCert Global Root</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Validity</span>
          <span class="prop-value" id="propValid">Active</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Algorithm</span>
          <span class="prop-value" id="propSigAlg">sha256ECDSA</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Thumbprint</span>
          <span class="prop-value" id="propThumbprint" style="font-family: monospace; font-size: 11px;">D3B63E...</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Protocol</span>
          <span class="prop-value" id="propProtocol">TLS 1.3</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Encryption</span>
          <span class="prop-value" id="propCipher">256-bit AES-GCM</span>
        </div>

        <button class="btn-cert-details" id="btnOpenCertDialog" onclick="openCertificateDialog()">
          <svg viewBox="0 0 24 24" style="width: 14px; height: 14px;"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>
          View Certificate Details (Windows Dialog)
        </button>
      </div>

      <!-- Ephemeral Privacy -->
      <div class="card">
        <span class="card-title">Storage & Partitioning</span>
        <div class="prop-row">
          <span class="prop-label">Persistent Disk Cookies</span>
          <span class="prop-value" style="color: var(--accent-green);">0 (RAM Only)</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Cache Storage</span>
          <span class="prop-value">Ephemeral</span>
        </div>
      </div>

      <!-- Site Permissions -->
      <div class="card">
        <span class="card-title">Site Permissions</span>
        <div class="prop-row">
          <span class="prop-label">Location</span>
          <span class="prop-value">Blocked</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Camera & Microphone</span>
          <span class="prop-value">Blocked</span>
        </div>
        <div class="prop-row">
          <span class="prop-label">Notifications</span>
          <span class="prop-value">Blocked</span>
        </div>
      </div>
    </div>
  </div>

  <script>
    let currentMode = 'menu';

    function postIpc(action, payload = null) {
      if (window.ipc) {
        window.ipc.postMessage(JSON.stringify({ action, payload }));
      } else {
        console.log('[SIDEBAR IPC]', action, payload);
      }
    }

    function sendAction(action) {
      if (action === 'CreateTab') {
        postIpc('CreateTab', { url: null });
      } else if (action === 'OpenSettings') {
        postIpc('OpenSettings');
      } else {
        postIpc(action);
      }
    }

    function switchMode(mode) {
      currentMode = mode;
      const tabMenu = document.getElementById('tabMenu');
      const tabSec = document.getElementById('tabSecurity');
      const viewMenu = document.getElementById('viewMenu');
      const viewSec = document.getElementById('viewSecurity');
      const title = document.getElementById('panelHeaderTitle');
      const icon = document.getElementById('panelHeaderIcon');

      if (mode === 'security') {
        tabSec.classList.add('active');
        tabMenu.classList.remove('active');
        viewSec.classList.add('active');
        viewMenu.classList.remove('active');
        title.textContent = 'Site Information';
        icon.innerHTML = '<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>';
      } else {
        tabMenu.classList.add('active');
        tabSec.classList.remove('active');
        viewMenu.classList.add('active');
        viewSec.classList.remove('active');
        title.textContent = 'Evergreen Menu';
        icon.innerHTML = '<circle cx="12" cy="12" r="1.5"/><circle cx="12" cy="5" r="1.5"/><circle cx="12" cy="19" r="1.5"/>';
      }
    }

    function openCertificateDialog() {
      const host = document.getElementById('propHost').textContent;
      postIpc('OpenCertificateDialog', { host });
    }

    // Called by host to update sidebar data
    window.__sidebarSync = function(msg) {
      if (!msg) return;
      const data = msg.data || msg;
      if (data.mode) {
        switchMode(data.mode);
      }

      const sec = data.security_info;
      if (sec) {
        document.getElementById('propHost').textContent = sec.host || 'Unknown';
        document.getElementById('propCert').textContent = sec.certificate_status || 'Unknown';
        document.getElementById('propProtocol').textContent = sec.protocol || 'Unknown';
        document.getElementById('propCipher').textContent = sec.cipher || 'Standard';

        if (document.getElementById('propSubject')) {
          document.getElementById('propSubject').textContent = sec.subject || 'None';
        }
        if (document.getElementById('propIssuer')) {
          document.getElementById('propIssuer').textContent = sec.issuer || 'None';
        }
        if (document.getElementById('propValid')) {
          document.getElementById('propValid').textContent = (sec.valid_from && sec.valid_to) ? (sec.valid_from + ' to ' + sec.valid_to) : 'Valid';
        }
        if (document.getElementById('propThumbprint')) {
          document.getElementById('propThumbprint').textContent = sec.thumbprint || 'None';
        }
        if (document.getElementById('propSigAlg')) {
          document.getElementById('propSigAlg').textContent = sec.signature_algorithm || 'None';
        }

        const banner = document.getElementById('securityBanner');
        const icon = document.getElementById('statusIcon');
        const title = document.getElementById('statusTitle');
        const desc = document.getElementById('statusDesc');

        banner.className = 'security-status-card';
        if (sec.host.startsWith('evergreen://') || sec.protocol.includes('Sandbox') || sec.protocol.includes('Local')) {
          banner.classList.add('internal');
          icon.innerHTML = '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>';
          title.textContent = 'Secure Sandbox Surface';
          desc.textContent = 'This is a local, sandboxed Evergreen Browser system surface.';
        } else if (sec.is_secure) {
          banner.classList.add('secure');
          icon.innerHTML = '<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>';
          title.textContent = 'Connection is secure';
          desc.textContent = 'Your information (for example, passwords or credit card numbers) is private when sent to this site.';
        } else {
          banner.classList.add('insecure');
          icon.innerHTML = '<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/>';
          title.textContent = 'Connection is not secure';
          desc.textContent = 'You should not enter any sensitive information on this site (passwords, cards), as it could be intercepted by attackers.';
        }
      }
    };

    window.__syncZoom = function(factor) {
      const el = document.getElementById('sidebarZoomVal');
      if (el) el.textContent = Math.round(factor * 100) + '%';
    };
  </script>
</body>
</html>
"#;

pub fn get_sidebar_html() -> String {
    SIDEBAR_TEMPLATE.replace("{{LOGO_BASE64}}", LOGO_BASE64.trim())
}

pub static SIDEBAR_HTML: LazyLock<String> = LazyLock::new(get_sidebar_html);
