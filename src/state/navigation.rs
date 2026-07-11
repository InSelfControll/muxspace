use super::AppState;

impl AppState {
    pub fn switch_workspace(&mut self, idx: usize) {
        crate::browser::with_manager(|mgr| {
            mgr.hide_all();
        });
        if let Some(project) = self.active_project_mut() {
            if idx < project.workspaces.len() {
                project.active_workspace_idx = idx;
            }
        }
        self.spawn_ptys_for_active_workspace();
        self.focused_pane_id = self.active_pane_ids().into_iter().next();
    }

    pub fn next_workspace(&mut self) {
        if let Some(project) = self.active_project() {
            let count = project.workspaces.len();
            if count > 0 {
                self.switch_workspace((project.active_workspace_idx + 1) % count);
            }
        }
    }

    pub fn prev_workspace(&mut self) {
        if let Some(project) = self.active_project() {
            let count = project.workspaces.len();
            if count == 0 {
                return;
            }
            let prev = if project.active_workspace_idx == 0 {
                count - 1
            } else {
                project.active_workspace_idx - 1
            };
            self.switch_workspace(prev);
        }
    }

    pub fn goto_workspace(&mut self, idx: usize) {
        if self
            .active_project()
            .is_some_and(|project| idx < project.workspaces.len())
        {
            self.switch_workspace(idx);
        }
    }

    pub fn next_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        let current = self
            .active_project_id
            .as_ref()
            .and_then(|id| self.projects.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        let id = self.projects[(current + 1) % self.projects.len()]
            .id
            .clone();
        self.switch_project_blocking(&id);
    }

    pub fn prev_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        let current = self
            .active_project_id
            .as_ref()
            .and_then(|id| self.projects.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        let prev = if current == 0 {
            self.projects.len() - 1
        } else {
            current - 1
        };
        let id = self.projects[prev].id.clone();
        self.switch_project_blocking(&id);
    }
}
