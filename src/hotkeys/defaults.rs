use super::{Action, HotkeyConfig, Keybind, PrefixKey};
use std::collections::HashMap;

impl Default for HotkeyConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(
            Action::FocusNextPane,
            vec![pfx("ArrowRight"), pfx("ArrowDown")],
        );
        bindings.insert(
            Action::FocusPrevPane,
            vec![pfx("ArrowLeft"), pfx("ArrowUp")],
        );
        bindings.insert(Action::NextWorkspace, vec![pfx("n"), ctrl("Tab", false)]);
        bindings.insert(Action::PrevWorkspace, vec![pfx("p"), ctrl("Tab", true)]);
        bindings.insert(Action::NewWorkspace, vec![pfx("c"), ctrl("t", true)]);
        bindings.insert(Action::NewTerminalPane, vec![pfx("|"), pfx("%")]);
        bindings.insert(Action::ClosePane, vec![pfx("x")]);
        bindings.insert(Action::NextProject, vec![pfx_shift("n")]);
        bindings.insert(Action::PrevProject, vec![pfx_shift("p")]);

        for (idx, action) in workspace_actions().into_iter().enumerate() {
            let key = if idx == 9 {
                "0".to_string()
            } else {
                (idx + 1).to_string()
            };
            bindings.insert(action, vec![pfx(&key), alt(&key)]);
        }

        Self {
            prefix_key: PrefixKey::default(),
            bindings,
        }
    }
}

fn workspace_actions() -> [Action; 10] {
    [
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
    ]
}

fn pfx(key: &str) -> Keybind {
    bind(true, false, false, false, key)
}

fn pfx_shift(key: &str) -> Keybind {
    bind(true, false, false, true, key)
}

fn ctrl(key: &str, shift: bool) -> Keybind {
    bind(false, true, false, shift, key)
}

fn alt(key: &str) -> Keybind {
    bind(false, false, true, false, key)
}

fn bind(prefix: bool, ctrl: bool, alt: bool, shift: bool, key: &str) -> Keybind {
    Keybind {
        prefix,
        ctrl,
        alt,
        shift,
        key: key.to_string(),
    }
}
