use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use vte::{Params, Parser, Perform};

/// Whether tmux is available on this system (checked once at startup).
static TMUX_AVAILABLE: LazyLock<bool> = LazyLock::new(|| which::which("tmux").is_ok());

/// Path to the Muxspace tmux config file (created on first use).
static TMUX_CONFIG: LazyLock<PathBuf> = LazyLock::new(|| {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("muxspace");
    let _ = std::fs::create_dir_all(&config_dir);
    let config_path = config_dir.join("tmux.conf");
    if !config_path.exists() {
        let config = "\
# Muxspace tmux config \u{2014} transparent session wrapper\n\
set -g status off\n\
set -g escape-time 0\n\
set -g prefix C-]\n\
unbind C-b\n\
set -g mouse on\n\
set -g history-limit 50000\n";
        let _ = std::fs::write(&config_path, config);
    }
    config_path
});

// ── Global PTY manager ──────────────────────────────────────────────────────

pub static PTY_MANAGER: LazyLock<Mutex<PtyManager>> =
    LazyLock::new(|| Mutex::new(PtyManager::new()));

pub struct PtyManager {
    sessions: HashMap<String, PtySession>,
    receivers: HashMap<String, std::sync::mpsc::Receiver<Vec<u8>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            receivers: HashMap::new(),
        }
    }

    pub fn spawn_for_pane(&mut self, pane_id: &str, cwd: &Path, cmd: Option<&str>) -> Result<()> {
        if self.sessions.contains_key(pane_id) {
            return Ok(()); // already running
        }
        let (session, rx) = PtySession::spawn(pane_id, cwd, cmd, 24, 80)?;
        self.sessions.insert(pane_id.to_string(), session);
        self.receivers.insert(pane_id.to_string(), rx);
        tracing::info!("Spawned PTY for pane {}", pane_id);
        Ok(())
    }

    pub fn write_to_pane(&self, pane_id: &str, data: &[u8]) -> Result<()> {
        if let Some(session) = self.sessions.get(pane_id) {
            session.write(data)?;
        }
        Ok(())
    }

    /// Drain all pending output for a pane. Returns empty vec if no data.
    pub fn drain_output(&self, pane_id: &str) -> Vec<u8> {
        let mut data = Vec::new();
        if let Some(rx) = self.receivers.get(pane_id) {
            while let Ok(chunk) = rx.try_recv() {
                data.extend(chunk);
            }
        }
        data
    }

    pub fn active_pane_ids(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    pub fn remove_pane(&mut self, pane_id: &str) {
        self.sessions.remove(pane_id);
        self.receivers.remove(pane_id);
        // Kill the tmux session so it doesn't linger after the pane is closed
        kill_tmux_session(pane_id);
    }
}

// ── Terminal cell ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct Cell {
    pub ch: char,
    pub fg: u8,
    #[allow(dead_code)]
    pub bg: u8,
    pub bold: bool,
}

// ── Screen buffer ───────────────────────────────────────────────────────────

pub struct ScreenBuffer {
    pub rows: usize,
    pub cols: usize,
    pub grid: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scrollback: Vec<Vec<Cell>>,
    /// Terminal title set via OSC 0/2 escape sequences (e.g. by the shell or running program).
    pub title: String,
    max_scrollback: usize,
    style: CellStyle,
    parser: Parser,
}

#[derive(Clone, Default, Debug)]
struct CellStyle {
    fg: u8,
    bg: u8,
    bold: bool,
}

impl Clone for ScreenBuffer {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows,
            cols: self.cols,
            grid: self.grid.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            scrollback: self.scrollback.clone(),
            title: self.title.clone(),
            max_scrollback: self.max_scrollback,
            style: self.style.clone(),
            parser: Parser::new(), // fresh parser for the clone
        }
    }
}

impl PartialEq for ScreenBuffer {
    fn eq(&self, _other: &Self) -> bool {
        false // always re-render when screen buffer is compared
    }
}

impl std::fmt::Debug for ScreenBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenBuffer")
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .finish()
    }
}

impl ScreenBuffer {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: vec![vec![Cell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            scrollback: Vec::new(),
            title: String::new(),
            max_scrollback: 10_000,
            style: CellStyle::default(),
            parser: Parser::new(),
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        // Take parser out to avoid aliasing (parser calls &mut self as Perform)
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        for &byte in bytes {
            parser.advance(self, byte);
        }
        self.parser = parser;
    }

    fn scroll_up(&mut self) {
        let old = self.grid.remove(0);
        self.scrollback.push(old);
        if self.scrollback.len() > self.max_scrollback {
            self.scrollback.remove(0);
        }
        self.grid.push(vec![Cell::default(); self.cols]);
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
        }
        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows - 1;
        }
        self.grid[self.cursor_row][self.cursor_col] = Cell {
            ch,
            fg: self.style.fg,
            bg: self.style.bg,
            bold: self.style.bold,
        };
        self.cursor_col += 1;
    }
}

