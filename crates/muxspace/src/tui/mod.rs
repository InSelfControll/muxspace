pub mod layout;
pub mod render;

use anyhow::Result;
use crossterm::{
    cursor,
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyModifiers, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

use crate::ansi::ScreenBuffer;
use crate::pty::PtyPane;

/// Per-pane state, always backed by a ScreenBuffer.
pub struct PaneState {
    pub screen: Arc<Mutex<ScreenBuffer>>,
    pub pty: Option<Arc<PtyPane>>,
    pub scroll_offset: usize,
    pub title: String,
    /// Search mode state
    pub search_query: String,
    pub search_matches: Vec<(usize, usize)>, // (row, col) positions
    pub current_match: usize,
}

impl PaneState {
    /// Demo pane (no PTY) — pre-populate the screen buffer with text lines.
    pub fn demo(title: impl Into<String>, lines: &[&str]) -> Self {
        let screen = Arc::new(Mutex::new(ScreenBuffer::new(24, 80)));
        {
            let mut s = screen.lock().unwrap();
            for line in lines {
                s.process(format!("{}\r\n", line).as_bytes());
            }
        }
        PaneState { 
            screen, 
            pty: None, 
            scroll_offset: 0, 
            title: title.into(),
            search_query: String::new(),
            search_matches: vec![],
            current_match: 0,
        }
    }

    pub fn from_pty(title: impl Into<String>, pty: Arc<PtyPane>) -> Self {
        PaneState {
            screen: Arc::clone(&pty.screen),
            pty: Some(pty),
            scroll_offset: 0,
            title: title.into(),
            search_query: String::new(),
            search_matches: vec![],
            current_match: 0,
        }
    }

    pub fn scroll_up(&mut self)   { self.scroll_offset = self.scroll_offset.saturating_add(3); }
    pub fn scroll_down(&mut self) { self.scroll_offset = self.scroll_offset.saturating_sub(3); }
    
    /// Search for a query in the scrollback and grid
    pub fn search(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.search_matches.clear();
        self.current_match = 0;
        
        if query.is_empty() {
            return;
        }
        
        let screen = self.screen.lock().unwrap();
        
        // Search scrollback
        for (row_idx, row) in screen.scrollback.iter().enumerate() {
            let line: String = row.iter().map(|c| c.ch).collect();
            for (col_idx, _) in line.match_indices(query) {
                self.search_matches.push((row_idx, col_idx));
            }
        }
        
        // Search current grid
        let scrollback_len = screen.scrollback.len();
        for (row_idx, row) in screen.grid.iter().enumerate() {
            let line: String = row.iter().map(|c| c.ch).collect();
            for (col_idx, _) in line.match_indices(query) {
                self.search_matches.push((scrollback_len + row_idx, col_idx));
            }
        }
    }
    
    pub fn next_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.search_matches.len();
            // Auto-scroll to match
            let (row, _) = self.search_matches[self.current_match];
            let screen = self.screen.lock().unwrap();
            let total_rows = screen.scrollback.len() + screen.rows;
            let target_scroll = total_rows.saturating_sub(row + 5);
            self.scroll_offset = target_scroll.min(screen.scrollback.len());
        }
    }
    
    pub fn prev_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.current_match = self.current_match.checked_sub(1)
                .unwrap_or(self.search_matches.len() - 1);
            // Auto-scroll to match
            let (row, _) = self.search_matches[self.current_match];
            let screen = self.screen.lock().unwrap();
            let total_rows = screen.scrollback.len() + screen.rows;
            let target_scroll = total_rows.saturating_sub(row + 5);
            self.scroll_offset = target_scroll.min(screen.scrollback.len());
        }
    }
}

/// Application UI mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    WorkspaceView,
    ProjectNavigator,
    SearchMode,
}

/// Project representation for the navigator
#[derive(Debug, Clone)]
pub struct ProjectEntry {
    pub name: String,
    pub workspaces: Vec<String>,
}

/// Central application state for the TUI
pub struct AppState {
    pub mode: AppMode,
    pub workspace_names: Vec<String>,
    pub active_workspace: usize,
    pub panes: Vec<PaneState>,
    pub active_pane: usize,
    pub prefix_mode: bool,
    
