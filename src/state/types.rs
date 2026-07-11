use crate::browser::profile::BrowserProfile;
use crate::pty::ScreenBuffer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppState {
    pub projects: Vec<Project>,
    pub active_project_id: Option<String>,
    pub screen_buffers: HashMap<String, ScreenBuffer>,
    pub show_create_project: bool,
    pub show_hotkey_editor: bool,
    pub show_shortcuts_help: bool,
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
    pub pty_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaneKind {
    Terminal {
        command: Option<String>,
    },
    Browser {
        #[serde(default, skip_serializing)]
        url: String,
        #[serde(default)]
        tabs: Vec<String>,
        #[serde(default)]
        active_tab: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<BrowserProfile>,
    },
    Editor {
        executable: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        root_dir: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active_file: Option<PathBuf>,
    },
}