impl Perform for ScreenBuffer {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0B | 0x0C => {
                self.cursor_row += 1;
                if self.cursor_row >= self.rows {
                    self.scroll_up();
                    self.cursor_row = self.rows - 1;
                }
            }
            b'\r' => self.cursor_col = 0,
            b'\t' => {
                // Advance to next tab stop (every 8 columns)
                self.cursor_col = ((self.cursor_col / 8) + 1) * 8;
                if self.cursor_col >= self.cols {
                    self.cursor_col = self.cols.saturating_sub(1);
                }
            }
            0x08 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            0x07 => {} // BEL — bell, ignore
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        fn p(params: &Params, idx: usize, def: usize) -> usize {
            params.iter().nth(idx).and_then(|s| s.first().copied()).unwrap_or(def as u16) as usize
        }

        match action {
            // CUU — Cursor Up
            'A' => self.cursor_row = self.cursor_row.saturating_sub(p(params, 0, 1)),
            // CUD — Cursor Down
            'B' => self.cursor_row = (self.cursor_row + p(params, 0, 1)).min(self.rows.saturating_sub(1)),
            // CUF — Cursor Forward
            'C' => self.cursor_col = (self.cursor_col + p(params, 0, 1)).min(self.cols.saturating_sub(1)),
            // CUB — Cursor Back
            'D' => self.cursor_col = self.cursor_col.saturating_sub(p(params, 0, 1)),
            // CNL — Cursor Next Line
            'E' => {
                self.cursor_row = (self.cursor_row + p(params, 0, 1)).min(self.rows.saturating_sub(1));
                self.cursor_col = 0;
            }
            // CPL — Cursor Previous Line
            'F' => {
                self.cursor_row = self.cursor_row.saturating_sub(p(params, 0, 1));
                self.cursor_col = 0;
            }
            // CHA — Cursor Horizontal Absolute
            'G' => {
                self.cursor_col = p(params, 0, 1).saturating_sub(1).min(self.cols.saturating_sub(1));
            }
            // CUP — Cursor Position
            'H' | 'f' => {
                self.cursor_row = p(params, 0, 1).saturating_sub(1).min(self.rows.saturating_sub(1));
                self.cursor_col = p(params, 1, 1).saturating_sub(1).min(self.cols.saturating_sub(1));
            }
            // ED — Erase in Display
            'J' => match p(params, 0, 0) {
                0 => {
                    for c in self.cursor_col..self.cols { self.grid[self.cursor_row][c] = Cell::default(); }
                    for r in (self.cursor_row + 1)..self.rows { self.grid[r] = vec![Cell::default(); self.cols]; }
                }
                1 => {
                    for r in 0..self.cursor_row { self.grid[r] = vec![Cell::default(); self.cols]; }
                    for c in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) { self.grid[self.cursor_row][c] = Cell::default(); }
                }
                2 | 3 => { for r in 0..self.rows { self.grid[r] = vec![Cell::default(); self.cols]; } }
                _ => {}
            },
            // EL — Erase in Line
            'K' => match p(params, 0, 0) {
                0 => { for c in self.cursor_col..self.cols { self.grid[self.cursor_row][c] = Cell::default(); } }
                1 => { for c in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) { self.grid[self.cursor_row][c] = Cell::default(); } }
                2 => { self.grid[self.cursor_row] = vec![Cell::default(); self.cols]; }
                _ => {}
            },
            // IL — Insert Lines
            'L' => {
                let n = p(params, 0, 1);
                for _ in 0..n {
                    if self.cursor_row < self.rows {
                        self.grid.insert(self.cursor_row, vec![Cell::default(); self.cols]);
                        if self.grid.len() > self.rows { self.grid.pop(); }
                    }
                }
            }
            // DL — Delete Lines
            'M' => {
                let n = p(params, 0, 1);
                for _ in 0..n {
                    if self.cursor_row < self.grid.len() {
                        self.grid.remove(self.cursor_row);
                        self.grid.push(vec![Cell::default(); self.cols]);
                    }
                }
            }
            // DCH — Delete Characters
            'P' => {
                let n = p(params, 0, 1);
                let row = &mut self.grid[self.cursor_row];
                for _ in 0..n {
                    if self.cursor_col < row.len() {
                        row.remove(self.cursor_col);
                        row.push(Cell::default());
                    }
                }
            }
            // SU — Scroll Up
            'S' => {
                let n = p(params, 0, 1);
                for _ in 0..n { self.scroll_up(); }
            }
            // SD — Scroll Down
            'T' => {
                let n = p(params, 0, 1);
                for _ in 0..n {
                    if !self.grid.is_empty() {
                        self.grid.pop();
                        self.grid.insert(0, vec![Cell::default(); self.cols]);
                    }
                }
            }
            // ECH — Erase Characters
            'X' => {
                let n = p(params, 0, 1);
                for i in 0..n {
                    let col = self.cursor_col + i;
                    if col < self.cols {
                        self.grid[self.cursor_row][col] = Cell::default();
                    }
                }
            }
            // ICH — Insert Characters
            '@' => {
                let n = p(params, 0, 1);
                let row = &mut self.grid[self.cursor_row];
                for _ in 0..n {
                    row.insert(self.cursor_col, Cell::default());
                    if row.len() > self.cols { row.pop(); }
                }
            }
            // VPA — Line Position Absolute
            'd' => {
                self.cursor_row = p(params, 0, 1).saturating_sub(1).min(self.rows.saturating_sub(1));
            }
            // SGR — Select Graphic Rendition
            'm' => {
                let values: Vec<u16> = params.iter().flat_map(|s| s.iter().copied()).collect();
                let mut i = 0;
                while i < values.len() {
                    match values[i] {
                        0 => self.style = CellStyle::default(),
                        1 => self.style.bold = true,
                        22 => self.style.bold = false,
                        30..=37 => self.style.fg = (values[i] - 30) as u8,
                        39 => self.style.fg = 0,
                        40..=47 => self.style.bg = (values[i] - 40) as u8,
                        49 => self.style.bg = 0,
                        90..=97 => self.style.fg = (values[i] - 90 + 8) as u8,
                        100..=107 => self.style.bg = (values[i] - 100 + 8) as u8,
                        _ => {}
                    }
                    i += 1;
                }
            }
            // DECSTBM, DEC private modes, DSR, DA — acknowledge but ignore
            'h' | 'l' | 'r' | 'n' | 'c' => {}
            _ => {}
        }
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // OSC 0 = set icon name + window title
        // OSC 2 = set window title
        if params.len() >= 2 {
            let cmd = params[0];
            if cmd == b"0" || cmd == b"2" {
                if let Ok(title) = std::str::from_utf8(params[1]) {
                    self.title = title.to_string();
                }
            }
        }
    }
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
}

