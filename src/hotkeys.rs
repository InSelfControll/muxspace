mod defaults;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    FocusNextPane,
    FocusPrevPane,
    NextWorkspace,
    PrevWorkspace,
    GotoWorkspace0,
    GotoWorkspace1,
    GotoWorkspace2,
    GotoWorkspace3,
    GotoWorkspace4,
    GotoWorkspace5,
    GotoWorkspace6,
    GotoWorkspace7,
    GotoWorkspace8,
    GotoWorkspace9,
    NewWorkspace,
    NewTerminalPane,
    ClosePane,
    NextProject,
    PrevProject,
}

impl Action {
    pub const ALL: &'static [Action] = &[
        Action::FocusNextPane,
        Action::FocusPrevPane,
        Action::NextWorkspace,
        Action::PrevWorkspace,
        Action::NewWorkspace,
        Action::NewTerminalPane,
        Action::ClosePane,
        Action::NextProject,
        Action::PrevProject,
        Action::GotoWorkspace0,
        Action::GotoWorkspace1,
        Action::GotoWorkspace2,
        Action::GotoWorkspace3,
        Action::GotoWorkspace4,
        Action::GotoWorkspace5,
        Action::GotoWorkspace6,
        Action::GotoWorkspace7,
        Action::GotoWorkspace8,
        Action::GotoWorkspace9,
    ];
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Action::FocusNextPane => "Focus Next Pane",
            Action::FocusPrevPane => "Focus Previous Pane",
            Action::NextWorkspace => "Next Workspace",
            Action::PrevWorkspace => "Previous Workspace",
            Action::NewWorkspace => "New Workspace",
            Action::NewTerminalPane => "New Terminal Pane",
            Action::ClosePane => "Close Pane",
            Action::NextProject => "Next Project",
            Action::PrevProject => "Previous Project",
            Action::GotoWorkspace0 => "Go to Workspace 1",
            Action::GotoWorkspace1 => "Go to Workspace 2",
            Action::GotoWorkspace2 => "Go to Workspace 3",
            Action::GotoWorkspace3 => "Go to Workspace 4",
            Action::GotoWorkspace4 => "Go to Workspace 5",
            Action::GotoWorkspace5 => "Go to Workspace 6",
            Action::GotoWorkspace6 => "Go to Workspace 7",
            Action::GotoWorkspace7 => "Go to Workspace 8",
            Action::GotoWorkspace8 => "Go to Workspace 9",
            Action::GotoWorkspace9 => "Go to Workspace 10",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Keybind {
    pub prefix: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: String,
}

impl std::fmt::Display for Keybind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.prefix {
            parts.push("Prefix".to_string());
        }
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        parts.push(self.key.clone());
        write!(f, "{}", parts.join("+"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub prefix_key: PrefixKey,
    pub bindings: HashMap<Action, Vec<Keybind>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixKey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: String,
}

impl Default for PrefixKey {
    fn default() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
            key: "b".to_string(),
        }
    }
}

impl HotkeyConfig {
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&data) {
                return config;
            }
        }
        let config = Self::default();
        config.save();
        config
    }

    pub fn save(&self) {
        if let Err(err) = self.try_save() {
            tracing::error!("Failed to save hotkey config: {err}");
        }
    }

    fn try_save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn match_action(
        &self,
        prefix_active: bool,
        ctrl: bool,
        alt: bool,
        shift: bool,
        key: &str,
    ) -> Option<Action> {
        self.bindings.iter().find_map(|(action, binds)| {
            binds
                .iter()
                .any(|bind| {
                    bind.prefix == prefix_active
                        && bind.ctrl == ctrl
                        && bind.alt == alt
                        && bind.shift == shift
                        && bind.key == key
                })
                .then_some(*action)
        })
    }

    pub fn is_prefix_key(&self, ctrl: bool, alt: bool, shift: bool, key: &str) -> bool {
        self.prefix_key.ctrl == ctrl
            && self.prefix_key.alt == alt
            && self.prefix_key.shift == shift
            && self.prefix_key.key == key
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("muxspace")
            .join("hotkeys.json")
    }
}