    // Project navigator state
    pub projects: Vec<ProjectEntry>,
    pub selected_project: usize,
    
    // Search state
    pub search_query: String,
    
    // Project switch callback
    pub on_project_switch: Option<Box<dyn Fn(&str) + Send>>,
}

impl AppState {
    pub fn new(workspace_names: Vec<String>, panes: Vec<PaneState>) -> Self {
        AppState {
            mode: AppMode::WorkspaceView,
            workspace_names,
            active_workspace: 0,
            panes,
            active_pane: 0,
            prefix_mode: false,
            projects: Vec::new(),
            selected_project: 0,
            search_query: String::new(),
            on_project_switch: None,
        }
    }

    pub fn next_pane(&mut self) {
        if !self.panes.is_empty() {
            self.active_pane = (self.active_pane + 1) % self.panes.len();
        }
    }

    pub fn prev_pane(&mut self) {
        if !self.panes.is_empty() {
            self.active_pane = (self.active_pane + self.panes.len() - 1) % self.panes.len();
        }
    }

    pub fn next_workspace(&mut self) {
        if !self.workspace_names.is_empty() {
            self.active_workspace = (self.active_workspace + 1) % self.workspace_names.len();
        }
    }

    pub fn next_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project = (self.selected_project + 1) % self.projects.len();
        }
    }

    pub fn prev_project(&mut self) {
        if !self.projects.is_empty() {
            self.selected_project = (self.selected_project + self.projects.len() - 1) % self.projects.len();
        }
    }

    /// Resize all pane screen buffers and PTYs to match new terminal dimensions.
    pub fn resize(&mut self, term_rows: u16, term_cols: u16) {
        let pane_rows = term_rows.saturating_sub(3);
        let pane_cols = (term_cols / self.panes.len().max(1) as u16).max(10);
        for pane in &self.panes {
            if let Some(pty) = &pane.pty {
                let _ = pty.resize(pane_rows, pane_cols);
            }
        }
    }
    
    /// Get currently selected project name
    pub fn selected_project_name(&self) -> Option<&str> {
        self.projects.get(self.selected_project).map(|p| p.name.as_str())
    }
}

/// Run the full async TUI event loop.
pub async fn run(app: &mut AppState, redraw_rx: watch::Receiver<()>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, cursor::Hide)?;
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, app, redraw_rx).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        cursor::Show
    )?;
    terminal.show_cursor()?;
    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<&mut std::io::Stdout>>,
    app: &mut AppState,
    mut redraw_rx: watch::Receiver<()>,
) -> Result<()> {
    let mut events = EventStream::new();

    loop {
        terminal.draw(|f| render::draw(f, app))?;

        tokio::select! {
            maybe_ev = events.next() => {
                match maybe_ev {
                    Some(Ok(Event::Key(key))) => {
                        if handle_key(app, key).await { return Ok(()); }
                    }
                    Some(Ok(Event::Mouse(mouse))) => handle_mouse(app, mouse),
                    Some(Ok(Event::Resize(cols, rows))) => app.resize(rows, cols),
                    None | Some(Err(_)) => return Ok(()),
                    _ => {}
                }
            }
            // PTY produced output — just redraw
            _ = redraw_rx.changed() => {}
        }
    }
}

/// Returns `true` when the app should quit.
async fn handle_key(app: &mut AppState, key: KeyEvent) -> bool {
    match app.mode {
        AppMode::ProjectNavigator => handle_project_nav_key(app, key).await,
        AppMode::SearchMode => handle_search_key(app, key),
        AppMode::WorkspaceView => handle_workspace_key(app, key),
    }
}

async fn handle_project_nav_key(app: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::WorkspaceView;
            false
        }
        KeyCode::Enter => {
            // Switch to selected project
            if let Some(project_name) = app.selected_project_name() {
                let name = project_name.to_string();
                if let Some(callback) = &app.on_project_switch {
                    callback(&name);
                }
                app.mode = AppMode::WorkspaceView;
            }
            false
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.prev_project();
            false
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_project();
            false
        }
        _ => false,
    }
}

