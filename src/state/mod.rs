use crate::pty::{ScreenBuffer, PTY_MANAGER};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Global application state
#[derive(Clone, Debug)]
pub struct AppState {
    pub projects: Vec<Project>,
    pub active_project_id: Option<String>,
    pub screen_buffers: HashMap<String, ScreenBuffer>,
    pub show_create_project: bool,
    pub show_hotkey_editor: bool,
    pub show_shortcuts_help: bool,
    /// Which pane currently has keyboard focus
    pub focused_pane_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_idx: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub panes: Vec<Pane>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pane {
    pub id: String,
    pub kind: PaneKind,
    /// Only used for Terminal panes
    pub pty_id: Option<String>,
    /// User-assigned custom name (overrides auto-detected title)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaneKind {
    Terminal { command: Option<String> },
    Browser {
        /// Legacy single-URL field — kept for deserializing old data.
        #[serde(default, skip_serializing)]
        url: String,
        /// URLs of all open tabs.
        #[serde(default)]
        tabs: Vec<String>,
        /// Index of the currently active tab.
        #[serde(default)]
        active_tab: usize,
    },
}

impl AppState {
    pub fn new_blocking() -> Self {
        let mut projects = Self::load_projects_blocking().unwrap_or_default();
        Self::migrate_browser_tabs(&mut projects);

        Self {
            projects,
            active_project_id: None,
            screen_buffers: HashMap::new(),
            show_create_project: false,
            show_hotkey_editor: false,
            show_shortcuts_help: false,
            focused_pane_id: None,
        }
    }

