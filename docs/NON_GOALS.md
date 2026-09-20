# Evergreen Browser — Non-Goals

To maintain high performance, minimal resource usage, zero telemetry, and long-term stability, the following features are permanently out of scope:

1. **No Bookmarks or History UI**: In-memory session tracking only. The browser forgets when closed unless explicitly allowed.
2. **No Accounts or Sync**: Zero cloud infrastructure, no user accounts, no cross-device sync.
3. **No Extension Store or Plugin Engine**: Extensions are the single largest source of memory bloat, security vulnerabilities, and maintenance churn in web browsers.
4. **No AI Sidebars or Chatbots**: The browser rendering path remains clean and dedicated strictly to browsing web pages.
5. **No Built-in VPN, Crypto Wallets, or Shopping Tools**: No sponsored crapware or revenue-generating affiliate junk.
6. **No Telemetry or Analytics**: Zero tracking, zero usage reporting, zero network calls without direct user action.
