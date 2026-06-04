# Changelog

All notable changes to the Accountability App will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

Planned work for future versions. Items move into a dated release section below once shipped.

### Fixed

- **Timer stuck at zero after clearing data** — after `Clear All Data`, the timer no longer freezes until an app switch. Two separate state machines were out of sync: the Zustand store wasn't refreshing after the DB clear, and the Rust tracking thread was holding an orphaned in-memory session pointing at a deleted row. Fixed by calling `refreshStats`/`refreshSessions` in the store after clear, and adding a `needs_reset` flag to `ActivityTracker` so the thread drops its stale session on the next poll tick and immediately starts a fresh one.

### Planned

- **App blocking UI** — frontend surface for the existing `add_blocked_app` / `remove_blocked_app` / `get_blocked_apps` backend commands.
- **Tab session bridge** — frontend and backend read paths now exist for `tab_sessions`; browser/editor clients are still pending.
- **AI therapist chatbot** — personalised advice based on tracked usage, powered by a local LLM via [Ollama](https://ollama.com/).

### Platform Expansion (Future Versions)

- **iOS** — [Screen Time API](https://developer.apple.com/documentation/screentime) (requires Apple Developer entitlements).
- **Android** — Tauri Mobile supports Android; activity tracking API (`UsageStatsManager`) requires `PACKAGE_USAGE_STATS` permission.
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
- **macOS active window tracking** — merged via PR #1; `active-win-pos-rs` now covers both Windows and macOS.
- **CI pipeline** — GitHub Actions running `npm test` (frontend), `cargo test` on Ubuntu (backend), and `cargo test` on macOS. Missing: Windows runner and a Tauri build check.
- **Encrypted local database** — SQLCipher bundled via `rusqlite`; the database key is stored in the OS keychain.
- **Safer key-loss handling** — when the OS keychain entry is missing, the unreadable database is moved aside instead of being deleted.

### Known Limitations

- App blocking has backend commands but no user-facing UI.