    /// Migrate old single-URL browser panes to the new multi-tab format.
    fn migrate_browser_tabs(projects: &mut [Project]) {
        for project in projects {
            for workspace in &mut project.workspaces {
                for pane in &mut workspace.panes {
                    if let PaneKind::Browser { ref url, ref mut tabs, .. } = pane.kind {
                        if tabs.is_empty() && !url.is_empty() {
                            tabs.push(url.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn active_project(&self) -> Option<&Project> {
        self.active_project_id
            .as_ref()
            .and_then(|id| self.projects.iter().find(|p| p.id == *id))
    }

    pub fn active_project_mut(&mut self) -> Option<&mut Project> {
        let id = self.active_project_id.clone();
        id.and_then(move |id| self.projects.iter_mut().find(|p| p.id == id))
    }

    /// Get pane IDs in the active workspace (for focus navigation)
    pub fn active_pane_ids(&self) -> Vec<String> {
        self.active_project()
            .map(|p| {
                p.workspaces[p.active_workspace_idx]
                    .panes
                    .iter()
                    .map(|pane| pane.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Focus the next pane in the active workspace
    pub fn focus_next_pane(&mut self) {
        let ids = self.active_pane_ids();
        if ids.is_empty() {
            return;
        }
        let current_idx = self
            .focused_pane_id
            .as_ref()
            .and_then(|id| ids.iter().position(|i| i == id))
            .unwrap_or(0);
        let next = (current_idx + 1) % ids.len();
        self.focused_pane_id = Some(ids[next].clone());
    }

    /// Focus the previous pane in the active workspace
    pub fn focus_prev_pane(&mut self) {
        let ids = self.active_pane_ids();
        if ids.is_empty() {
            return;
        }
        let current_idx = self
            .focused_pane_id
            .as_ref()
            .and_then(|id| ids.iter().position(|i| i == id))
            .unwrap_or(0);
        let prev = if current_idx == 0 {
            ids.len() - 1
        } else {
            current_idx - 1
        };
        self.focused_pane_id = Some(ids[prev].clone());
    }

    /// Switch to a project and spawn PTY sessions for its active workspace
    pub fn switch_project_blocking(&mut self, project_id: &str) {
        self.active_project_id = Some(project_id.to_string());
        self.spawn_ptys_for_active_workspace();
        // Auto-focus first pane
        let ids = self.active_pane_ids();
        self.focused_pane_id = ids.into_iter().next();
    }

    /// Spawn PTYs for all terminal panes in the active workspace
    pub fn spawn_ptys_for_active_workspace(&mut self) {
        let project = match self.active_project() {
            Some(p) => p.clone(),
            None => return,
        };

        let workspace = &project.workspaces[project.active_workspace_idx];
        let cwd = &workspace.cwd;

        let mut mgr = PTY_MANAGER.lock().unwrap();
        let mut updates: Vec<(String, String)> = Vec::new();

        for pane in &workspace.panes {
            // Only spawn PTY for terminal panes
            let command = match &pane.kind {
                PaneKind::Terminal { command } => command.as_deref(),
                PaneKind::Browser { .. } => continue,
            };

            let pty_id = pane.pty_id.clone().unwrap_or_else(|| pane.id.clone());

            if mgr
                .spawn_for_pane(&pty_id, cwd, command)
                .is_ok()
            {
                tracing::info!("PTY spawned for pane {}", pty_id);
            }

            if !self.screen_buffers.contains_key(&pty_id) {
                self.screen_buffers
                    .insert(pty_id.clone(), ScreenBuffer::new(24, 80));
            }

            updates.push((pane.id.clone(), pty_id));
        }

        drop(mgr);

        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            for (pane_id, pty_id) in updates {
                if let Some(pane) = project.workspaces[ws_idx]
                    .panes
                    .iter_mut()
                    .find(|p| p.id == pane_id)
                {
                    if pane.pty_id.is_none() {
                        pane.pty_id = Some(pty_id);
                    }
                }
            }
        }

        self.save_projects_blocking();
    }

    /// Switch workspace within the active project
    pub fn switch_workspace(&mut self, idx: usize) {
        // Hide all embedded browsers — the new workspace's coroutines will re-show theirs
        crate::browser::BROWSER_MGR.lock().unwrap().hide_all();

        if let Some(project) = self.active_project_mut() {
            if idx < project.workspaces.len() {
                project.active_workspace_idx = idx;
            }
        }
        self.spawn_ptys_for_active_workspace();
        let ids = self.active_pane_ids();
        self.focused_pane_id = ids.into_iter().next();
    }

    /// Add a new terminal pane to the active workspace
    pub fn add_terminal_pane(&mut self, command: Option<String>) {
        let pane_id = format!("pane-{}", chrono::Utc::now().timestamp_millis());
        let pane = Pane {
            id: pane_id.clone(),
            kind: PaneKind::Terminal { command },
            pty_id: None,
            custom_name: None,
        };

        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            project.workspaces[ws_idx].panes.push(pane);
        }

        self.spawn_ptys_for_active_workspace();
        self.focused_pane_id = Some(pane_id);
    }


    /// Add a new browser pane to the active workspace
    pub fn add_browser_pane(&mut self, url: String) {
        let pane_id = format!("pane-{}", chrono::Utc::now().timestamp_millis());
        let pane = Pane {
            id: pane_id.clone(),
            kind: PaneKind::Browser {
                url: String::new(),
                tabs: vec![url],
                active_tab: 0,
            },
            pty_id: None,
            custom_name: None,
        };

        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            project.workspaces[ws_idx].panes.push(pane);
        }

        self.save_projects_blocking();
        self.focused_pane_id = Some(pane_id);
    }

    /// Remove a pane from the active workspace
    pub fn remove_pane(&mut self, pane_id: &str) {
        // Destroy embedded browser WebView if this is a browser pane
        if let Some(project) = self.active_project() {
            let ws = &project.workspaces[project.active_workspace_idx];
            if let Some(pane) = ws.panes.iter().find(|p| p.id == pane_id) {
                if matches!(pane.kind, PaneKind::Browser { .. }) {
                    crate::browser::BROWSER_MGR.lock().unwrap().destroy(pane_id);
                }
            }
        }

        // Collect PTY ID to clean up
        let pty_to_remove = self.active_project().and_then(|project| {
            let ws = &project.workspaces[project.active_workspace_idx];
            ws.panes
                .iter()
                .find(|p| p.id == pane_id)
                .and_then(|p| p.pty_id.clone())
        });

        if let Some(pty_id) = &pty_to_remove {
            let mut mgr = PTY_MANAGER.lock().unwrap();
            mgr.remove_pane(pty_id);
            self.screen_buffers.remove(pty_id);
        }

        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            project.workspaces[ws_idx]
                .panes
                .retain(|p| p.id != pane_id);
        }

        // Fix focus if removed pane was focused
        if self.focused_pane_id.as_deref() == Some(pane_id) {
            let ids = self.active_pane_ids();
            self.focused_pane_id = ids.into_iter().next();
        }

        self.save_projects_blocking();
    }

    /// Remove a workspace by index from the active project (keeps at least one)
    pub fn remove_workspace(&mut self, idx: usize) {
        // Collect cleanup info before mutating
        let cleanup: Vec<(String, bool, Option<String>)> = match self.active_project() {
            Some(project) if project.workspaces.len() > 1 => {
                project.workspaces[idx]
                    .panes
                    .iter()
                    .map(|p| (
                        p.id.clone(),
                        matches!(p.kind, PaneKind::Browser { .. }),
                        p.pty_id.clone(),
                    ))
                    .collect()
            }
            _ => return, // keep at least one workspace
        };

        // Clean up PTYs and browsers
        {
            let mut mgr = PTY_MANAGER.lock().unwrap();
            for (pane_id, is_browser, pty_id) in &cleanup {
                if *is_browser {
                    crate::browser::BROWSER_MGR.lock().unwrap().destroy(pane_id);
                }
                if let Some(pid) = pty_id {
                    mgr.remove_pane(pid);
                    self.screen_buffers.remove(pid);
                }
            }
        }

        if let Some(project) = self.active_project_mut() {
            project.workspaces.remove(idx);
            if project.active_workspace_idx >= project.workspaces.len() {
                project.active_workspace_idx = project.workspaces.len() - 1;
            }
        }

        self.spawn_ptys_for_active_workspace();
        let ids = self.active_pane_ids();
        self.focused_pane_id = ids.into_iter().next();
        self.save_projects_blocking();
    }

    /// Remove the currently active workspace
    pub fn remove_active_workspace(&mut self) {
        if let Some(project) = self.active_project() {
            let idx = project.active_workspace_idx;
            self.remove_workspace(idx);
        }
    }

    /// Add a new workspace to the active project
    pub fn add_workspace(&mut self, name: &str) {
        if let Some(project) = self.active_project_mut() {
            let ws = Workspace {
                id: format!("ws-{}", chrono::Utc::now().timestamp_millis()),
                name: name.to_string(),
                panes: vec![Pane {
                    id: format!("pane-{}", chrono::Utc::now().timestamp_millis()),
                    kind: PaneKind::Terminal { command: None },
                    pty_id: None,
                    custom_name: None,
                }],
                cwd: project.workspaces[0].cwd.clone(),
            };
            project.workspaces.push(ws);
        }
        self.save_projects_blocking();
    }

    /// Create a new project
    pub fn create_project(&mut self, name: &str, cwd: PathBuf) -> Project {
        let ts = chrono::Utc::now().timestamp_millis();
        let project = Project {
            id: format!("proj-{}", ts),
            name: name.to_string(),
            workspaces: vec![Workspace {
                id: format!("ws-{}", ts),
                name: "main".to_string(),
                panes: vec![Pane {
                    id: format!("pane-{}", ts),
                    kind: PaneKind::Terminal { command: None },
                    pty_id: None,
                    custom_name: None,
                }],
                cwd,
            }],
            active_workspace_idx: 0,
        };

        self.projects.push(project.clone());
        self.save_projects_blocking();
        project
    }

    /// Rename a project
    pub fn rename_project(&mut self, project_id: &str, new_name: &str) {
        if let Some(project) = self.projects.iter_mut().find(|p| p.id == project_id) {
            project.name = new_name.to_string();
        }
        self.save_projects_blocking();
    }

    /// Rename a pane in the active workspace
    pub fn rename_pane(&mut self, pane_id: &str, new_name: &str) {
        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            if let Some(pane) = project.workspaces[ws_idx].panes.iter_mut().find(|p| p.id == pane_id) {
                pane.custom_name = if new_name.is_empty() { None } else { Some(new_name.to_string()) };
            }
        }
        self.save_projects_blocking();
    }

    /// Rename a workspace in the active project
    pub fn rename_workspace(&mut self, ws_idx: usize, new_name: &str) {
        if let Some(project) = self.active_project_mut() {
            if let Some(ws) = project.workspaces.get_mut(ws_idx) {
                ws.name = new_name.to_string();
            }
        }
        self.save_projects_blocking();
    }

    /// Delete a project and clean up its PTY sessions
    pub fn delete_project(&mut self, project_id: &str) {
        if let Some(project) = self.projects.iter().find(|p| p.id == project_id) {
            let mut mgr = PTY_MANAGER.lock().unwrap();
            for ws in &project.workspaces {
                for pane in &ws.panes {
                    if let Some(pty_id) = &pane.pty_id {
                        mgr.remove_pane(pty_id);
                        self.screen_buffers.remove(pty_id);
                    }
                }
            }
        }

        self.projects.retain(|p| p.id != project_id);

        if self.active_project_id.as_deref() == Some(project_id) {
            self.active_project_id = self.projects.first().map(|p| p.id.clone());
        }

        self.save_projects_blocking();
    }

    /// Drain PTY output and update screen buffers
    pub fn poll_pty_output(&mut self) -> bool {
        let mgr = PTY_MANAGER.lock().unwrap();
        let pane_ids = mgr.active_pane_ids();
        let mut any_data = false;

        for pane_id in &pane_ids {
            let data = mgr.drain_output(pane_id);
            if !data.is_empty() {
                any_data = true;
                if let Some(buf) = self.screen_buffers.get_mut(pane_id) {
                    buf.process(&data);
                }
            }
        }

        any_data
    }

    /// Switch to next workspace (wraps around)
    pub fn next_workspace(&mut self) {
        if let Some(project) = self.active_project() {
            let count = project.workspaces.len();
            let next = (project.active_workspace_idx + 1) % count;
            self.switch_workspace(next);
        }
    }

    /// Switch to previous workspace (wraps around)
    pub fn prev_workspace(&mut self) {
        if let Some(project) = self.active_project() {
            let count = project.workspaces.len();
            let prev = if project.active_workspace_idx == 0 {
                count - 1
            } else {
                project.active_workspace_idx - 1
            };
            self.switch_workspace(prev);
        }
    }

    pub fn goto_workspace(&mut self, idx: usize) {
        if let Some(project) = self.active_project() {
            if idx < project.workspaces.len() {
                self.switch_workspace(idx);
            }
        }
    }

    pub fn next_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        let current_idx = self
            .active_project_id
            .as_ref()
            .and_then(|id| self.projects.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        let next = (current_idx + 1) % self.projects.len();
        let id = self.projects[next].id.clone();
        self.switch_project_blocking(&id);
    }

    pub fn prev_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        let current_idx = self
            .active_project_id
            .as_ref()
            .and_then(|id| self.projects.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        let prev = if current_idx == 0 {
            self.projects.len() - 1
        } else {
            current_idx - 1
        };
        let id = self.projects[prev].id.clone();
        self.switch_project_blocking(&id);
    }

    fn load_projects_blocking() -> Result<Vec<Project>> {
        let db_path = Self::db_path()?;
        std::fs::create_dir_all(&db_path)?;
        let db = sled::open(&db_path)?;
        let mut projects = Vec::new();
        for item in db.scan_prefix(b"project:") {
            let (_, value) = item?;
            if let Ok(project) = serde_json::from_slice::<Project>(&value) {
                projects.push(project);
            }
        }
        Ok(projects)
    }

    /// Pull current tab URLs from the native browser manager into pane state
    /// so the persisted data reflects where the user actually navigated.
    fn sync_browser_urls(&mut self) {
        let mgr = match crate::browser::BROWSER_MGR.lock().ok() {
            Some(m) => m,
            None => return,
        };
        for project in &mut self.projects {
            for workspace in &mut project.workspaces {
                for pane in &mut workspace.panes {
                    if let PaneKind::Browser { ref mut tabs, ref mut active_tab, .. } = pane.kind {
                        if let Some((tab_info, active)) = mgr.get_tabs_info(&pane.id) {
                            let urls: Vec<String> = tab_info.into_iter().map(|t| t.url).collect();
                            if !urls.is_empty() {
                                *tabs = urls;
                                *active_tab = active;
                            }
                        }
                    }
                }
            }
        }
    }

    fn save_projects_blocking(&mut self) {
        self.sync_browser_urls();
        if let Ok(db_path) = Self::db_path() {
            if let Ok(db) = sled::open(&db_path) {
                // Remove stale project entries that are no longer in self.projects
                let current_keys: std::collections::HashSet<String> =
                    self.projects.iter().map(|p| format!("project:{}", p.id)).collect();
                for item in db.scan_prefix(b"project:") {
                    if let Ok((key, _)) = item {
                        if let Ok(key_str) = std::str::from_utf8(&key) {
                            if !current_keys.contains(key_str) {
                                let _ = db.remove(&key);
                            }
                        }
                    }
                }

                // Insert / update current projects
                for project in &self.projects {
                    let key = format!("project:{}", project.id);
                    if let Ok(value) = serde_json::to_vec(project) {
                        let _ = db.insert(key.as_bytes(), value);
                    }
                }
                let _ = db.flush();
            }
        }
    }

    fn db_path() -> Result<PathBuf> {
        let data_dir =
            dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
        Ok(data_dir.join("muxspace").join("db"))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new_blocking()
    }
}
