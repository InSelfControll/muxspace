# Changelog

All notable changes to Muxspace are documented in this file.

## [Unreleased]

## [0.2.0] — 2026-03-15

### Added

- **Browser Profile Integration** — Import cookies and detect extensions from Chrome/Chromium browser profiles using SQLite and PBKDF2+AES-CBC decryption
- **Multi-Tab Browser Panes** — Embed real webkit2gtk WebViews inside the app via GtkOverlay with per-pane URL bar, user agent persistence, and tab management
- **Custom Header with Menu Bar** — Application header with Window, Edit, and Help dropdown menus replacing the native menu bar
  - **Window menu**: New Terminal Pane, New Workspace, Close Pane, Close Workspace, Next/Previous Workspace, Next/Previous Project
  - **Edit menu**: Configure Hotkeys
  - **Help menu**: Keyboard Shortcuts
- **Configurable Hotkeys** — JSON-based hotkey configuration at `~/.config/muxspace/hotkeys.json` with prefix key support (tmux-style `Ctrl+B` prefix)
- **Hotkey Editor Modal** — GUI for viewing and configuring keyboard shortcuts, grouped by category (Pane Navigation, Panes, Workspaces, Quick Switch, Projects) with consistent dark theme styling
- **Shortcuts Help Modal** — Quick-reference overlay for all keyboard shortcuts
- **Workspace Management** — Add, switch, rename, and close workspaces with tab UI including × close buttons
- **Workspace Deletion Persistence** — `save_projects_blocking()` now scans and removes stale `project:*` keys from sled DB so deleted projects/workspaces stay deleted on restart
- **Project Navigation** — Sidebar with project list, create/switch/delete projects, next/previous project cycling via hotkeys
- **Directory Picker for Project Creation** — Native OS folder picker via `rfd::AsyncFileDialog` in the Create Project modal alongside manual text input
- **PTY Terminal Panes** — Embedded terminal emulator using `portable-pty` and `vte` parser with 33ms polling
- **Prefix Mode Indicator** — Fixed overlay at bottom-right showing available prefix-mode shortcuts when prefix key is active
- **Welcome View** — Landing page with keyboard shortcut reference when no project is selected
- **Native File Dialogs** — Added `rfd = "0.15"` for cross-platform file/folder picker support

### Changed

- **Removed native window decorations** — Set `with_decorations(false)` on WindowBuilder, `with_menu(None)` on Config, and added GTK/GDK-level decoration removal via `glib::idle_add_local_once` for window managers with server-side decorations
- **Removed duplicate window controls** — Removed custom minimize/maximize/close buttons from the Header since the native title bar persists on some WMs (TUXEDO OS/KDE/GNOME SSD), avoiding double controls
- **Removed duplicate header** — Removed "Muxspace" h1 from the Header component since the sidebar already contains branding
- **Modal z-order fix** — WebViews are hidden when modals are open (via a polling coroutine) so HTML-based modals aren't obscured by the GTK overlay
- **Hotkey editor redesign** — Rewrote with category grouping, consistent dark theme colors (`#0f0f1a` background, `#1a1a2e` chips, `#6366f1` accent), fixed 200px label column, section headers, and config file path reference in footer

### Fixed

- **glib v2_68 feature flag** — Enabled `v2_68` feature on `glib` crate to fix `G_SIGNAL_ACCUMULATOR_FIRST_RUN` compile error on systems with GLib ≥ 2.68
- **glib/gtk Cast import mismatch** — Changed `glib::Cast` to `gtk::prelude::Cast` to resolve trait bound conflicts between `glib 0.20` and `gtk 0.18` (which internally depends on `glib 0.18`)
- **Workspace deletion not persisting** — `save_projects_blocking()` only inserted projects but never removed stale keys from sled DB; added cleanup logic
- **Browser URL bar not updating** — Fixed URL persistence when switching between browser tabs/workspaces
- **Unused code warnings** — Removed unused `get_tab_count`, added `#[allow(dead_code)]` on `ExtensionInfo`

### Dependencies

- `dioxus` 0.5 with desktop feature
- `dioxus-desktop` 0.5 with transparent feature
- `glib` 0.20 with `v2_68` feature
- `gtk` 0.18, `gdk` 0.18, `webkit2gtk` 2.0
- `rfd` 0.15 — native file dialogs
- `rusqlite` 0.31 (bundled) — browser cookie/extension SQLite reading
- `aes` 0.8, `cbc` 0.1, `pbkdf2` 0.12, `sha1` 0.10 — Chrome cookie decryption
- `portable-pty` 0.8, `vte` 0.13 — terminal emulation
- `sled` 0.34 — embedded database for project persistence
- `tokio` 1, `serde` 1, `chrono` 0.4, `anyhow` 1

## [0.1.0] — 2026-03-15

- Initial commit: Muxspace workspace manager with Dioxus desktop
- Simplified repo structure to Dioxus-only app