// ── PTY session ─────────────────────────────────────────────────────────────

pub struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    #[allow(dead_code)]
    master: Box<dyn MasterPty + Send>,
}

/// Kill a tmux session for the given pane ID (no-op if tmux not available).
fn kill_tmux_session(pane_id: &str) {
    if !*TMUX_AVAILABLE {
        return;
    }
    let session_name = format!("mux-{}", pane_id);
    let _ = std::process::Command::new("tmux")
        .args(["-L", "muxspace", "kill-session", "-t", &session_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

impl PtySession {
    pub fn spawn(
        pane_id: &str,
        cwd: &Path,
        command: Option<&str>,
        rows: u16,
        cols: u16,
    ) -> Result<(Self, std::sync::mpsc::Receiver<Vec<u8>>)> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .context("opening PTY")?;

        let mut cmd = if *TMUX_AVAILABLE {
            // Use tmux with a dedicated socket so sessions persist across app restarts
            let session_name = format!("mux-{}", pane_id);
            let config = TMUX_CONFIG.to_string_lossy().to_string();
            let mut c = CommandBuilder::new("tmux");
            c.args(["-L", "muxspace", "-f", &config, "new-session", "-A", "-s", &session_name]);
            if let Some(initial_cmd) = command {
                c.arg(initial_cmd);
            }
            tracing::info!("PTY using tmux session '{}'", session_name);
            c
        } else {
            let shell = command
                .map(|s| s.to_string())
                .or_else(|| std::env::var("SHELL").ok())
                .unwrap_or_else(|| "/bin/sh".to_string());
            CommandBuilder::new(&shell)
        };
        cmd.cwd(cwd);

        let _child = pair.slave.spawn_command(cmd).context("spawning shell")?;

        let mut reader = pair.master.try_clone_reader().context("cloning reader")?;
        let writer = pair.master.take_writer().context("taking writer")?;

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok((
            Self {
                writer: Arc::new(Mutex::new(writer)),
                master: pair.master,
            },
            rx,
        ))
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.writer.lock().unwrap().write_all(data)?;
        Ok(())
    }
}
