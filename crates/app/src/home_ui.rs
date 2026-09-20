//! Default Home Screen (New Tab Page) for Evergreen Browser
//! Styled in Direction 01 — Fluent Desktop (WinUI 3 / Edge Dev minimal).
//! Clean start page with search input and zero clutter (no quick link tiles).
//! Uses official project logo with transparent background.

pub const LOGO_BASE64: &str = include_str!("../ui/logo.b64");

pub const HOME_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>New Tab - Evergreen</title>
  <link rel="icon" type="image/png" href="data:image/png;base64,{{LOGO_BASE64}}">
  <style>
    :root {
      --bg-gradient: radial-gradient(circle at 50% 25%, #22222e 0%, #16161d 100%);
      --surface-card: rgba(255, 255, 255, 0.04);
      --border-subtle: rgba(255, 255, 255, 0.08);
      --text-main: #f0f0f5;
      --text-muted: #9595a8;
      --accent: #4e8cff;
      --accent-green: #34d399;
      --font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, -apple-system, sans-serif;
    }

    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }

    body {
      background: var(--bg-gradient);
      color: var(--text-main);
      font-family: var(--font-family);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: space-between;
      padding: 80px 24px 36px;
      user-select: none;
    }

    .main-content {
      display: flex;
      flex-direction: column;
      align-items: center;
      max-width: 640px;
      width: 100%;
      margin-top: 50px;
    }

    /* Logo & Branding */
    .brand {
      display: flex;
      flex-direction: column;
      align-items: center;
      margin-bottom: 36px;
    }

    .brand-logo {
      width: 88px;
      height: 80px;
      object-fit: contain;
      background: transparent;
      margin-bottom: 16px;
      filter: drop-shadow(0 8px 24px rgba(52, 211, 153, 0.25));
    }

    .brand-title {
      font-size: 32px;
      font-weight: 600;
      letter-spacing: -0.5px;
      margin-bottom: 10px;
    }

    .brand-badges {
      display: flex;
      gap: 8px;
      font-size: 11px;
    }

    .badge {
      background: rgba(255, 255, 255, 0.06);
      border: 1px solid var(--border-subtle);
      padding: 3px 10px;
      border-radius: 12px;
      color: var(--text-muted);
    }

    .badge.ephemeral {
      color: var(--accent-green);
      border-color: rgba(52, 211, 153, 0.3);
      background: rgba(52, 211, 153, 0.08);
      font-weight: 500;
    }

    /* Search Bar */
    .search-container {
      width: 100%;
      position: relative;
    }

    .search-box {
      width: 100%;
      height: 52px;
      background: rgba(26, 26, 34, 0.85);
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 26px;
      padding: 0 48px;
      color: var(--text-main);
      font-family: inherit;
      font-size: 15px;
      outline: none;
      backdrop-filter: blur(12px);
      transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    }

    .search-box:focus {
      border-color: var(--accent);
      background: rgba(30, 30, 42, 0.95);
      box-shadow: 0 6px 28px rgba(78, 140, 255, 0.22);
    }

    .search-icon {
      position: absolute;
      left: 18px;
      top: 50%;
      transform: translateY(-50%);
      width: 18px;
      height: 18px;
      color: var(--text-muted);
      pointer-events: none;
    }

    .search-engine-badge {
      position: absolute;
      right: 16px;
      top: 50%;
      transform: translateY(-50%);
      font-size: 11px;
      color: var(--text-muted);
      background: rgba(255, 255, 255, 0.06);
      padding: 3px 10px;
      border-radius: 10px;
    }

    /* Footer / Specs */
    .footer {
      display: flex;
      align-items: center;
      justify-content: space-between;
      width: 100%;
      max-width: 800px;
      border-top: 1px solid var(--border-subtle);
      padding-top: 20px;
      font-size: 11px;
      color: var(--text-muted);
    }
  </style>
</head>
<body>
  <div class="main-content">
    <div class="brand">
      <img src="data:image/png;base64,{{LOGO_BASE64}}" class="brand-logo" alt="Evergreen" />
      <h1 class="brand-title">Evergreen</h1>
      <div class="brand-badges">
        <span class="badge ephemeral">Ephemeral Mode Active</span>
        <span class="badge">Native Rust Shell</span>
        <span class="badge">WebView2 Evergreen</span>
      </div>
    </div>

    <!-- Search / Address input -->
    <div class="search-container">
      <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8"/>
        <path d="m21 21-4.35-4.35"/>
      </svg>
      <input
        type="text"
        class="search-box"
        id="homeSearch"
        placeholder="Search DuckDuckGo or enter web address..."
        autocomplete="off"
        autofocus
      />
      <span class="search-engine-badge">DuckDuckGo</span>
    </div>
  </div>

  <footer class="footer">
    <span>Evergreen Browser · Ephemeral Zero-Footprint Navigation</span>
    <span>Powered by Microsoft WebView2 & Rust</span>
  </footer>

  <script>
    const searchBox = document.getElementById('homeSearch');
    searchBox.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const query = searchBox.value.trim();
        if (!query) return;

        if (query.startsWith('http://') || query.startsWith('https://') || query.startsWith('evergreen://')) {
          window.location.href = query;
        } else if (query.includes('.') && !query.includes(' ')) {
          window.location.href = 'https://' + query;
        } else {
          window.location.href = 'https://duckduckgo.com/?q=' + encodeURIComponent(query);
        }
      }
    });
  </script>
</body>
</html>
"#;

pub fn get_home_html() -> String {
    HOME_TEMPLATE.replace("{{LOGO_BASE64}}", LOGO_BASE64.trim())
}

pub static HOME_HTML: std::sync::LazyLock<String> = std::sync::LazyLock::new(get_home_html);
