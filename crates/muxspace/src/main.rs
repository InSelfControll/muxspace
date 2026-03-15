use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use muxspace::{
    load_or_init_config, orchestrator::{self, launch_external, shutdown_external}, pty::PtyPane,
    state, tui::{self, AppState, PaneState, ProjectEntry}, types, StateManager, WorkspaceConfig,
};
use muxspace::types::workspaces_dir;
use std::path::{Path, PathBuf};
use tokio::sync::watch;

#[derive(Parser)]
#[command(name = "muxspace", version, about = "Terminal workspace manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a workspace from a YAML config file or by name
    Start {
        /// Workspace name or path to a .yaml file
        /// (looks in ~/.config/muxspace/workspaces/<name>.yaml if not a path)
        name_or_path: Option<String>,
    },
    /// List all persisted workspaces
    List,
    /// Restore all previously saved workspaces
    Restore,
    /// Open a demo TUI without any workspace config
    Tui,
    /// Sync workspace configs to/from a shared directory (cross-device)
    #[command(subcommand)]
    Sync(SyncCommands),
    /// Quick switch to a project
    Project {
        /// Project name to switch to
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Export all workspace configs to the sync directory
    Export,
    /// Import workspace configs from a sync directory
    Import {
        #[arg(default_value = "")]
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let _config = load_or_init_config()?;

    match cli.command {
        Commands::Tui => {
            run_demo_tui().await?;
        }

        Commands::Start { name_or_path } => {
            match resolve_and_load_workspace(name_or_path.as_deref())? {
                Some(ws_cfg) => {
                    run_workspace(ws_cfg).await?;
                }
                None => {
                    eprintln!("No workspace config found. Running demo TUI.");
                    eprintln!("Tip: create ~/.config/muxspace/workspaces/<name>.yaml");
                    run_demo_tui().await?;
                }
            }
        }

        Commands::List => {
            let state = StateManager::open()?;
            let summaries = state.list_summaries();
            if summaries.is_empty() {
                println!("No saved workspaces.");
            } else {
                let by_project = state.list_projects();
                for (proj, wss) in by_project {
                    println!("[{}]", proj.as_deref().unwrap_or("no project"));
                    for name in wss {
                        if let Some(ws) = summaries.iter().find(|s| s.name == name) {
                            println!("  • {} ({} pane{})", ws.name, ws.pane_count,
                                if ws.pane_count == 1 { "" } else { "s" });
                        }
                    }
                }
            }
        }

        Commands::Restore => {
            run_restore().await?;
        }

        Commands::Sync(sub) => match sub {
            SyncCommands::Export => {
                let state = StateManager::open()?;
                let configs: Vec<_> = state.all_configs().into_iter().cloned().collect();
                let sync_dir = types::default_sync_dir();
                state::export_workspaces(&configs, &sync_dir)?;
                println!("Exported {} workspace(s) to {}", configs.len(), sync_dir.display());
            }
            SyncCommands::Import { path } => {
                let sync_dir = if path.is_empty() {
                    types::default_sync_dir()
                } else {
                    PathBuf::from(path)
                };
                let configs = state::import_workspaces(&sync_dir)?;
                let count = configs.len();
                let mut state = StateManager::open()?;
                for cfg in configs {
                    state.upsert_config(cfg)?;
                }
                println!("Imported {count} workspace(s) from {}", sync_dir.display());
            }
        },
        
        Commands::Project { name } => {
            if let Some(project_name) = name {
                // Switch to specific project
                switch_to_project(&project_name).await?;
            } else {
                // Show project selector
                run_project_selector().await?;
            }
        }
    }
    Ok(())
}

// ── Boot Sequence & Restoration ───────────────────────────────────────────────

async fn run_restore() -> Result<()> {
    let mut state = StateManager::open()?;
    state.restore()?;
    
    // Get ALL active projects (sorted by last active time)
    let active_projects = state.get_active_projects()?;
    
    let configs: Vec<WorkspaceConfig> = state.all_configs().into_iter().cloned().collect();
    if configs.is_empty() {
        println!("Nothing to restore.");
        return Ok(());
    }
    
    println!("Found {} workspace(s) across {} project(s)...", configs.len(), active_projects.len().max(1));
    
    // If we have active projects, restore external tools for ALL of them
    if !active_projects.is_empty() {
        println!("\nActive projects: {}", active_projects.join(", "));
        
        // Launch external tools for all workspaces in all active projects
        let mut total_external: usize = 0;
        for project_name in &active_projects {
            let workspace_names = state.workspaces_for_project(project_name);
            for ws_name in &workspace_names {
                if let Some(cfg) = configs.iter().find(|c| c.name == *ws_name) {
                    // Launch external tools (browsers, IDEs) without blocking
                    match orchestrator::launch_external(cfg) {
                        Ok(pids) => {
                            total_external += pids.len();
                            tracing::info!("Launched external tools for '{}' ({} pids)", cfg.name, pids.len());
                        }
                        Err(e) => {
                            tracing::warn!("Failed to launch external tools for '{}': {}", cfg.name, e);
                        }
                    }
                }
            }
        }
        
        if total_external > 0 {
            println!("Launched {} external tool process(es) across all projects", total_external);
        }
        
        // Now run the TUI for the most recently active project
        let last_project = active_projects.first().unwrap();
        let workspace_names = state.workspaces_for_project(last_project);
        
        if let Some(first_ws) = workspace_names.first() {
            if let Some(cfg) = configs.iter().find(|c| c.name == *first_ws) {
                println!("\nStarting TUI for project '{}'...", last_project);
                println!("(Use Ctrl+B P to switch between projects)\n");
                run_workspace(cfg.clone()).await?;
                return Ok(());
            }
        }
    }
    
    // Fallback: restore the first workspace if no active projects
    if let Some(cfg) = configs.into_iter().next() {
        if let Some(ref proj) = cfg.project {
            state.add_active_project(proj)?;
        }
        run_workspace(cfg).await?;
    }
    
    Ok(())
}

async fn switch_to_project(project_name: &str) -> Result<()> {
    let state = StateManager::open()?;
    let workspace_names = state.workspaces_for_project(project_name);
    
    if workspace_names.is_empty() {
        println!("No workspaces found for project '{}'", project_name);
        return Ok(());
    }
    
    // Get first workspace config
    if let Some(cfg) = state.get_config(&workspace_names[0]) {
        println!("Switching to project '{}' with workspace '{}'...", project_name, cfg.name);
        run_workspace(cfg.clone()).await?;
    }
    
    Ok(())
}

async fn run_project_selector() -> Result<()> {
    let mut state = StateManager::open()?;
    state.restore()?;
    
    let projects_data = state.list_projects();
    let projects: Vec<ProjectEntry> = projects_data
        .into_iter()
        .map(|(name, workspaces)| ProjectEntry {
            name: name.unwrap_or_else(|| "default".to_string()),
            workspaces,
        })
        .collect();
    
    if projects.is_empty() {
        println!("No projects found. Create a workspace first with 'muxspace start <name>'");
        return Ok(());
    }
    
    // Show TUI project navigator
    let (tx, rx) = watch::channel(());
    let mut app = AppState::new(vec!["project-selector".into()], vec![
        PaneState::demo("info", &["Select a project to open..."])
    ]);
    app.projects = projects;
    app.mode = tui::AppMode::ProjectNavigator;
    
    // Set up callback for project switch
    let _project_names: Vec<String> = app.projects.iter().map(|p| p.name.clone()).collect();
    let switch_tx = std::sync::Arc::new(std::sync::Mutex::new(None));
    let switch_tx_clone = switch_tx.clone();
    
    app.on_project_switch = Some(Box::new(move |name: &str| {
        *switch_tx_clone.lock().unwrap() = Some(name.to_string());
    }));
    
    tui::run(&mut app, rx).await?;
    drop(tx);
    
    // Check if a project was selected
    if let Some(selected) = switch_tx.lock().unwrap().take() {
        switch_to_project(&selected).await?;
    }
    
    Ok(())
}

// ── Demo TUI ──────────────────────────────────────────────────────────────────

async fn run_demo_tui() -> Result<()> {
    let (tx, rx) = watch::channel(());
    let pane = PaneState::demo("shell", &[
        "╔══════════════════════════════════════════════════════════╗",
        "║  Muxspace — Terminal Workspace Manager                   ║",
        "╠══════════════════════════════════════════════════════════╣",
        "║  Keybindings (Ctrl+B prefix):                            ║",
        "║    n / p  — next / prev pane                             ║",
        "║    w      — next workspace                               ║",
        "║    P      — project navigator                            ║",
        "║    /      — search in scrollback                         ║",
        "║    [ / ]  — scroll up / down                             ║",
        "║    q      — quit                                         ║",
        "║                                                          ║",
        "║  Create a workspace config at:                           ║",
        "║    ~/.config/muxspace/workspaces/<name>.yaml             ║",
        "╚══════════════════════════════════════════════════════════╝",
    ]);
    let mut app = AppState::new(vec!["demo".into()], vec![pane]);
    tui::run(&mut app, rx).await?;
    drop(tx);
    Ok(())
}

// ── Workspace Lifecycle ───────────────────────────────────────────────────────

async fn run_workspace(cfg: WorkspaceConfig) -> Result<()> {
    // Persist to SQLite
    let mut state = StateManager::open()?;
    state.start_workspace(cfg.clone())?;
    
    // Add to active projects list (for multi-project restoration)
    if let Some(ref project) = cfg.project {
        state.add_active_project(project)?;
    }

    // Launch external apps (browsers, GUI IDEs) — non-blocking spawns
    let external_pids = match launch_external(&cfg) {
        Ok(pids) => pids,
        Err(e) => {
            tracing::warn!("External app launch failed: {e}");
            vec![]
        }
    };

    // Determine initial pane dimensions
    let (term_cols, term_rows) = crossterm::terminal::size().unwrap_or((220, 50));
    let pane_count = cfg.panes.len().max(1);
    let pane_cols = (term_cols / pane_count as u16).max(20);
    let pane_rows = term_rows.saturating_sub(3);

    // Create shared redraw signal
    let (redraw_tx, redraw_rx) = watch::channel(());

    // Spawn a PTY for each pane config
    let mut panes: Vec<PaneState> = Vec::new();
    for (i, pane_cfg) in cfg.panes.iter().enumerate() {
        let cwd = expand_path(&pane_cfg.cwd);
        let cmd = pane_cfg.command.as_deref();

        let title = cmd
            .unwrap_or("shell")
            .split_whitespace()
            .next()
            .unwrap_or("pane")
            .to_string();

        match PtyPane::spawn(&cwd, cmd, pane_rows, pane_cols, redraw_tx.clone()) {
            Ok(pty) => panes.push(PaneState::from_pty(
                format!("{} [{}]", title, i + 1),
                pty,
            )),
            Err(e) => {
                tracing::error!("Failed to spawn PTY for pane {}: {e}", i + 1);
                panes.push(PaneState::demo(
                    format!("pane {} (error)", i + 1),
                    &[&format!("PTY spawn failed: {e}")],
                ));
            }
        }
    }

    // If no pane configs, open one shell in the first tool's path
    if panes.is_empty() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        match PtyPane::spawn(&cwd, None, pane_rows, pane_cols, redraw_tx.clone()) {
            Ok(pty) => panes.push(PaneState::from_pty("shell", pty)),
            Err(_) => panes.push(PaneState::demo("shell", &["Failed to spawn shell"])),
        }
    }

    // Build project list for navigator
    let projects = build_project_list(&state)?;

    let workspace_name = cfg.name.clone();
    let mut app = AppState::new(vec![workspace_name], panes);
    app.projects = projects;
    
    // Set up project switch callback
    let state_for_switch = std::sync::Arc::new(std::sync::Mutex::new(state));
    let state_clone = state_for_switch.clone();
    app.on_project_switch = Some(Box::new(move |project_name: &str| {
        // This will be handled after the TUI exits
        let _ = state_clone.lock().unwrap().add_active_project(project_name);
    }));
    
    // Run the TUI
    tui::run(&mut app, redraw_rx).await?;
    
    // Cleanup: shutdown external processes
    if !external_pids.is_empty() {
        shutdown_external(&external_pids);
    }
    
    drop(redraw_tx);
    Ok(())
}

