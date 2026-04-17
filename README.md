# Accountability App

A privacy-first Windows desktop widget that tracks your app usage and helps you stay aware of your digital habits. All data is stored locally on your machine — no cloud, no telemetry.

> **Status:** Planning pre-beta (v0.1.0). Core tracking works; blocking UI and AI chatbot are on the roadmap.

---

## Description

Accountability App is a lightweight, always-on-top desktop widget that runs quietly in your system tray and records which apps you use and for how long. It surfaces daily usage stats and session history in a draggable widget so you can see where your time is actually going.

Built with a Rust backend for system-level activity tracking and a React frontend for the widget UI, with a local SQLite database as the single source of truth.

### Current Features

- [x] Windows app activity tracking (active window polling every 3s)
- [x] Session-based time tracking with crash recovery
- [x] Daily usage stats and most-used app summary
- [x] Draggable always-on-top widget with expand/collapse views
- [x] System tray integration
- [x] Global hotkey (`Ctrl+Shift+A`) to show/hide widget
- [x] First-run consent modal for privacy confirmation
- [x] Clear all session data at any time
- [ ] App blocking UI *(backend commands ready, UI pending)*
- [ ] AI therapist chatbot *(planned)*

See [CHANGELOG.md](./CHANGELOG.md) for version history and upcoming work.

---

## Built With

| Layer | Technology |
|-------|------------|
| App Framework | [Tauri 2](https://tauri.app/) |
| Frontend | [React 19](https://react.dev/) + TypeScript |
| State Management | [Zustand](https://github.com/pmndrs/zustand) |
| Backend | [Rust](https://www.rust-lang.org/) |
| Database | [SQLite](https://www.sqlite.org/) via [rusqlite](https://github.com/rusqlite/rusqlite) |
| Activity Tracking | [active-win-pos-rs](https://github.com/dimusic/active-win-pos-rs) |
| Build Tool | [Vite 7](https://vitejs.dev/) |
| Testing (Frontend) | [Vitest](https://vitest.dev/) + [Testing Library](https://testing-library.com/) |
| Testing (Backend) | Rust built-in test framework + [tempfile](https://crates.io/crates/tempfile) |

---

## Getting Started

### Prerequisites

You'll need the following installed on a Windows 10 or later machine:

- **Node.js** 18+ and npm — [download](https://nodejs.org/)
- **Rust** toolchain — install via [rustup](https://rustup.rs/)
- **Tauri prerequisites** for Windows (WebView2 is preinstalled on Windows 10+) — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/<your-username>/AccountabilityApp.git
   cd AccountabilityApp
   ```

2. Install frontend dependencies:

   ```bash
   npm install
   ```

3. Run the app in development mode:

   ```bash
   npm run tauri dev
   ```

The first launch compiles the Rust backend, which can take a few minutes. Subsequent launches are much faster.

### Running the Tests

The project has both frontend and backend tests. If you want to verify things are working or explore the test setup:

**Frontend (Vitest):**

```bash
npm test           # Run the frontend test suite once
npm run test:watch # Run in watch mode
```

Frontend tests live in `src/__tests__/` and cover the Zustand store, formatter utilities, and the Widget component.

**Backend (Rust):**

```bash
cd src-tauri
cargo test         # Run the Rust unit test suite
```

Backend unit tests live inline at the bottom of `src-tauri/src/database.rs` (inside a `#[cfg(test)]` module) and cover session CRUD, session lifecycle updates, blocked-apps CRUD, settings CRUD, per-app usage aggregation, and total tracked time.

> **Note:** This project is not currently accepting external contributions. The tests are here so you can confirm your local setup works and explore how the app is structured.

---

## Usage
Video to be determined once satisfied and closer to potential release

---

## Roadmap

High-level direction for the project. See [CHANGELOG.md](./CHANGELOG.md) for version-specific planning.

- [ ] App blocking UI (backend already supports `add_blocked_app` / `remove_blocked_app`)
- [ ] AI therapist chatbot powered by a local LLM via [Ollama](https://ollama.com/)
- [x] Encrypted SQLite database with a user-derived key
- [x] Expand backend test coverage (`end_crash_session`, error cases) 
- [x] Add a CI pipeline
- [x] macOS support *(active window tracking — merged PR #1)*
- [ ] iOS support (via the Screen Time API)
- [ ] Browser extension for tab-level tracking (Chrome / Firefox)

---

## License

License to be decided before beta release.
