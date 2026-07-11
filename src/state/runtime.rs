use super::{AppState, PaneKind, Project, Workspace};
use crate::pty::{ScreenBuffer, PTY_MANAGER};
use std::collections::HashMap;
use std::path::PathBuf;

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

    fn migrate_browser_tabs(projects: &mut [Project]) {
        for project in projects {
            for workspace in &mut project.workspaces {
                for pane in &mut workspace.panes {
                    if let PaneKind::Browser { url, tabs, .. } = &mut pane.kind {
                        if tabs.is_empty() && !url.is_empty() {
                            tabs.push(url.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn empty_for_tests() -> Self {
        Self {
            projects: Vec::new(),
            active_project_id: None,
            screen_buffers: HashMap::new(),
            show_create_project: false,
            show_hotkey_editor: false,
            show_shortcuts_help: false,
            focused_pane_id: None,
        }
    }

    pub fn active_project(&self) -> Option<&Project> {
        self.active_project_id
            .as_ref()
            .and_then(|id| self.projects.iter().find(|p| p.id == *id))
    }

    pub fn active_workspace(&self) -> Option<&Workspace> {
        self.active_project()
            .and_then(|project| project.workspaces.get(project.active_workspace_idx))
    }

    pub fn active_workspace_cwd(&self) -> PathBuf {
        self.active_workspace()
            .map(|workspace| workspace.cwd.clone())
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")))
    }

    pub fn active_project_mut(&mut self) -> Option<&mut Project> {
        let id = self.active_project_id.clone();
        id.and_then(move |id| self.projects.iter_mut().find(|p| p.id == id))
    }

    pub fn active_pane_ids(&self) -> Vec<String> {
        self.active_workspace()
            .map(|ws| ws.panes.iter().map(|pane| pane.id.clone()).collect())
            .unwrap_or_default()
    }

    pub fn focus_next_pane(&mut self) {
        let ids = self.active_pane_ids();
        if ids.is_empty() {
            return;
        }
        let current = self
            .focused_pane_id
            .as_ref()
            .and_then(|id| ids.iter().position(|i| i == id))
            .unwrap_or(0);
        self.focused_pane_id = Some(ids[(current + 1) % ids.len()].clone());
    }

    pub fn focus_prev_pane(&mut self) {
        let ids = self.active_pane_ids();
        if ids.is_empty() {
            return;
        }
        let current = self
            .focused_pane_id
            .as_ref()
            .and_then(|id| ids.iter().position(|i| i == id))
            .unwrap_or(0);
        let prev = if current == 0 {
            ids.len() - 1
        } else {
            current - 1
        };
        self.focused_pane_id = Some(ids[prev].clone());
    }

    pub fn switch_project_blocking(&mut self, project_id: &str) {
        self.active_project_id = Some(project_id.to_string());
        self.spawn_ptys_for_active_workspace();
        self.focused_pane_id = self.active_pane_ids().into_iter().next();
    }

    pub fn spawn_ptys_for_active_workspace(&mut self) {
        let Some(project) = self.active_project().cloned() else {
            return;
        };
        let Some(workspace) = project.workspaces.get(project.active_workspace_idx) else {
            return;
        };
        let mut mgr = match PTY_MANAGER.lock() {
            Ok(mgr) => mgr,
            Err(err) => {
                tracing::error!("Failed to lock PTY_MANAGER: {err}");
                return;
            }
        };
        let mut updates = Vec::new();
        for pane in &workspace.panes {
            let command = match &pane.kind {
                PaneKind::Terminal { command } => command.as_deref(),
                _ => continue,
            };
            let pty_id = pane.pty_id.clone().unwrap_or_else(|| pane.id.clone());
            if mgr.spawn_for_pane(&pty_id, &workspace.cwd, command).is_ok() {
                tracing::info!("PTY spawned for pane {pty_id}");
            }
            self.screen_buffers
                .entry(pty_id.clone())
                .or_insert_with(|| ScreenBuffer::new(24, 80));
            updates.push((pane.id.clone(), pty_id));
        }
        drop(mgr);
        self.apply_pty_updates(updates);
        self.save_projects_blocking();
    }

    pub fn rename_project(&mut self, project_id: &str, new_name: &str) {
        if let Some(project) = self.projects.iter_mut().find(|p| p.id == project_id) {
            project.name = new_name.to_string();
        }
        self.save_projects_blocking();
    }

    pub fn rename_pane(&mut self, pane_id: &str, new_name: &str) {
        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            if let Some(pane) = project.workspaces[ws_idx]
                .panes
                .iter_mut()
                .find(|p| p.id == pane_id)
            {
                pane.custom_name = if new_name.is_empty() {
                    None
                } else {
                    Some(new_name.to_string())
                };
            }
        }
        self.save_projects_blocking();
    }

    pub fn rename_workspace(&mut self, ws_idx: usize, new_name: &str) {
        if let Some(project) = self.active_project_mut() {
            if let Some(ws) = project.workspaces.get_mut(ws_idx) {
                ws.name = new_name.to_string();
            }
        }
        self.save_projects_blocking();
    }

    pub fn poll_pty_output(&mut self) -> bool {
        let mgr = match PTY_MANAGER.lock() {
            Ok(mgr) => mgr,
            Err(err) => {
                tracing::error!("Failed to lock PTY_MANAGER: {err}");
                return false;
            }
        };
        let pane_ids = mgr.active_pane_ids();
        let mut any_data = false;
        for pane_id in &pane_ids {
            let data = mgr.drain_output(pane_id);
            if data.is_empty() {
                continue;
            }
            any_data = true;
            if let Some(buf) = self.screen_buffers.get_mut(pane_id) {
                buf.process(&data);
            }
        }
        any_data
    }
}
