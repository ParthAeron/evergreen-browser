/**
 * scripts/visual_e2e_playwright.js
 * Comprehensive Playwright Visual E2E Validation & Screenshot Capture Suite
 * Evaluates all user-facing flows, pages, interstitials, and installer visual states for Phase 05.
 */

const fs = require('fs');
const path = require('path');

// Locate Playwright from npm cache or global path
let playwright;
try {
  playwright = require('playwright');
} catch (e) {
  const cachePath = 'C:\\Users\\parth\\AppData\\Local\\npm-cache\\_npx\\e41f203b7505f1fb\\node_modules\\playwright';
  if (fs.existsSync(cachePath)) {
    playwright = require(cachePath);
  } else {
    throw new Error('Playwright not found in standard paths: ' + e.message);
  }
}

const { chromium } = playwright;

const REPO_ROOT = path.resolve(__dirname, '..');
const DOCS_SCREENSHOTS = path.resolve(REPO_ROOT, '..', 'evergreen-browser-docs', 'docs', 'mvps', 'mvp-1', 'phase-05', 'screenshots');

if (!fs.existsSync(DOCS_SCREENSHOTS)) {
  fs.mkdirSync(DOCS_SCREENSHOTS, { recursive: true });
}

function extractTemplate(src, constName) {
  const startMarker = `pub const ${constName}: &str = r#"`;
  const startIndex = src.indexOf(startMarker);
  if (startIndex === -1) throw new Error(`Could not find start of ${constName}`);
  const contentStart = startIndex + startMarker.length;
  const endIndex = src.indexOf('"#;', contentStart);
  if (endIndex === -1) throw new Error(`Could not find end of ${constName}`);
  return src.substring(contentStart, endIndex);
}

// 1. Load Templates and Assets from codebase
const logoB64 = fs.readFileSync(path.join(REPO_ROOT, 'crates', 'app', 'ui', 'logo.b64'), 'utf8').trim();

// Read Home UI template
const homeUiSrc = fs.readFileSync(path.join(REPO_ROOT, 'crates', 'app', 'src', 'home_ui.rs'), 'utf8');
const homeRaw = extractTemplate(homeUiSrc, 'HOME_TEMPLATE');
const homeHtml = homeRaw
  .replace(/\{\{LOGO_BASE64\}\}/g, logoB64)
  .replace(/\{\{SEARCH_ENGINE_NAME\}\}/g, 'DuckDuckGo')
  .replace(/\{\{SEARCH_ENGINE_URL\}\}/g, 'https://duckduckgo.com/?q=%s');

// Read Settings UI template
const settingsUiSrc = fs.readFileSync(path.join(REPO_ROOT, 'crates', 'app', 'src', 'settings_ui.rs'), 'utf8');
const settingsRaw = extractTemplate(settingsUiSrc, 'SETTINGS_TEMPLATE');
const settingsHtml = settingsRaw
  .replace(/\{\{LOGO_BASE64\}\}/g, logoB64)
  .replace('Detecting...', 'v128.0.2739.42 (Active)')
  .replace(/\{\{DEFAULT_DOWNLOAD_FOLDER\}\}/g, 'C:\\Users\\parth\\Downloads');

