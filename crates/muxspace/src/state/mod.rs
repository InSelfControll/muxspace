use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{
    db_path, Pane, Workspace, WorkspaceConfig, WorkspaceSummary,
};

pub struct StateManager {
    conn: Connection,
    active: HashMap<String, Workspace>,
    configs: HashMap<String, WorkspaceConfig>,
    projects: Vec<Project>,
    active_project_idx: usize,
}

impl StateManager {
    pub fn open() -> Result<Self> {
        let db_path = db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("creating state directory")?;
        }
        let conn = Connection::open(&db_path)
            .with_context(|| format!("opening state DB at {}", db_path.display()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                name TEXT PRIMARY KEY,
                data TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS workspace_configs (
                name TEXT PRIMARY KEY,
                data TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS app_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS active_projects (
                project_name TEXT PRIMARY KEY,
                last_active INTEGER DEFAULT (strftime('%s', 'now'))
             );",
        )
        .context("creating tables")?;
        Ok(Self {
            conn,
            active: HashMap::new(),
            configs: HashMap::new(),
            projects: Vec::new(),
            active_project_idx: 0,
        })
    }

    /// Get the last active project name from the database
    pub fn get_last_active_project(&self) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT project_name FROM active_projects ORDER BY last_active DESC LIMIT 1")
            .context("preparing query")?;
        let result: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .ok();
        Ok(result)
    }

    /// Get ALL recently active projects (sorted by last active time)
    pub fn get_active_projects(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT project_name FROM active_projects ORDER BY last_active DESC")
            .context("preparing query")?;
        let projects: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(projects)
    }

