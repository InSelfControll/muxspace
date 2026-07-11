use super::{new_pane_id, AppState, Pane, PaneCleanup, PaneKind, Project, Workspace};
use crate::browser::profile::BrowserProfile;
use crate::pty::PTY_MANAGER;
use std::path::PathBuf;

impl AppState {
    pub fn add_terminal_pane(&mut self, command: Option<String>) {
        let pane_id = new_pane_id();
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

    pub fn add_browser_pane(&mut self, url: String, profile: Option<BrowserProfile>) {
        let pane_id = new_pane_id();
        let pane = Pane {
            id: pane_id.clone(),
            kind: PaneKind::Browser {
                url: String::new(),
                tabs: vec![url],
                active_tab: 0,
                profile,
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

    pub fn add_editor_pane(&mut self, editor_name: String, executable: String, root_dir: PathBuf) {
        let pane_id = new_pane_id();
        let pane = Pane {
            id: pane_id.clone(),
            kind: PaneKind::Editor {
                name: editor_name,
                executable,
                root_dir: Some(root_dir),
                active_file: None,
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

    pub fn editor_root_dir(&self, pane_id: &str) -> Option<PathBuf> {
        self.active_workspace().and_then(|workspace| {
            workspace
                .panes
                .iter()
                .find(|pane| pane.id == pane_id)
                .and_then(|pane| match &pane.kind {
                    PaneKind::Editor { root_dir, .. } => root_dir.clone(),
                    _ => None,
                })
        })
    }

    pub fn set_editor_active_file(&mut self, pane_id: &str, file: Option<PathBuf>) {
        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            if let Some(pane) = project.workspaces[ws_idx]
                .panes
                .iter_mut()
                .find(|p| p.id == pane_id)
            {
                if let PaneKind::Editor { active_file, .. } = &mut pane.kind {
                    *active_file = file;
                }
            }
        }
        self.save_projects_blocking();
    }

    pub fn remove_pane(&mut self, pane_id: &str) {
        let cleanup = self
            .active_workspace()
            .and_then(|ws| ws.panes.iter().find(|p| p.id == pane_id))
            .map(PaneCleanup::from);
        if let Some(cleanup) = cleanup {
            self.cleanup_pane(&cleanup);
        }
        if let Some(project) = self.active_project_mut() {
            let ws_idx = project.active_workspace_idx;
            project.workspaces[ws_idx].panes.retain(|p| p.id != pane_id);
        }
        if self.focused_pane_id.as_deref() == Some(pane_id) {
            self.focused_pane_id = self.active_pane_ids().into_iter().next();
        }
        self.save_projects_blocking();
    }

    pub fn remove_workspace(&mut self, idx: usize) {
        let cleanup = match self.active_project() {
            Some(project) if project.workspaces.len() > 1 && idx < project.workspaces.len() => {
                project.workspaces[idx]
                    .panes
                    .iter()
                    .map(PaneCleanup::from)
                    .collect::<Vec<_>>()
            }
            _ => return,
        };
        for pane in &cleanup {
            self.cleanup_pane(pane);
        }
        if let Some(project) = self.active_project_mut() {
            project.workspaces.remove(idx);
            if project.active_workspace_idx >= project.workspaces.len() {
                project.active_workspace_idx = project.workspaces.len().saturating_sub(1);
            }
        }
        self.spawn_ptys_for_active_workspace();
        self.focused_pane_id = self.active_pane_ids().into_iter().next();
        self.save_projects_blocking();
    }

    pub fn remove_active_workspace(&mut self) {
        if let Some(project) = self.active_project() {
            self.remove_workspace(project.active_workspace_idx);
        }
    }

    pub fn add_workspace(&mut self, name: &str) {
        if let Some(project) = self.active_project_mut() {
            let cwd = project
                .workspaces
                .first()
                .map(|ws| ws.cwd.clone())
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")));
            project.workspaces.push(Workspace {
                id: format!("ws-{}", chrono::Utc::now().timestamp_millis()),
                name: name.to_string(),
                panes: vec![Pane {
                    id: new_pane_id(),
                    kind: PaneKind::Terminal { command: None },
                    pty_id: None,
                    custom_name: None,
                }],
                cwd,
            });
        }
        self.save_projects_blocking();
    }

    pub fn create_project(&mut self, name: &str, cwd: PathBuf) -> Project {
        let ts = chrono::Utc::now().timestamp_millis();
        let project = Project {
            id: format!("proj-{ts}"),
            name: name.to_string(),
            workspaces: vec![Workspace {
                id: format!("ws-{ts}"),
                name: "main".to_string(),
                panes: vec![Pane {
                    id: new_pane_id(),
                    kind: PaneKind::Terminal { command: None },
                    pty_id: None,
                    custom_name: None,
                }],
                cwd,
            }],
            active_workspace_idx: 0,
        };
        self.projects.push(project.clone());
        self.active_project_id = Some(project.id.clone());
        self.focused_pane_id = project.workspaces[0]
            .panes
            .first()
            .map(|pane| pane.id.clone());
        self.save_projects_blocking();
        project
    }

    pub fn delete_project(&mut self, project_id: &str) {
        let cleanup = self
            .projects
            .iter()
            .find(|p| p.id == project_id)
            .map(|project| {
                project
                    .workspaces
                    .iter()
                    .flat_map(|ws| ws.panes.iter().map(PaneCleanup::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for pane in &cleanup {
            self.cleanup_pane(pane);
        }
        self.projects.retain(|p| p.id != project_id);
        if self.active_project_id.as_deref() == Some(project_id) {
            self.active_project_id = self.projects.first().map(|p| p.id.clone());
        }
        self.save_projects_blocking();
    }

    fn cleanup_pane(&mut self, cleanup: &PaneCleanup) {
        if cleanup.is_browser {
            crate::browser::with_manager_mut(|mgr| {
                mgr.destroy(&cleanup.pane_id);
            });
        }
        if let Some(pty_id) = &cleanup.pty_id {
            if let Ok(mut mgr) = PTY_MANAGER.lock() {
                mgr.remove_pane(pty_id);
            }
            self.screen_buffers.remove(pty_id);
        }
    }

    pub(crate) fn apply_pty_updates(&mut self, updates: Vec<(String, String)>) {
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
    }
}