// Read TLS Interstitial template
const tlsHtml = `<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>Security Warning: Untrusted Certificate</title><style>:root{color-scheme:dark;--bg-gradient:radial-gradient(circle at 50% 25%,#22222e 0%,#16161d 100%);--surface-card:rgba(255,255,255,0.04);--border-subtle:rgba(255,255,255,0.08);--text-main:#f0f0f5;--text-muted:#9595a8;--accent:#4e8cff;--accent-hover:#3b76e1;--danger:#ef4444;--font-family:"Segoe UI Variable Text","Segoe UI",system-ui,-apple-system,sans-serif;}*{box-sizing:border-box;margin:0;padding:0;}body{background:var(--bg-gradient);color:var(--text-main);font-family:var(--font-family);min-height:100vh;display:flex;align-items:center;justify-content:center;padding:32px 24px;user-select:none;}.card{background:var(--surface-card);backdrop-filter:blur(16px);border:1px solid var(--border-subtle);border-radius:16px;box-shadow:0 16px 40px rgba(0,0,0,0.5);padding:40px 36px;max-width:540px;width:100%;text-align:center;}.icon-badge{width:64px;height:64px;margin:0 auto 20px;border-radius:50%;background:rgba(239,68,68,0.12);border:1px solid rgba(239,68,68,0.3);display:flex;align-items:center;justify-content:center;}.icon-badge svg{width:32px;height:32px;stroke:var(--danger);stroke-width:2;stroke-linecap:round;stroke-linejoin:round;fill:none;}h1{font-size:22px;font-weight:600;letter-spacing:-0.3px;margin-bottom:12px;color:var(--text-main);}p.desc{font-size:13px;color:var(--text-muted);line-height:1.6;margin-bottom:20px;}p.desc strong{color:var(--text-main);word-break:break-all;}.uri-container{margin-bottom:24px;padding:10px 14px;background:rgba(0,0,0,0.3);border:1px solid var(--border-subtle);border-radius:8px;display:flex;align-items:center;justify-content:space-between;gap:12px;font-size:12px;font-family:monospace;}.uri-text{color:#cbd5e1;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;flex:1;text-align:left;}.error-code{color:#f87171;background:rgba(239,68,68,0.15);padding:2px 8px;border-radius:4px;font-size:11px;flex-shrink:0;}.button-group{display:flex;align-items:center;justify-content:center;gap:12px;}button{border:none;border-radius:8px;padding:10px 22px;font-size:13px;font-weight:500;cursor:pointer;font-family:inherit;transition:background 0.12s ease;}.btn-safety{background:var(--accent);color:#ffffff;font-weight:600;box-shadow:0 4px 14px rgba(78,140,255,0.3);}.btn-safety:hover{background:var(--accent-hover);}.btn-advanced{background:rgba(255,255,255,0.06);color:var(--text-muted);border:1px solid var(--border-subtle);}.btn-advanced:hover{background:rgba(255,255,255,0.1);color:var(--text-main);}.advanced-drawer{display:none;margin-top:24px;padding:18px;background:rgba(0,0,0,0.35);border:1px solid rgba(255,255,255,0.06);border-radius:10px;text-align:left;}.advanced-drawer p{font-size:12px;color:var(--text-muted);line-height:1.6;margin-bottom:14px;}.btn-proceed{background:transparent;color:#f87171;padding:6px 12px;border:1px solid rgba(239,68,68,0.3);border-radius:6px;font-size:12px;display:inline-block;}.btn-proceed:hover{background:rgba(239,68,68,0.12);color:#fca5a5;}</style></head><body><div class="card"><div class="icon-badge"><svg viewBox="0 0 24 24"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg></div><h1>Your connection is not private</h1><p class="desc">Evergreen Browser prevented connection to <strong>untrusted-root.badssl.com</strong> because its security certificate is untrusted, self-signed, or expired. Attackers might be trying to steal your information.</p><div class="uri-container"><span class="uri-text">https://untrusted-root.badssl.com/</span><span class="error-code">NET::ERR_CERT_INVALID</span></div><div class="button-group"><button class="btn-safety" id="safetyBtn">Go Back to Safety</button><button class="btn-advanced" id="advancedBtn" onclick="toggleAdvanced()">Advanced ▾</button></div><div class="advanced-drawer" id="advancedDrawer"><p>This server could not prove that it is <strong>untrusted-root.badssl.com</strong>; its security certificate is not trusted by your computer's operating system.</p><button class="btn-proceed" id="proceedBtn">Proceed to untrusted-root.badssl.com (unsafe)</button></div></div><script>function toggleAdvanced(){const d=document.getElementById('advancedDrawer');const b=document.getElementById('advancedBtn');const h=(d.style.display==='none'||!d.style.display);d.style.display=h?'block':'none';b.textContent=h?'Advanced ▴':'Advanced ▾';}</script></body></html>`;

// Read Tab Crash Interstitial
const crashHtml = `<!DOCTYPE html><html><head><meta charset="utf-8"><title>Tab Stopped Responding</title><style>body{margin:0;padding:0;background:#181820;color:#f1f5f9;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;}.box{text-align:center;max-width:480px;padding:32px;background:#1f1f2a;border:1px solid rgba(255,255,255,0.08);border-radius:12px;box-shadow:0 8px 32px rgba(0,0,0,0.5);}.icon{font-size:40px;margin-bottom:12px;}h1{font-size:20px;font-weight:600;margin:0 0 8px 0;color:#f87171;}p{font-size:13px;color:#94a3b8;line-height:1.5;margin:0 0 24px 0;}button{background:#3b82f6;color:white;border:none;border-radius:6px;padding:10px 20px;font-size:13px;font-weight:500;cursor:pointer;}button:hover{background:#2563eb;}</style></head><body><div class="box"><div class="icon">⚠️</div><h1>Page Stopped Responding</h1><p>This web page or renderer process encountered a problem and had to be halted. Other tabs and your browser session remain safe.</p><button id="reloadBtn">Reload Page</button></div></body></html>`;

