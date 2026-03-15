use crate::state::Project;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// High-performance embedded database using sled
pub struct SyncManager {
    db: sled::Db,
}

#[derive(Serialize, Deserialize)]
struct ExportData {
    version: u32,
    projects: Vec<Project>,
    active_projects: Vec<String>,
}

impl SyncManager {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("creating data directory")?;
        }
        
        let db = sled::open(&db_path)
            .with_context(|| format!("opening sled database at {}", db_path.display()))?;
        
        Ok(Self { db })
    }
    
    /// Save a project
    pub fn save_project(&self, project: &Project) -> Result<()> {
        let key = format!("project:{}", project.id);
        let value = serde_json::to_vec(project)?;
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }
    
    /// Load a project by ID
    #[allow(dead_code)]
    pub fn load_project(&self, id: &str) -> Result<Option<Project>> {
        let key = format!("project:{}", id);
        match self.db.get(key.as_bytes())? {
            Some(data) => {
                let project: Project = serde_json::from_slice(&data)?;
                Ok(Some(project))
            }
            None => Ok(None),
        }
    }
    
    /// Load all projects
    pub fn load_projects(&self) -> Result<Vec<Project>> {
        let mut projects = Vec::new();
        
        for item in self.db.scan_prefix(b"project:") {
            let (_, value) = item?;
            if let Ok(project) = serde_json::from_slice::<Project>(&value) {
                projects.push(project);
            }
        }
        
        Ok(projects)
    }
    
    /// Delete a project
    #[allow(dead_code)]
    pub fn delete_project(&self, id: &str) -> Result<()> {
        let key = format!("project:{}", id);
        self.db.remove(key.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }
    
    /// Save active project list
    pub fn save_active_projects(&self, project_ids: &[String]) -> Result<()> {
        let value = serde_json::to_vec(project_ids)?;
        self.db.insert(b"meta:active_projects", value)?;
        self.db.flush()?;
        Ok(())
    }
    
    /// Load active project list
    pub fn load_active_projects(&self) -> Result<Vec<String>> {
        match self.db.get(b"meta:active_projects")? {
            Some(data) => {
                let ids: Vec<String> = serde_json::from_slice(&data)?;
                Ok(ids)
            }
            None => Ok(Vec::new()),
        }
    }
    
    /// Export all data for sync
    pub fn export(&self) -> Result<Vec<u8>> {
        let export = ExportData {
            version: 1,
            projects: self.load_projects()?,
            active_projects: self.load_active_projects()?,
        };
        
        Ok(serde_json::to_vec(&export)?)
    }
    
    /// Import data from sync
    pub fn import(&self, data: &[u8]) -> Result<()> {
        let import: ExportData = serde_json::from_slice(data)?;
        
        for project in import.projects {
            self.save_project(&project)?;
        }
        
        self.save_active_projects(&import.active_projects)?;
        
        Ok(())
    }
    
    fn db_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"));
        Ok(data_dir.join("muxspace").join("db"))
    }
}

/// Cross-device sync engine
#[allow(dead_code)]
pub struct SyncEngine;

#[allow(dead_code)]
impl SyncEngine {
    pub fn new() -> Self {
        Self
    }
    
    pub fn sync_to_git(&self, _repo_path: &PathBuf) -> Result<()> {
        Ok(())
    }
    
    pub fn sync_from_git(&self, _repo_path: &PathBuf) -> Result<()> {
        Ok(())
    }
}
