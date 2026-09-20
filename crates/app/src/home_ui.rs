//! Default Home Screen (New Tab Page) for Evergreen Browser
//! Styled in Direction 01 — Fluent Desktop (WinUI 3 / Edge Dev minimal).

pub const HOME_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>New Tab - Evergreen</title>
  <style>
    :root {
      --bg-gradient: radial-gradient(circle at 50% 20%, #22222e 0%, #16161d 100%);
      --surface-card: rgba(255, 255, 255, 0.04);
      --surface-card-hover: rgba(255, 255, 255, 0.08);
      --border-subtle: rgba(255, 255, 255, 0.08);
      --border-hover: rgba(78, 140, 255, 0.4);
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
      padding: 60px 24px 30px;
      user-select: none;
    }

    .main-content {
      display: flex;
      flex-direction: column;
      align-items: center;
      max-width: 680px;
      width: 100%;
      margin-top: 40px;
    }

    /* Logo & Branding */
    .brand {
      display: flex;
      flex-direction: column;
      align-items: center;
      margin-bottom: 36px;
    }

    .brand-icon {
      width: 64px;
      height: 64px;
      border-radius: 16px;
      background: linear-gradient(135deg, rgba(52, 211, 153, 0.2) 0%, rgba(78, 140, 255, 0.2) 100%);
      border: 1px solid rgba(255, 255, 255, 0.12);
      display: flex;
      align-items: center;
      justify-content: center;
      margin-bottom: 16px;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    }

    .brand-icon svg {
      width: 36px;
      height: 36px;
      color: var(--accent-green);
    }

    .brand-title {
      font-size: 28px;
      font-weight: 600;
      letter-spacing: -0.5px;
      margin-bottom: 6px;
    }

    .brand-badges {
      display: flex;
      gap: 8px;
      font-size: 11px;
    }

    .badge {
      background: rgba(255, 255, 255, 0.06);
      border: 1px solid var(--border-subtle);
      padding: 2px 8px;
      border-radius: 12px;
      color: var(--text-muted);
    }

    .badge.ephemeral {
      color: var(--accent-green);
      border-color: rgba(52, 211, 153, 0.25);
    }

    /* Search Bar */
    .search-container {
      width: 100%;
      position: relative;
      margin-bottom: 40px;
    }

    .search-box {
      width: 100%;
      height: 48px;
      background: rgba(26, 26, 34, 0.8);
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 24px;
      padding: 0 48px;
      color: var(--text-main);
      font-family: inherit;
      font-size: 15px;
      outline: none;
      backdrop-filter: blur(12px);
      transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
    }

    .search-box:focus {
      border-color: var(--accent);
      background: rgba(30, 30, 42, 0.95);
      box-shadow: 0 6px 28px rgba(78, 140, 255, 0.2);
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
      right: 14px;
      top: 50%;
      transform: translateY(-50%);
      font-size: 11px;
      color: var(--text-muted);
      background: rgba(255, 255, 255, 0.06);
      padding: 3px 8px;
      border-radius: 10px;
    }

    /* Quick Links Grid */
    .tiles-grid {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 14px;
      width: 100%;
      max-width: 600px;
    }

    .tile {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 16px 12px;
      background: var(--surface-card);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
      text-decoration: none;
      color: var(--text-main);
      transition: all 0.15s ease;
      cursor: pointer;
    }

    .tile:hover {
      background: var(--surface-card-hover);
      border-color: var(--border-hover);
      transform: translateY(-2px);
      box-shadow: 0 6px 18px rgba(0, 0, 0, 0.3);
    }

    .tile-icon {
      width: 36px;
      height: 36px;
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.05);
      display: flex;
      align-items: center;
      justify-content: center;
      margin-bottom: 10px;
    }

    .tile-icon svg {
      width: 20px;
      height: 20px;
      stroke: var(--text-main);
      stroke-width: 2;
      stroke-linecap: round;
      stroke-linejoin: round;
      fill: none;
    }

    .tile-title {
      font-size: 12px;
      font-weight: 500;
      color: var(--text-main);
      text-align: center;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      max-width: 100%;
    }

    /* Footer / Specs */
    .footer {
      display: flex;
      align-items: center;
      justify-content: space-between;
      width: 100%;
      max-width: 800px;
      border-top: 1px solid var(--border-subtle);
      padding-top: 18px;
      font-size: 11px;
      color: var(--text-muted);
    }

    .footer a {
      color: var(--accent);
      text-decoration: none;
    }

    .footer a:hover {
      text-decoration: underline;
    }
  </style>
</head>
<body>
  <div class="main-content">
    <div class="brand">
      <div class="brand-icon">
        <!-- Evergreen Pine Tree SVG -->
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 2L4 12h5l-4 8h14l-4-8h5L12 2z"/>
          <path d="M12 20v2"/>
        </svg>
      </div>
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

    <!-- Quick Launch Tiles -->
    <div class="tiles-grid">
      <a class="tile" href="https://github.com">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
          </svg>
        </div>
        <span class="tile-title">GitHub</span>
      </a>

      <a class="tile" href="https://duckduckgo.com">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <circle cx="11" cy="11" r="8"/>
            <path d="m21 21-4.35-4.35"/>
          </svg>
        </div>
        <span class="tile-title">DuckDuckGo</span>
      </a>

      <a class="tile" href="https://www.rust-lang.org">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <polygon points="12 2 2 7 12 12 22 7 12 2"/>
            <polyline points="2 17 12 22 22 17"/>
            <polyline points="2 12 12 17 22 12"/>
          </svg>
        </div>
        <span class="tile-title">Rust Lang</span>
      </a>

      <a class="tile" href="https://en.wikipedia.org">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>
          </svg>
        </div>
        <span class="tile-title">Wikipedia</span>
      </a>

      <a class="tile" href="https://news.ycombinator.com">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <polyline points="4 17 10 11 4 5"/>
            <line x1="12" y1="19" x2="20" y2="19"/>
          </svg>
        </div>
        <span class="tile-title">Hacker News</span>
      </a>

      <a class="tile" href="https://docs.rs">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="16" y1="13" x2="8" y2="13"/>
            <line x1="16" y1="17" x2="8" y2="17"/>
          </svg>
        </div>
        <span class="tile-title">Docs.rs</span>
      </a>

      <a class="tile" href="https://crates.io">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
            <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
            <line x1="12" y1="22.08" x2="12" y2="12"/>
          </svg>
        </div>
        <span class="tile-title">Crates.io</span>
      </a>

      <a class="tile" href="evergreen://settings">
        <div class="tile-icon">
          <svg viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </div>
        <span class="tile-title">Settings</span>
      </a>
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

        // If it starts with a scheme or looks like a hostname
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