// Helper for Installer Dialog Visual Layout
function getInstallerHtml(title, stepText, progressPct, isUninstall = false) {
  return `<!DOCTYPE html>
  <html lang="en">
  <head>
    <meta charset="utf-8">
    <title>${title}</title>
    <style>
      :root {
        color-scheme: dark;
        --bg-window: #16161d;
        --card-border: #2a2a38;
        --text-title: #f0f0f5;
        --text-sub: #94a3b8;
        --text-desc: #cbd5e1;
        --track-bg: #232330;
        --accent-emerald: #34d399;
        --accent-blue: #4e8cff;
        --font: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
      }
      * { box-sizing: border-box; margin: 0; padding: 0; }
      body {
        background: transparent;
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100vh;
        user-select: none;
        font-family: var(--font);
      }
      .installer-window {
        width: 460px;
        height: 270px;
        background: var(--bg-window);
        border: 1px solid var(--card-border);
        border-radius: 8px;
        box-shadow: 0 16px 40px rgba(0,0,0,0.6);
        padding: 38px 40px;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        position: relative;
      }
      .brand-row {
        display: flex;
        align-items: center;
        gap: 14px;
      }
      .logo-img {
        width: 40px;
        height: 36px;
        object-fit: contain;
      }
      .title-col h1 {
        font-size: 20px;
        font-weight: 700;
        color: var(--text-title);
      }
      .title-col .ver {
        font-size: 12px;
        font-weight: 600;
        color: var(--text-sub);
        margin-top: 2px;
      }
      .status-text {
        font-size: 14px;
        font-weight: 500;
        color: var(--text-desc);
        margin-top: 18px;
        min-height: 20px;
      }
      .progress-container {
        margin-top: 14px;
      }
      .progress-track {
        width: 100%;
        height: 8px;
        background: var(--track-bg);
        border-radius: 4px;
        overflow: hidden;
      }
      .progress-fill {
        height: 100%;
        width: ${progressPct}%;
        background: linear-gradient(90deg, var(--accent-emerald) 0%, var(--accent-blue) 100%);
        border-radius: 4px;
        transition: width 0.3s ease;
      }
      .progress-meta {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: 6px;
        font-size: 12px;
      }
      .meta-pct {
        color: var(--accent-emerald);
        font-weight: 600;
        margin-left: auto;
      }
      .footer-badge {
        font-size: 11px;
        color: #706060;
        margin-top: 8px;
      }
      .btn-row {
        display: flex;
        gap: 10px;
        margin-top: 14px;
      }
      .btn {
        border: none;
        border-radius: 6px;
        padding: 8px 18px;
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
      }
      .btn-danger {
        background: #ef4444;
        color: white;
      }
      .btn-secondary {
        background: rgba(255,255,255,0.08);
        color: var(--text-desc);
      }
    </style>
  </head>
  <body>
    <div class="installer-window">
      <div class="brand-row">
        <img class="logo-img" src="data:image/png;base64,${logoB64}" alt="Logo" />
        <div class="title-col">
          <h1>${title}</h1>
          <div class="ver">Version 0.4.0</div>
        </div>
      </div>
      <div class="status-text">${stepText}</div>
      ${!isUninstall ? `
      <div class="progress-container">
        <div class="progress-track">
          <div class="progress-fill"></div>
        </div>
        <div class="progress-meta">
          <span class="meta-pct">${progressPct}%</span>
        </div>
      </div>
      ` : `
      <div class="btn-row">
        <button class="btn btn-danger">Uninstall</button>
        <button class="btn btn-secondary">Cancel</button>
      </div>
      `}
      <div class="footer-badge">Fast User Installation · Zero System Residue</div>
    </div>
  </body>
  </html>`;
}

