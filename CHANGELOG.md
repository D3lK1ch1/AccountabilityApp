# Changelog

All notable changes to the Accountability App will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.0] — 2026-08-29

### Added

- **macOS and Linux build support** — `tauri.conf.json`'s bundle targets were hardcoded to `["nsis", "msi"]` (Windows-only installer formats), which would produce no output on any other platform; changed to `"all"` so `tauri build` emits whatever installer format fits the host OS. `Cargo.toml`'s `keyring` dependency is now gated per target (`windows-native` / `apple-native` / `sync-secret-service`) instead of a single unconditional dependency, closing the "not yet confirmed fixed" macOS keyring gap noted below in `0.2.0`. CI gained a `tauri-build-macos` job that runs a full build and uploads the resulting `.dmg`, and `backend-macos` now runs the Rust test suite on macOS. Both are green in CI. **Not yet smoke-tested on end-user Mac hardware** — CI verification is on GitHub's hosted macOS runner, not a person opening the built `.dmg` on their own machine; that step is still outstanding before distributing it to a Mac user.

### Fixed

- **`backend-linux` CI job failed on every run — `cargo test` had no Secret Service to talk to.** All 11 tests that construct a `Database` (`test_settings_crud`, both `test_x_keyword_*` tests, etc.) call `get_or_create_key`, which on Linux uses the `sync-secret-service` keyring backend — a D-Bus interface requiring a running keyring daemon (gnome-keyring or similar). A bare `ubuntu-latest` CI runner has no desktop session and therefore no D-Bus session bus or keyring daemon, so every one of those tests panicked on `Database::new(...).unwrap()`. Fixed by installing `dbus-x11`/`gnome-keyring` in the `backend-linux` job and wrapping the test step in a real D-Bus session (`dbus-launch`) with the keyring unlocked (`gnome-keyring-daemon --unlock`) before running `cargo test`. Unlike the `0.2.0` Windows keyring bug below, this was a loud failure — tests went red immediately — not a silent one; no data was ever at risk, only CI coverage was blocked.
- **Report elapsed time could balloon to days if the app went untouched for a while** — `generate_session_report`'s `since` boundary came only from the persisted `last_report_generated_at` checkpoint, with no ceiling. If the app kept running in the background without a report ever being generated, the next report's footer would show real wall-clock elapsed time since that stale checkpoint — technically accurate, but confusing when read next to a Time column that only showed `HH:MM:SS` with no date, making a multi-day gap look like the same afternoon. Fixed two ways: `since` is now floored at this run's launch time (`AppState::launched_at`, stamped once at startup), so a report can never claim to cover time from before you opened the app this run; and each table row now shows its date, but only on the first row or when the date changes from the row before it, so a genuine multi-day gap is visible without cluttering every row with a redundant date.
- **Closing the window silently kept the app running in the background** — the `CloseRequested` handler called `prevent_close()` and hid the window instead of exiting, so Alt+F4 or the taskbar's "Close window" looked like a quit but left the tracker running invisibly in the tray. Removed hide-to-tray entirely: the `hide_to_tray` command, the tray's "Hide to Tray" menu item, and the `CloseRequested` auto-hide are gone. Any close action — the widget's ✖ button, the tray's Quit item, or the native window close — now stops the tracker and exits the process the same way. The tray's remaining menu is Show / Quit only, and clicking the tray icon always shows and focuses the window rather than toggling visibility.

---

## [0.2.0] — 2026-08-18

### Added

- **Category-based quota blocking** — configurable categories (Social Media, Games, ...) with daily limits, manual pause, and app/window-title keyword matching; usage aggregated across `app_sessions` and the live session. Widget controls for viewing usage, editing limits, and enabling/pausing categories.
- **Downloadable session reports** — a download-icon button in the widget (`generate_session_report` command) produces a chronological Markdown report of tracked app activity since the last report was generated, or since the start of the current day if none has been generated yet. Each row lists start time, duration, app name, and window title; a footer totals entries and compares tracked switch-time against wall-clock elapsed time. Saved through a native Save-As dialog (new `tauri-plugin-dialog` dependency, paired with a `save_text_file` command that writes the bytes) rather than a browser-style download, so the app can confirm the write actually happened. Filenames auto-number (`session_01_YYYY-MM-DD_HHMM.md`, ...) via a `next_session_report_number` counter in the `settings` table; the counter and the `last_report_generated_at` checkpoint both advance only on a confirmed save, never on a cancelled dialog.
- **Live category indicator in the collapsed widget** — the collapsed header now shows the name, used/limit time, and a progress bar for whichever enabled, unpaused category the current app or window belongs to, refreshing on the same 3-second poll as the rest of the widget.

### Fixed

