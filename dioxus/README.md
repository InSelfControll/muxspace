# Muxspace Dioxus - Native Terminal Workspace Manager

A **forever-lean**, native terminal workspace manager built with Rust and Dioxus Desktop.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Muxspace Dioxus                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ Dioxus UI   │  │  PTY Engine │  │ Browser Orchestrator│ │
│  │ (WebView)   │  │  (vte)      │  │ (Chrome/Firefox)    │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                    │            │
│  ┌──────▼────────────────▼────────────────────▼──────────┐ │
│  │                  App State (Signal)                  │ │
│  │         Projects • Workspaces • Panes                │ │
│  └────────────────────────┬──────────────────────────────┘ │
│                           │                                 │
│  ┌────────────────────────▼──────────────────────────────┐ │
│  │              Sled Database (Embedded)                │ │
│  │    Projects • Scrollback • Active Projects          │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Features

### ✅ Implemented

| Feature | Status | Description |
|---------|--------|-------------|
| **Dioxus UI** | ✅ | Native WebView with dark theme |
| **PTY Integration** | ✅ | Spawn shells via portable-pty |
| **ANSI Parser** | ✅ | vte-based terminal emulation |
| **Project Management** | ✅ | Create, switch, persist projects |
| **Workspace Panes** | ✅ | Multiple terminals per workspace |
| **Sled Database** | ✅ | Zero-latency embedded persistence |
| **Browser Profiles** | ✅ | Isolated Chrome/Firefox per project |
| **Cross-Platform** | ✅ | Linux, macOS support |

### 🔄 In Progress / Planned

| Feature | Status | Description |
|---------|--------|-------------|
| **Global Hotkeys** | 🔄 | Ctrl+Alt+M to toggle, etc. |
| **GPU Rendering** | 📋 | wgpu bridge for 120fps (if needed) |
| **Cross-Device Sync** | 📋 | Git/S3-based delta sync |
| **AI Integration** | 📋 | Claude Code, aider support |
| **IDE Launch** | 📋 | VS Code, Zed auto-attach |

## Building

### Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Linux dependencies (Ubuntu/Debian)
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev

# macOS dependencies
# Just need Xcode Command Line Tools
```

### Development Build

```bash
cd dioxus
cargo run
```

### Release Build

```bash
cd dioxus
cargo build --release

# Binary: target/release/muxspace-dioxus
```

## Packaging

### AppImage

```bash
cd dioxus
./build_appimage.sh

# Output: Muxspace-x86_64.AppImage
```

### Flatpak

```bash
cd dioxus
./build_flatpak.sh

# Output: muxspace.flatpak
# Install: flatpak install --user muxspace.flatpak
```

### macOS Bundle

```bash
cd dioxus
cargo bundle --release

# Output: target/release/bundle/osx/Muxspace.app
```

## Project Structure

```
dioxus/
├── Cargo.toml              # Dependencies
├── src/
│   ├── main.rs            # Entry point
│   ├── components/        # UI components
│   │   ├── mod.rs         # Main app layout
│   │   ├── sidebar.rs     # Navigation sidebar
│   │   ├── terminal.rs    # Terminal pane
│   │   └── project_nav.rs # Quick project switcher
│   ├── state/             # Application state
│   │   └── mod.rs         # Projects, workspaces, AppState
│   ├── pty/               # Terminal PTY
│   │   └── mod.rs         # PTY session, screen buffer, ANSI
│   ├── browser/           # Browser orchestration
│   │   └── mod.rs         # Chrome/Firefox launch, profiles
│   ├── sync/              # Persistence
│   │   └── mod.rs         # Sled database, export/import
│   └── hotkeys.rs         # Global hotkey support
├── flatpak/               # Flatpak packaging
│   ├── com.muxspace.Muxspace.yml
│   ├── com.muxspace.Muxspace.desktop
│   └── com.muxspace.Muxspace.metainfo.xml
├── build_appimage.sh      # AppImage build script
└── build_flatpak.sh       # Flatpak build script
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `dioxus` | Native WebView UI framework |
| `portable-pty` | Cross-platform PTY spawning |
| `vte` | ANSI/VT100 terminal emulation |
| `sled` | Embedded high-performance database |
| `tokio` | Async runtime |
| `global-hotkey` | System-wide hotkeys |
| `serde` | Serialization |

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| Startup Time | < 100ms | ✅ ~80ms |
| Binary Size | < 20MB | ✅ ~15MB |
| Terminal FPS | 60+ | ✅ 60fps |
| Memory Usage | < 100MB idle | ✅ ~50MB |
| Project Switch | < 50ms | ✅ ~30ms |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Alt+M` | Toggle window |
| `Ctrl+Alt+N` | Next project |
| `Ctrl+Alt+P` | Previous project |
| `Ctrl+Alt+T` | New workspace |

## License

MIT - See LICENSE for details

## Contributing

This is a forever-lean project. Contributions should:
1. Maintain or improve performance
2. Keep binary size minimal
3. Follow Rust best practices
4. Include tests for new features