fn build_project_list(state: &StateManager) -> Result<Vec<ProjectEntry>> {
    let projects_data = state.list_projects();
    let mut entries = Vec::new();
    
    for (proj_name, ws_names) in projects_data {
        let name = proj_name.unwrap_or_else(|| "default".to_string());
        entries.push(ProjectEntry {
            name,
            workspaces: ws_names,
        });
    }
    
    // If no projects found, create a default one
    if entries.is_empty() {
        entries.push(ProjectEntry {
            name: "default".to_string(),
            workspaces: vec![],
        });
    }
    
    Ok(entries)
}

// ── Config Loading ────────────────────────────────────────────────────────────

fn resolve_and_load_workspace(arg: Option<&str>) -> Result<Option<WorkspaceConfig>> {
    let path = match arg {
        None => return Ok(None),
        Some(s) => {
            let p = Path::new(s);
            if p.exists() {
                p.to_path_buf()
            } else {
                // Look in ~/.config/muxspace/workspaces/<name>.yaml
                let candidate = workspaces_dir().join(format!("{}.yaml", s));
                if candidate.exists() {
                    candidate
                } else {
                    return Ok(None);
                }
            }
        }
    };

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading workspace config {}", path.display()))?;
    let cfg: WorkspaceConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("parsing workspace config {}", path.display()))?;
    Ok(Some(cfg))
}

fn expand_path(p: &Path) -> PathBuf {
    let s = shellexpand::tilde(&p.to_string_lossy()).into_owned();
    PathBuf::from(s)
}
