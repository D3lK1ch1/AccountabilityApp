# Changelog

All notable changes to the Accountability App will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

Planned work for future versions. Items move into a dated release section below once shipped.

### Planned

- **App blocking UI** — frontend surface for the existing `add_blocked_app` / `remove_blocked_app` / `get_blocked_apps` backend commands.
- **AI therapist chatbot** — personalised advice based on tracked usage, powered by a local LLM via [Ollama](https://ollama.com/).
- **Encrypted local database** — SQLite encryption with a user-derived key.
- **CI pipeline** — GitHub Actions running `cargo test`, `cargo clippy`, `npm test`, and a Tauri build check on every push.

### Platform Expansion (Future Versions)

Activity tracking is currently Windows-only via `active-win-pos-rs`. These are the platform-specific APIs to research for each target:

- **macOS** — investigate a Cocoa / AppKit accessibility API approach for active window detection.
- **iOS** — [Screen Time API](https://developer.apple.com/documentation/screentime) (requires Apple Developer entitlements).
- **Browser tab tracking** — browser extensions for deeper insight into browser-based time:
  - Chrome: [Extensions API — tabs](https://developer.chrome.com/docs/extensions/reference/tabs)
  - Firefox: [WebExtensions API](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions)

---

## [0.1.0] — 2026-04-10

Initial pre-beta release. Core activity tracking is functional; blocking and AI features are scaffolded or planned.

### Added

- Tauri 2 + React 19 + TypeScript project scaffold with Vite and Zustand.
- Rust backend with a single SQLite database (`accountability.db`) managing three tables: `app_sessions`, `blocked_apps`, and `settings`.
- Windows active window tracking via `active-win-pos-rs`, polling every 3 seconds.
- Session lifecycle handling — starts a new session on app change, updates duration on the previous one.
- Crash recovery — `end_crash_session` closes any dangling sessions on startup.
- Dashboard stats aggregation: total tracked time, most-used app, per-app usage breakdown, session count.
- Draggable, always-on-top widget window (400x1000 expanded, 400x100 collapsed) with transparent background.
- System tray integration with Show Dashboard / Hide to Tray / Quit menu.
- Global hotkey `Ctrl+Shift+A` to toggle widget visibility.
- First-run consent modal that persists the user's decision in the `settings` table.
- Clear all session data button with confirmation prompt.
- Quit button in the widget header.
- Personalized, privacy-safe error messages (raw database errors never reach the frontend).
- Frontend test setup with Vitest, Testing Library, and jsdom — tests for `useAppStore`, formatters, and the Widget component.
- Backend Rust unit tests in `database.rs` covering session insert/get, session lifecycle updates, blocked-apps CRUD, settings CRUD, per-app usage aggregation, and total tracked time — run with `cargo test`.
- Backend blocking data layer (`add_blocked_app`, `remove_blocked_app`, `get_blocked_apps`) — backend-only, no UI yet.

### Known Limitations

- Activity tracking only works on Windows.
- App blocking has backend commands but no user-facing UI.
- Backend tests cover the database CRUD and aggregation paths, but do not yet cover `end_crash_session` or error cases.
- No CI/CD pipeline.
