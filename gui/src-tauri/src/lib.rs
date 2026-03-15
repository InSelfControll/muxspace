use muxspace::{MuxspaceApp, WorkspaceConfig, detect_all_tools, types::{workspaces_dir, ToolCategory}};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

// Global app state
pub struct AppState {
    app: Mutex<MuxspaceApp>,
}

// Data transfer objects
#[derive(Serialize, Clone)]
struct ProjectDto {
    name: String,
    workspaces: Vec<String>,
}

#[derive(Serialize, Clone)]
struct WorkspaceDto {
    name: String,
    project: Option<String>,
    pane_count: usize,
}

#[derive(Serialize, Clone)]
struct ToolDto {
    name: String,
    category: String,
    path: String,
}

#[derive(Deserialize)]
struct CreateWorkspaceRequest {
    name: String,
    project: Option<String>,
    cwd: String,
    command: Option<String>,
}

// Commands

#[tauri::command]
fn get_projects(state: State<AppState>) -> Result<Vec<ProjectDto>, String> {
    let app = state.app.lock().map_err(|e| e.to_string())?;
    let projects = app.list_projects();
    
    Ok(projects.into_iter().map(|(name, workspaces)| {
        ProjectDto {
            name: name.unwrap_or_else(|| "default".to_string()),
            workspaces,
        }
    }).collect())
}

#[tauri::command]
fn get_workspaces(state: State<AppState>) -> Result<Vec<WorkspaceDto>, String> {
    let app = state.app.lock().map_err(|e| e.to_string())?;
    let workspaces = app.list_workspaces();
    
    Ok(workspaces.into_iter().map(|ws| {
        WorkspaceDto {
            name: ws.name,
            project: ws.project,
            pane_count: ws.pane_count,
        }
    }).collect())
}

#[tauri::command]
fn get_workspace_config(name: String, state: State<AppState>) -> Result<Option<WorkspaceConfig>, String> {
    let app = state.app.lock().map_err(|e| e.to_string())?;
    Ok(app.get_workspace(&name).cloned())
}

#[tauri::command]
fn start_workspace(name: String, state: State<AppState>) -> Result<Vec<u32>, String> {
    let mut app = state.app.lock().map_err(|e| e.to_string())?;
    let config = app.get_workspace(&name)
        .ok_or_else(|| "Workspace not found".to_string())?
        .clone();
    
    app.start_workspace(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_detected_tools() -> Result<Vec<ToolDto>, String> {
    let tools = detect_all_tools();
    Ok(tools.into_iter().map(|t| ToolDto {
        name: t.name,
        category: format!("{:?}", t.category).to_lowercase().replace("toolcategory::", ""),
        path: t.path.to_string_lossy().to_string(),
    }).collect())
}

#[tauri::command]
fn export_workspaces(state: State<AppState>) -> Result<String, String> {
    let app = state.app.lock().map_err(|e| e.to_string())?;
    app.export_workspaces().map_err(|e| e.to_string())?;
    let sync_dir = muxspace::types::default_sync_dir();
    Ok(sync_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn import_workspaces(path: Option<String>, state: State<AppState>) -> Result<usize, String> {
    let mut app = state.app.lock().map_err(|e| e.to_string())?;
    let path = path.map(PathBuf::from);
    let count = app.import_workspaces(path).map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
fn get_active_projects(state: State<AppState>) -> Result<Vec<String>, String> {
    let app = state.app.lock().map_err(|e| e.to_string())?;
    app.get_active_projects().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_active_project(name: String, state: State<AppState>) -> Result<(), String> {
    let app = state.app.lock().map_err(|e| e.to_string())?;
    app.add_active_project(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_workspaces_dir() -> String {
    workspaces_dir().to_string_lossy().to_string()
}

#[tauri::command]
fn load_workspace_from_path(path: String) -> Result<WorkspaceConfig, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let cfg: WorkspaceConfig = serde_yaml::from_str(&content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;
    Ok(cfg)
}

#[tauri::command]
fn save_workspace_config(cfg: WorkspaceConfig) -> Result<String, String> {
    let ws_dir = workspaces_dir();
    std::fs::create_dir_all(&ws_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    
    let path = ws_dir.join(format!("{}.yaml", cfg.name));
    let yaml = serde_yaml::to_string(&cfg)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    
    std::fs::write(&path, yaml)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn scan_for_workspaces() -> Result<Vec<WorkspaceConfig>, String> {
    let ws_dir = workspaces_dir();
    let mut configs = Vec::new();
    
    if !ws_dir.exists() {
        return Ok(configs);
    }
    
    for entry in std::fs::read_dir(&ws_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_yaml::from_str::<WorkspaceConfig>(&content) {
                    configs.push(cfg);
                }
            }
        }
    }
    
    Ok(configs)
}

pub fn run() {
    let app = MuxspaceApp::new().expect("Failed to initialize Muxspace");
    
    let state = AppState {
        app: Mutex::new(app),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_projects,
            get_workspaces,
            get_workspace_config,
            start_workspace,
            get_detected_tools,
            export_workspaces,
            import_workspaces,
            get_active_projects,
            add_active_project,
            get_workspaces_dir,
            load_workspace_from_path,
            save_workspace_config,
            scan_for_workspaces,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