- **Encryption key was never actually persisted on Windows — every restart silently reset all tracked history.** The `keyring` dependency was declared with no Cargo features, and the `keyring` crate has no default features; on Windows this made it silently compile in its in-memory-only `mock` backend instead of the real Windows Credential Manager backend, with no error at compile or run time. Every launch, the app correctly detected "no key found," moved the (undecryptable) database aside, and generated a new key — but that new key only ever lived in that process's RAM, so the exact same thing happened again on the next launch. This affected every Windows install of this app, including the published `v0.1.0` release; tracked history did not persist across restarts. Fixed by enabling the `windows-native` Cargo feature. Also fixed a related issue this surfaced: the test suite used the same hardcoded keychain identifier as the real app, so running `cargo test` locally would have overwritten a real user's production encryption key — tests now use an isolated `accountability_app_test` identifier. (macOS's equivalent `apple-native` feature has the same default-off gap and is not yet confirmed fixed; tracked as a follow-up.)
- **Category keyword matching produced false positives on substring overlap** — `contains_keyword` matched a category's keywords anywhere inside a window title or app name, including inside unrelated words. The Social Media category's `"x"` keyword (added for the X/Twitter taskbar app) matched the letter *x* inside "Windows Explorer," silently counting File Explorer time toward the Social Media quota — confirmed against an actual generated report, not a hypothetical. Fixed by requiring a match not be flanked by another alphanumeric character, so `"x"` still matches a window titled "X" but no longer matches inside "Explorer." New tests cover both the false positive and the still-intended match.
- **Report elapsed time inflated by checkpoint fallback** — `format_report` used the query's window boundary (e.g. start-of-today, when no report had been generated yet) as the displayed window start, so a report covering 20 minutes of actual activity could claim "17h elapsed" if generated late in the day. Fixed by deriving the displayed window start from the first entry's actual timestamp instead of the query boundary.
- **Redundant default block keywords** — the default Social Media category listed both `twitter.com`/`twitter` and `x.com` as separate keywords for the same site post-rebrand. Removed the stale `twitter` entries and added `x` as an app keyword so window-title matching (e.g. an "X" taskbar app) still works.
- **"Today" boundary used UTC midnight instead of local midnight** — `get_sessions_today` (dashboard stats, and report generation's default lookback) computed the start of "today" from `Utc::now()`, so for users outside UTC, "today's" stats could include hours from the previous local day or exclude the first hours of the current one. Fixed with a `today_start_timestamp()` helper that computes local midnight instead.
- **Keyword editor resetting cursor on every keystroke** — the domain and app keyword inputs were directly bound to the Zustand store value and called `saveBlockCategory` (a Tauri invoke) on every `onChange` event. The Tauri round-trip refreshed the store before the user finished typing, re-rendering the input and resetting the cursor mid-word. Fixed by introducing a local draft state in Widget.tsx — keystrokes update only the draft, and the save fires once on `onBlur` when the user clicks away.
- **Timer stuck at zero after clearing data** — after `Clear All Data`, the timer no longer freezes until an app switch. Two separate state machines were out of sync: the Zustand store wasn't refreshing after the DB clear, and the Rust tracking thread was holding an orphaned in-memory session pointing at a deleted row. Fixed by calling `refreshStats`/`refreshSessions` in the store after clear, and adding a `needs_reset` flag to `ActivityTracker` so the thread drops its stale session on the next poll tick and immediately starts a fresh one.

### Removed

- **Tab-session bridge (WebSocket server + Chrome extension)** — a local WebSocket server (`127.0.0.1:7734`), a `tab_sessions` table, and an unpacked Chrome extension (`extensions/chrome`) reporting per-tab activity and rendering block overlays with rotating deterrent popups were built and fully tested, including a fix for a real WebSocket origin-spoofing vulnerability (any webpage's JavaScript could otherwise connect and inject forged tab events, since `WebSocket` isn't subject to CORS the way `fetch()` is — fixed by pinning the extension's ID via `manifest.json`'s `"key"` field and rejecting handshakes from any other origin). Removed before release, not abandoned mid-build: distributing the extension to real users would require either publishing through the Chrome Web Store or asking every user to manually enable Developer Mode and "Load Unpacked" — friction disproportionate to the gain, since app-level tracking (`app_sessions`) already captures most of the same signal for free, because Chrome and VS Code both put the active tab/file name directly in their OS window title. A VS Code/editor client was never built. 

### Known Limitations

- Category quota enforcement is app/window-title based only — there is no in-browser blocking overlay or deterrent popup. A category being over its daily limit is visible in the widget but does not interrupt browsing.

### Planned

- **App blocking UI** — frontend surface for the existing `add_blocked_app` / `remove_blocked_app` / `get_blocked_apps` backend commands.

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