fn handle_search_key(app: &mut AppState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::WorkspaceView;
            app.search_query.clear();
            if let Some(pane) = app.panes.get_mut(app.active_pane) {
                pane.search("");
            }
            false
        }
        KeyCode::Enter => {
            // Execute search
            if let Some(pane) = app.panes.get_mut(app.active_pane) {
                pane.search(&app.search_query);
            }
            false
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            false
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            false
        }
        KeyCode::Tab => {
            // Next match
            if let Some(pane) = app.panes.get_mut(app.active_pane) {
                pane.next_match();
            }
            false
        }
        _ => false,
    }
}

fn handle_workspace_key(app: &mut AppState, key: KeyEvent) -> bool {
    if app.prefix_mode {
        app.prefix_mode = false;
        return match key.code {
            KeyCode::Char('n') => { app.next_pane(); false }
            KeyCode::Char('p') => { app.prev_pane(); false }
            KeyCode::Char('w') => { app.next_workspace(); false }
            KeyCode::Char('P') => {
                // Open project navigator
                app.mode = AppMode::ProjectNavigator;
                false
            }
            KeyCode::Char('/') => {
                // Open search mode
                app.mode = AppMode::SearchMode;
                app.search_query.clear();
                false
            }
            KeyCode::Char('[') => {
                if let Some(p) = app.panes.get_mut(app.active_pane) { p.scroll_up(); }
                false
            }
            KeyCode::Char(']') => {
                if let Some(p) = app.panes.get_mut(app.active_pane) { p.scroll_down(); }
                false
            }
            KeyCode::Char('q') => true,
            _ => false,
        };
    }

    // Ctrl+B → enter prefix mode
    if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.prefix_mode = true;
        return false;
    }

    // Forward all other keys to the active pane's PTY
    if let Some(bytes) = key_to_bytes(key) {
        if let Some(pane) = app.panes.get(app.active_pane) {
            if let Some(pty) = &pane.pty {
                let _ = pty.write(&bytes);
            }
        }
    }
    false
}

fn handle_mouse(app: &mut AppState, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp   => { if let Some(p) = app.panes.get_mut(app.active_pane) { p.scroll_up(); } }
        MouseEventKind::ScrollDown => { if let Some(p) = app.panes.get_mut(app.active_pane) { p.scroll_down(); } }
        _ => {}
    }
}

/// Translate a crossterm key event into the raw bytes a terminal expects.
fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let b = c.to_ascii_lowercase() as u8;
                if b.is_ascii_alphabetic() {
                    return Some(vec![b - b'a' + 1]);
                }
            }
            let mut buf = [0u8; 4];
            Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
        }
        KeyCode::Enter     => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Delete    => Some(b"\x1b[3~".to_vec()),
        KeyCode::Tab       => Some(vec![b'\t']),
        KeyCode::Esc       => Some(vec![0x1b]),
        KeyCode::Up        => Some(b"\x1b[A".to_vec()),
        KeyCode::Down      => Some(b"\x1b[B".to_vec()),
        KeyCode::Right     => Some(b"\x1b[C".to_vec()),
        KeyCode::Left      => Some(b"\x1b[D".to_vec()),
        KeyCode::Home      => Some(b"\x1b[H".to_vec()),
        KeyCode::End       => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp    => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown  => Some(b"\x1b[6~".to_vec()),
        KeyCode::F(1)  => Some(b"\x1bOP".to_vec()),
        KeyCode::F(2)  => Some(b"\x1bOQ".to_vec()),
        KeyCode::F(3)  => Some(b"\x1bOR".to_vec()),
        KeyCode::F(4)  => Some(b"\x1bOS".to_vec()),
        KeyCode::F(5)  => Some(b"\x1b[15~".to_vec()),
        KeyCode::F(6)  => Some(b"\x1b[17~".to_vec()),
        KeyCode::F(7)  => Some(b"\x1b[18~".to_vec()),
        KeyCode::F(8)  => Some(b"\x1b[19~".to_vec()),
        KeyCode::F(9)  => Some(b"\x1b[20~".to_vec()),
        KeyCode::F(10) => Some(b"\x1b[21~".to_vec()),
        KeyCode::F(11) => Some(b"\x1b[23~".to_vec()),
        KeyCode::F(12) => Some(b"\x1b[24~".to_vec()),
        _ => None,
    }
}