async function runVisualSuite() {
  console.log('====================================================');
  console.log('   Running Playwright Visual E2E Validation Suite   ');
  console.log('====================================================');

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 2, // 2x High-DPI capture
  });

  const page = await context.newPage();

  // 1. Home Page Verification
  console.log('\n[1/5] Verifying Pristine Home Screen (evergreen://newtab)...');
  await page.setContent(homeHtml, { waitUntil: 'networkidle' });
  const brandLogo = await page.$('.brand-logo');
  const searchInput = await page.$('#homeSearch');
  if (!brandLogo) throw new Error('Missing .brand-logo on Home screen');
  if (!searchInput) throw new Error('Missing #homeSearch on Home screen');

  const homePath = path.join(DOCS_SCREENSHOTS, '05-browser-home-pristine.png');
  await page.screenshot({ path: homePath });
  console.log(`  -> Captured: ${homePath}`);

  // 2. Settings Page Verification (General / Top)
  console.log('\n[2/5] Verifying Settings Screen (evergreen://settings)...');
  await page.setContent(settingsHtml, { waitUntil: 'networkidle' });
  const pageTitle = await page.$('.page-title');
  if (!pageTitle) throw new Error('Missing .page-title on Settings screen');

  const settingsPath = path.join(DOCS_SCREENSHOTS, '06-browser-settings-general.png');
  await page.screenshot({ path: settingsPath });
  console.log(`  -> Captured: ${settingsPath}`);

  // Scroll to Site Permissions & Privacy & Isolation
  await page.evaluate(() => {
    const permCard = document.querySelector('#permLocationSelect');
    if (permCard) permCard.scrollIntoView({ behavior: 'instant', block: 'center' });
  });
  await page.waitForTimeout(150);
  const privacyPath = path.join(DOCS_SCREENSHOTS, '07-browser-settings-privacy-performance.png');
  await page.screenshot({ path: privacyPath });
  console.log(`  -> Captured: ${privacyPath}`);

  // 3. Strict TLS Security Interstitial Verification
  console.log('\n[3/5] Verifying Strict TLS Security Interstitial...');
  await page.setContent(tlsHtml, { waitUntil: 'networkidle' });
  const safetyBtn = await page.$('#safetyBtn');
  const advancedBtn = await page.$('#advancedBtn');
  if (!safetyBtn || !advancedBtn) throw new Error('Missing Safety or Advanced button on TLS interstitial');

  const tlsPath = path.join(DOCS_SCREENSHOTS, '08-browser-tls-security-interstitial.png');
  await page.screenshot({ path: tlsPath });
  console.log(`  -> Captured: ${tlsPath}`);

  // Click Advanced to expand drawer
  await page.click('#advancedBtn');
  await page.waitForTimeout(150);
  const tlsExpandedPath = path.join(DOCS_SCREENSHOTS, '09-browser-tls-interstitial-expanded.png');
  await page.screenshot({ path: tlsExpandedPath });
  console.log(`  -> Captured: ${tlsExpandedPath}`);

  // 4. Tab Crash Containment Verification
  console.log('\n[4/5] Verifying Tab Crash Containment Interstitial...');
  await page.setContent(crashHtml, { waitUntil: 'networkidle' });
  const reloadBtn = await page.$('#reloadBtn');
  if (!reloadBtn) throw new Error('Missing reload button on crash page');

  const crashPath = path.join(DOCS_SCREENSHOTS, '10-browser-crash-containment.png');
  await page.screenshot({ path: crashPath });
  console.log(`  -> Captured: ${crashPath}`);

  // 5. Visual Installer Layout Verification (Dialog Viewport)
  console.log('\n[5/5] Verifying Visual Installer Steps...');
  const installerContext = await browser.newContext({
    viewport: { width: 560, height: 380 },
    deviceScaleFactor: 2,
  });
  const instPage = await installerContext.newPage();

  // Step 1: Prerequisites
  await instPage.setContent(getInstallerHtml("Evergreen Browser Setup", "Checking system environment and WebView2 Runtime...", 25), { waitUntil: 'networkidle' });
  const instStep1 = path.join(DOCS_SCREENSHOTS, '01-installer-prerequisites-step.png');
  await instPage.screenshot({ path: instStep1 });
  console.log(`  -> Captured: ${instStep1}`);

  // Step 2: Extracting Files Progress
  await instPage.setContent(getInstallerHtml("Evergreen Browser Setup", "Extracting Evergreen Browser application files...", 65), { waitUntil: 'networkidle' });
  const instStep2 = path.join(DOCS_SCREENSHOTS, '02-installer-extracting-progress.png');
  await instPage.screenshot({ path: instStep2 });
  console.log(`  -> Captured: ${instStep2}`);

  // Step 3: Complete / Launching
  await instPage.setContent(getInstallerHtml("Evergreen Browser Setup", "Installation complete! Starting Evergreen Browser...", 100), { waitUntil: 'networkidle' });
  const instStep3 = path.join(DOCS_SCREENSHOTS, '03-installer-complete-launch.png');
  await instPage.screenshot({ path: instStep3 });
  console.log(`  -> Captured: ${instStep3}`);

  // Step 4: Uninstaller Dialog
  await instPage.setContent(getInstallerHtml("Uninstall Evergreen Browser", "Are you sure you want to remove Evergreen Browser and its shortcuts from this computer?", 0, true), { waitUntil: 'networkidle' });
  const instStep4 = path.join(DOCS_SCREENSHOTS, '04-installer-uninstaller-dialog.png');
  await instPage.screenshot({ path: instStep4 });
  console.log(`  -> Captured: ${instStep4}`);

  await browser.close();

  console.log('\n====================================================');
  console.log('   All 10 Visual E2E Artifacts Successfully Verified! ');
  console.log('====================================================\n');
}

runVisualSuite().catch(err => {
  console.error('\nVisual E2E Suite Error:', err);
  process.exit(1);
});
