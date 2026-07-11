use super::{AppState, PaneKind};
use anyhow::Result;
use std::path::PathBuf;

impl AppState {
    pub(crate) fn load_projects_blocking() -> Result<Vec<super::Project>> {
        let db_path = Self::db_path()?;
        std::fs::create_dir_all(&db_path)?;
        let db = sled::open(&db_path)?;
        let mut projects = Vec::new();
        for item in db.scan_prefix(b"project:") {
            let (_, value) = item?;
            if let Ok(project) = serde_json::from_slice::<super::Project>(&value) {
                projects.push(project);
            }
        }
        Ok(projects)
    }

    pub(crate) fn save_projects_blocking(&mut self) {
        self.sync_browser_urls();
        if let Err(err) = self.try_save_projects_blocking() {
            tracing::error!("Failed to save projects: {err:#}");
        }
    }

    fn try_save_projects_blocking(&self) -> Result<()> {
        let db_path = Self::db_path()?;
        let db = sled::open(&db_path)?;
        let current_keys = self
            .projects
            .iter()
            .map(|p| format!("project:{}", p.id))
            .collect::<std::collections::HashSet<_>>();

        for item in db.scan_prefix(b"project:") {
            let (key, _) = item?;
            if let Ok(key_str) = std::str::from_utf8(&key) {
                if !current_keys.contains(key_str) {
                    db.remove(&key)?;
                }
            }
        }
        for project in &self.projects {
            let key = format!("project:{}", project.id);
            db.insert(key.as_bytes(), serde_json::to_vec(project)?)?;
        }
        db.flush()?;
        Ok(())
    }

    fn sync_browser_urls(&mut self) {
        crate::browser::with_manager(|mgr| {
            for project in &mut self.projects {
                for workspace in &mut project.workspaces {
                    for pane in &mut workspace.panes {
                        if let PaneKind::Browser {
                            tabs, active_tab, ..
                        } = &mut pane.kind
                        {
                            if let Some((tab_info, active)) = mgr.get_tabs_info(&pane.id) {
                                let urls =
                                    tab_info.into_iter().map(|tab| tab.url).collect::<Vec<_>>();
                                if !urls.is_empty() {
                                    *tabs = urls;
                                    *active_tab = active;
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    fn db_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
        Ok(data_dir.join("muxspace").join("db"))
    }
}
