# Muxspace

A unified terminal workspace manager with TUI and GUI support. Manage multiple projects, terminals, browsers, and AI assistants from a single interface.

## Features

### Core Features
- **Multi-Pane Terminal**: Split terminal with multiple PTY sessions
- **Project Navigator**: Switch between projects with `Ctrl+B P`
- **Workspace Management**: YAML-based workspace configuration
- **Browser Integration**: Launch Chrome/Firefox with preserved profiles
- **AI Assistant Support**: Built-in support for Claude Code, aider, etc.
- **Cross-Device Sync**: Export/import workspace configs
- **Search in Scrollback**: `Ctrl+B /` to search terminal history

### GUI Features (Tauri)
- **Modern Web UI**: React-like interface for managing workspaces
- **Project Dashboard**: Visual overview of all projects
- **Tool Detection**: Auto-detect installed development tools
- **One-Click Launch**: Start workspaces from the GUI

## Installation

### Prerequisites
- Rust 1.70+
- For GUI: webkit2gtk-4.1 (Linux), WebView2 (Windows), or WebKit (macOS)

### Build Terminal Version Only
```bash
cargo build --release -p muxspace
```

### Build with GUI
```bash
# Install Tauri dependencies (Ubuntu/Debian)
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev

# Build both
cargo build --release
```

### Pre-built Binaries
```
target/release/muxspace      # Terminal version (5.1MB)
target/release/muxspace-gui  # GUI version (9.4MB)
```

## Usage

### Terminal UI

```bash
# Start the demo TUI
muxspace tui

# Create a workspace config at ~/.config/muxspace/workspaces/myapp.yaml
muxspace start myapp

# List saved workspaces
muxspace list

# Restore previous session
muxspace restore

# Switch to a project
muxspace project myproject
```

### Keybindings (in TUI)

| Key | Action |
|-----|--------|
| `Ctrl+B n` | Next pane |
| `Ctrl+B p` | Previous pane |
| `Ctrl+B w` | Next workspace |
| `Ctrl+B P` | Project navigator |
| `Ctrl+B /` | Search in scrollback |
| `Ctrl+B [` | Scroll up |
| `Ctrl+B ]` | Scroll down |
| `Ctrl+B q` | Quit |

### Workspace Configuration

Create `~/.config/muxspace/workspaces/my-project.yaml`:

```yaml
name: my-project
project: acme-app

panes:
  - cwd: ~/projects/acme-app
    command: cargo run
  - cwd: ~/projects/acme-app
    command: cargo test --watch

tools:
  - app: claude
    kind: ai_assistant
    path: ~/projects/acme-app
  - app: nvim
    kind: cli_editor
    path: ~/projects/acme-app
  - app: code
    kind: gui_ide
    path: ~/projects/acme-app

browser:
  kind: chrome
  urls:
    - http://localhost:3000
    - http://localhost:8080/api/health
```

### GUI

```bash
# Launch the GUI
muxspace-gui
```

### Multi-Project Restoration

When you run `muxspace restore`, it will:
1. Restore **ALL** projects you were recently working on (not just the last one)
2. Launch external tools (browsers, IDEs) for **all workspaces** in those projects
3. Start the TUI focused on the most recently used project

Use `Ctrl+B P` to switch between active projects.

## Architecture

```
muxspace/
├── crates/muxspace/       # Core library + CLI
│   ├── src/
│   │   ├── ansi/         # ANSI parsing with vte
│   │   ├── pty/          # PTY management
│   │   ├── state/        # SQLite persistence
│   │   ├── tui/          # ratatui interface
│   │   ├── detect.rs     # Tool detection
│   │   ├── orchestrator.rs # External app launcher
│   │   └── types.rs      # Core data structures
│   └── src/main.rs       # CLI entry point
│
└── gui/                  # Tauri GUI
    ├── src-tauri/        # Rust backend
    └── public/           # Web frontend
```

## Implementation Roadmap

### Phase 1: ✅ Architectural Unification
- Single unified crate replacing client/daemon split
- Central `AppState` for all runtime state
- Direct function calls (no IPC)

### Phase 2: ✅ Enhanced Tool Detection
- Auto-detect terminals, editors, IDEs, AI assistants, browsers
- Tool registry with categories

### Phase 3: ✅ Browser Profile Management
- Chrome/Chromium profile support
- Firefox profile support
- Session preservation

### Phase 4: ✅ Project Navigator
- Full-screen project switcher
- Project-based workspace grouping
- Quick context switching

### Phase 5: ✅ Persistence & Sync
- SQLite for workspace state
- Cross-device config sync
- Boot sequence restoration (ALL active projects)

### Phase 6: ✅ PTY Management
- portable-pty for cross-platform support
- Async I/O with tokio
- Non-blocking PTY reads

### Phase 7: ✅ ANSI Parsing
- vte crate for escape sequence parsing
- 10,000 line scrollback buffer
- Color and style support

### Phase 8: ✅ Scrollback & Search
- Search in terminal scrollback
- Match highlighting
- Auto-scroll to matches

### Phase 9: ✅ External App Orchestration
- Launch GUI IDEs and browsers
- PID tracking
- Graceful shutdown

### Phase 10: ✅ Restoration & Polish
- Auto-restore ALL recent projects on startup
- Launch external tools for all active projects
- Last active project tracking for TUI focus
- Workspace config import/export

### Phase 11: ✅ GUI (Tauri v2)
- Modern web-based interface (dark theme)
- Dashboard with stats and active projects
- Visual workspace management
- Tool detection display
- One-click workspace launch
- Cross-platform (Linux, macOS, Windows)

## License

MIT
