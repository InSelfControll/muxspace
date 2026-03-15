use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// All actions that can be bound to hotkeys
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
    /// All actions in display order.
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
        match self {
            Action::FocusNextPane => write!(f, "Focus Next Pane"),
            Action::FocusPrevPane => write!(f, "Focus Previous Pane"),
            Action::NextWorkspace => write!(f, "Next Workspace"),
            Action::PrevWorkspace => write!(f, "Previous Workspace"),
            Action::NewWorkspace => write!(f, "New Workspace"),
            Action::NewTerminalPane => write!(f, "New Terminal Pane"),
            Action::ClosePane => write!(f, "Close Pane"),
            Action::NextProject => write!(f, "Next Project"),
            Action::PrevProject => write!(f, "Previous Project"),
            Action::GotoWorkspace0 => write!(f, "Go to Workspace 1"),
            Action::GotoWorkspace1 => write!(f, "Go to Workspace 2"),
            Action::GotoWorkspace2 => write!(f, "Go to Workspace 3"),
            Action::GotoWorkspace3 => write!(f, "Go to Workspace 4"),
            Action::GotoWorkspace4 => write!(f, "Go to Workspace 5"),
            Action::GotoWorkspace5 => write!(f, "Go to Workspace 6"),
            Action::GotoWorkspace6 => write!(f, "Go to Workspace 7"),
            Action::GotoWorkspace7 => write!(f, "Go to Workspace 8"),
            Action::GotoWorkspace8 => write!(f, "Go to Workspace 9"),
            Action::GotoWorkspace9 => write!(f, "Go to Workspace 10"),
        }
    }
}

/// A key combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Keybind {
    /// Whether a prefix key (like Ctrl+B) must be pressed first
    pub prefix: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// The key character or name (e.g. "n", "Tab", "ArrowRight")
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

/// Hotkey configuration: maps keybinds to actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// The prefix key combo (default: Ctrl+B like tmux)
    pub prefix_key: PrefixKey,
    /// Map of action -> keybinds (multiple binds per action allowed)
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

impl Default for HotkeyConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Prefix mode shortcuts (tmux-style: Ctrl+B then key)
        let pfx = |key: &str| Keybind {
            prefix: true,
            ctrl: false,
            alt: false,
            shift: false,
            key: key.to_string(),
        };
        let pfx_shift = |key: &str| Keybind {
            prefix: true,
            ctrl: false,
            alt: false,
            shift: true,
            key: key.to_string(),
        };

        bindings.insert(Action::FocusNextPane, vec![
            pfx("ArrowRight"),
            pfx("ArrowDown"),
        ]);
        bindings.insert(Action::FocusPrevPane, vec![
            pfx("ArrowLeft"),
            pfx("ArrowUp"),
        ]);
        bindings.insert(Action::NextWorkspace, vec![
            pfx("n"),
            Keybind { prefix: false, ctrl: true, alt: false, shift: false, key: "Tab".to_string() },
        ]);
        bindings.insert(Action::PrevWorkspace, vec![
            pfx("p"),
            Keybind { prefix: false, ctrl: true, alt: false, shift: true, key: "Tab".to_string() },
        ]);
        bindings.insert(Action::NewWorkspace, vec![
            pfx("c"),
            Keybind { prefix: false, ctrl: true, alt: false, shift: true, key: "t".to_string() },
        ]);
        bindings.insert(Action::NewTerminalPane, vec![pfx("|"), pfx("%")]);
        bindings.insert(Action::ClosePane, vec![pfx("x")]);
        bindings.insert(Action::NextProject, vec![pfx_shift("n")]);
        bindings.insert(Action::PrevProject, vec![pfx_shift("p")]);

        // Alt+1..9 for workspace switching
        bindings.insert(Action::GotoWorkspace0, vec![
            pfx("1"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "1".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace1, vec![
            pfx("2"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "2".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace2, vec![
            pfx("3"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "3".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace3, vec![
            pfx("4"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "4".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace4, vec![
            pfx("5"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "5".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace5, vec![
            pfx("6"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "6".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace6, vec![
            pfx("7"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "7".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace7, vec![
            pfx("8"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "8".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace8, vec![
            pfx("9"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "9".to_string() },
        ]);
        bindings.insert(Action::GotoWorkspace9, vec![
            pfx("0"),
            Keybind { prefix: false, ctrl: false, alt: true, shift: false, key: "0".to_string() },
        ]);

        Self {
            prefix_key: PrefixKey::default(),
            bindings,
        }
    }
}

impl HotkeyConfig {
    /// Load config from disk, or return defaults
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

    /// Save config to disk
    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Check if a key event with prefix state matches any action
    pub fn match_action(
        &self,
        prefix_active: bool,
        ctrl: bool,
        alt: bool,
        shift: bool,
        key: &str,
    ) -> Option<Action> {
        for (action, binds) in &self.bindings {
            for bind in binds {
                if bind.prefix == prefix_active
                    && bind.ctrl == ctrl
                    && bind.alt == alt
                    && bind.shift == shift
                    && bind.key == key
                {
                    return Some(*action);
                }
            }
        }
        None
    }

    /// Check if a key event matches the prefix key
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