    /// Add or update an active project
    pub fn add_active_project(&self, name: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO active_projects (project_name, last_active) 
                 VALUES (?1, strftime('%s', 'now'))
                 ON CONFLICT(project_name) 
                 DO UPDATE SET last_active = strftime('%s', 'now')",
                params![name],
            )
            .context("saving active project")?;
        Ok(())
    }

    /// Remove a project from active list
    pub fn remove_active_project(&self, name: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM active_projects WHERE project_name = ?1",
                params![name],
            )
            .context("removing active project")?;
        Ok(())
    }

    /// Clear all active projects
    pub fn clear_active_projects(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM active_projects", [])
            .context("clearing active projects")?;
        Ok(())
    }

    pub fn list_summaries(&self) -> Vec<WorkspaceSummary> {
        self.active
            .values()
            .map(|ws| WorkspaceSummary {
                name: ws.name.clone(),
                project: ws.project.clone(),
                pane_count: ws.panes.len(),
            })
            .collect()
    }

    pub fn list_workspace_names(&self) -> Vec<String> {
        self.active.keys().cloned().collect()
    }

    pub fn start_workspace(&mut self, cfg: WorkspaceConfig) -> Result<String> {
        let name = cfg.name.clone();
        let project = cfg.project.clone();
        let panes = cfg
            .panes
            .iter()
            .map(|p| Pane {
                cwd: p.cwd.clone(),
                command: p.command.clone(),
                pid: None,
            })
            .collect();
        let ws = Workspace {
            name: name.clone(),
            project: project.clone(),
            panes,
            external_pids: vec![],
        };
        self.persist_workspace(&ws)?;
        self.persist_config(&cfg)?;
        self.active.insert(name.clone(), ws);
        self.configs.insert(name.clone(), cfg);
        
        // Track this project as active
        if let Some(proj) = project {
            self.add_active_project(&proj)?;
        }
        
        Ok(name)
    }

    /// Return the names of all workspaces belonging to `project`.
    pub fn workspaces_for_project(&self, project: &str) -> Vec<String> {
        self.active
            .values()
            .filter(|ws| ws.project.as_deref() == Some(project))
            .map(|ws| ws.name.clone())
            .collect()
    }

    /// Get all unique project names from loaded workspaces
    pub fn list_projects(&self) -> Vec<(Option<String>, Vec<String>)> {
        let mut by_project: Vec<(Option<String>, Vec<String>)> = Vec::new();
        for (name, ws) in &self.active {
            if let Some(e) = by_project.iter_mut().find(|(p, _)| p == &ws.project) {
                e.1.push(name.clone());
            } else {
                by_project.push((ws.project.clone(), vec![name.clone()]));
            }
        }
        by_project
    }

    pub fn restore(&mut self) -> Result<()> {
        // Restore workspaces
        let mut stmt = self
            .conn
            .prepare("SELECT name, data FROM workspaces")
            .context("preparing restore query")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .context("querying workspaces")?
            .collect::<rusqlite::Result<_>>()
            .context("collecting workspace rows")?;

        for (name, data) in rows {
            let ws: Workspace = serde_json::from_str(&data)
                .with_context(|| format!("deserializing workspace '{name}'"))?;
            tracing::info!("Restored workspace '{}', project={:?}", ws.name, ws.project);
            self.active.insert(name, ws);
        }

        // Restore configs
        let mut stmt = self
            .conn
            .prepare("SELECT name, data FROM workspace_configs")
            .context("preparing config restore query")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .context("querying configs")?
            .collect::<rusqlite::Result<_>>()
            .context("collecting config rows")?;
        for (name, data) in rows {
            if let Ok(cfg) = serde_json::from_str::<WorkspaceConfig>(&data) {
                self.configs.insert(name, cfg);
            }
        }
        Ok(())
    }

    pub fn snapshot_workspace(&mut self, name: &str) -> Result<()> {
        if let Some(ws) = self.active.get(name) {
            let ws = ws.clone();
            self.persist_workspace(&ws)?;
        }
        Ok(())
    }

    pub fn upsert_config(&mut self, cfg: WorkspaceConfig) -> Result<()> {
        let name = cfg.name.clone();
        self.persist_config(&cfg)?;
        self.configs.insert(name, cfg);
        Ok(())
    }

    pub fn all_configs(&self) -> Vec<&WorkspaceConfig> {
        self.configs.values().collect()
    }

    pub fn get_config(&self, name: &str) -> Option<&WorkspaceConfig> {
        self.configs.get(name)
    }

    // ── Persistence helpers ───────────────────────────────────────────────────

    fn persist_workspace(&self, ws: &Workspace) -> Result<()> {
        let data = serde_json::to_string(ws).context("serializing workspace")?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO workspaces (name, data) VALUES (?1, ?2)",
                params![ws.name, data],
            )
            .context("persisting workspace")?;
        Ok(())
    }

    fn persist_config(&self, cfg: &WorkspaceConfig) -> Result<()> {
        let data = serde_json::to_string(cfg).context("serializing workspace config")?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO workspace_configs (name, data) VALUES (?1, ?2)",
                params![cfg.name, data],
            )
            .context("persisting workspace config")?;
        Ok(())
    }
}

// ── Cross-Device Sync ─────────────────────────────────────────────────────────

// Local Project type for state management (runtime only)
pub struct Project {
    pub name: String,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_idx: usize,
}

/// Export all workspace configs to the sync directory.
pub fn export_workspaces(configs: &[WorkspaceConfig], sync_dir: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(sync_dir).context("creating sync directory")?;
    for cfg in configs {
        let yaml = serde_yaml::to_string(cfg).context("serializing config")?;
        let path = sync_dir.join(format!("{}.yaml", cfg.name));
        std::fs::write(&path, yaml)
            .with_context(|| format!("writing sync file {}", path.display()))?;
    }
    Ok(())
}

/// Import workspace configs from a sync directory.
pub fn import_workspaces(sync_dir: &PathBuf) -> Result<Vec<WorkspaceConfig>> {
    let mut configs = Vec::new();
    if !sync_dir.exists() {
        return Ok(configs);
    }
    for entry in std::fs::read_dir(sync_dir).context("reading sync directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let cfg: WorkspaceConfig = serde_yaml::from_str(&content)
                .with_context(|| format!("parsing {}", path.display()))?;
            configs.push(cfg);
        }
    }
    Ok(configs)
}
