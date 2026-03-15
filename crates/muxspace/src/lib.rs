//! Muxspace - Terminal Workspace Manager
//! 
//! This library provides the core functionality for managing development workspaces
//! with integrated terminal panes, browser profiles, and external tools.

pub mod ansi;
pub mod detect;
pub mod orchestrator;
pub mod pty;
pub mod state;
pub mod tui;
pub mod types;

// Re-export commonly used types
pub use types::{
    GlobalConfig, WorkspaceConfig, PaneConfig, ToolConfig, ToolKind,
    BrowserConfig, BrowserKind, Workspace, Pane, Project,
};

pub use detect::{load_or_init_config, save_config, detect_all_tools};
pub use state::StateManager;
pub use orchestrator::{launch_external, shutdown_external};
pub use pty::PtyPane;

use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::watch;

/// Main application controller for GUI/Tauri integration
pub struct MuxspaceApp {
    pub state: StateManager,
    pub config: GlobalConfig,
}

impl MuxspaceApp {
    /// Initialize the application
    pub fn new() -> Result<Self> {
        let config = load_or_init_config()?;
        let state = StateManager::open()?;
        Ok(Self { state, config })
    }
    
    /// Get list of all projects
    pub fn list_projects(&self) -> Vec<(Option<String>, Vec<String>)> {
        self.state.list_projects()
    }
    
    /// Get list of all workspaces
    pub fn list_workspaces(&self) -> Vec<crate::types::WorkspaceSummary> {
        self.state.list_summaries()
    }
    
    /// Get a workspace configuration
    pub fn get_workspace(&self, name: &str) -> Option<&WorkspaceConfig> {
        self.state.get_config(name)
    }
    
    /// Start a workspace (launch external tools)
    pub fn start_workspace(&mut self, cfg: &WorkspaceConfig) -> Result<Vec<u32>> {
        self.state.start_workspace(cfg.clone())?;
        launch_external(cfg)
    }
    
    /// Get the last active project
    pub fn get_last_active_project(&self) -> Result<Option<String>> {
        self.state.get_last_active_project()
    }
    
    /// Get ALL active projects
    pub fn get_active_projects(&self) -> Result<Vec<String>> {
        self.state.get_active_projects()
    }
    
    /// Add an active project
    pub fn add_active_project(&self, name: &str) -> Result<()> {
        self.state.add_active_project(name)
    }
    
    /// Export workspaces to sync directory
    pub fn export_workspaces(&self) -> Result<()> {
        let configs: Vec<_> = self.state.all_configs().into_iter().cloned().collect();
        let sync_dir = crate::types::default_sync_dir();
        state::export_workspaces(&configs, &sync_dir)
    }
    
    /// Import workspaces from sync directory
    pub fn import_workspaces(&mut self, path: Option<PathBuf>) -> Result<usize> {
        let sync_dir = path.unwrap_or_else(crate::types::default_sync_dir);
        let configs = state::import_workspaces(&sync_dir)?;
        let count = configs.len();
        for cfg in configs {
            self.state.upsert_config(cfg)?;
        }
        Ok(count)
    }
    
    /// Detect all available tools on the system
    pub fn detect_tools(&self) -> Vec<crate::types::ToolInfo> {
        detect_all_tools()
    }
}

/// Run the terminal UI for a workspace
pub async fn run_tui(
    workspace_names: Vec<String>,
    panes: Vec<tui::PaneState>,
    projects: Vec<tui::ProjectEntry>,
) -> Result<()> {
    let (tx, rx) = watch::channel(());
    let mut app = tui::AppState::new(workspace_names, panes);
    app.projects = projects;
    tui::run(&mut app, rx).await?;
    drop(tx);
    Ok(())
}

/// Create a demo pane for testing
pub fn create_demo_pane() -> tui::PaneState {
    tui::PaneState::demo("welcome", &[
        "Welcome to Muxspace!",
        "",
        "This is a terminal workspace manager.",
        "",
        "Features:",
        "  • Multiple terminal panes",
        "  • Project-based workspace organization",
        "  • Browser profile preservation",
        "  • AI assistant integration",
        "  • Cross-device sync",
    ])
}
