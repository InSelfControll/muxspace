use std::path::PathBuf;
use serde::{Deserialize, Serialize};

// ── Global Config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    pub preferred_terminal: Option<String>,
    pub workspaces: Vec<WorkspaceConfig>,
    /// Directory used for cross-device sync exports (e.g. a Dropbox/cloud path).
    pub sync_dir: Option<PathBuf>,
    /// Detected tools on this system
    #[serde(default)]
    pub detected_tools: Vec<ToolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub category: ToolCategory,
    pub path: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Terminal,
    CliEditor,
    GuiIde,
    AiAssistant,
    Browser,
}

// ── Workspace Config (YAML definition) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub name: String,
    /// Optional project grouping — workspaces sharing a project can be jumped
    /// between atomically.
    #[serde(default)]
    pub project: Option<String>,
    pub panes: Vec<PaneConfig>,
    /// Multiple tools (editors, AI assistants, IDEs) can be configured.
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    pub browser: Option<BrowserConfig>,
    /// Legacy single-editor field kept for backwards compat.
    #[serde(default)]
    pub editor: Option<LegacyEditorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneConfig {
    pub cwd: PathBuf,
    pub command: Option<String>,
}

// ── Tool configuration ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// AI coding assistant (e.g. Claude Code, Cursor AI)
    AiAssistant,
    /// CLI-based editor (nvim, helix, emacs, micro, …)
    CliEditor,
    /// GUI IDE (code, idea, zed, cursor, …)
    GuiIde,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Binary / app name, e.g. "claude", "nvim", "code", "zed"
    pub app: String,
    pub kind: ToolKind,
    /// Working directory to open the tool in
    pub path: PathBuf,
    /// Extra CLI arguments forwarded verbatim
    #[serde(default)]
    pub args: Vec<String>,
}

/// Kept for backwards-compatible workspace YAML files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyEditorConfig {
    pub app: String,
    pub path: PathBuf,
}

// ── Browser configuration ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    /// Let the OS decide (xdg-open / open)
    #[default]
    Default,
    Chrome,
    Chromium,
    Firefox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    #[serde(default)]
    pub kind: BrowserKind,
    pub urls: Vec<String>,
    /// Override the profile directory (Chrome) or profile name (Firefox).
    /// When absent the user's default profile is used.
    pub profile: Option<String>,
}

// ── Runtime Types (In-Memory State) ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_idx: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub project: Option<String>,
    pub panes: Vec<Pane>,
    pub external_pids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pane {
    pub cwd: PathBuf,
    pub command: Option<String>,
    pub pid: Option<u32>,
}

// ── Persistence Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub name: String,
    pub project: Option<String>,
    pub pane_count: usize,
}

// ── Paths ─────────────────────────────────────────────────────────────────────

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("muxspace")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

pub fn workspaces_dir() -> PathBuf {
    config_dir().join("workspaces")
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("muxspace")
}

pub fn db_path() -> PathBuf {
    data_dir().join("state.db")
}

pub fn default_sync_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".muxspace-sync")
}
