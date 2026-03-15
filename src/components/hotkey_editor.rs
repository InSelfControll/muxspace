use dioxus::prelude::*;

use crate::hotkeys::{Action, HotkeyConfig, Keybind};
use crate::state::AppState;

/// Shortcut categories for grouped display
const GROUPS: &[(&str, &[Action])] = &[
    ("Pane Navigation", &[
        Action::FocusNextPane,
        Action::FocusPrevPane,
    ]),
    ("Panes", &[
        Action::NewTerminalPane,
        Action::ClosePane,
    ]),
    ("Workspaces", &[
        Action::NextWorkspace,
        Action::PrevWorkspace,
        Action::NewWorkspace,
    ]),
    ("Quick Switch", &[
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
    ]),
    ("Projects", &[
        Action::NextProject,
        Action::PrevProject,
    ]),
];

/// Modal for viewing and editing keyboard shortcuts.
///
/// The editor works on a **clone** of the live config.  Changes are only
/// applied (and persisted to disk) when the user clicks "Save".
#[component]
pub fn HotkeyEditorModal() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut live_config = use_context::<Signal<HotkeyConfig>>();

    // Working copy — edits happen here until Save
    let mut draft = use_signal(|| live_config.read().clone());

    // Which binding is being recorded: (global_action_index, binding_index)
    let mut recording: Signal<Option<(usize, usize)>> = use_signal(|| None);

    let on_close = move |_: MouseEvent| {
        state.write().show_hotkey_editor = false;
    };

    let on_save = move |_: MouseEvent| {
        let cfg = draft.read().clone();
        cfg.save();
        *live_config.write() = cfg;
        state.write().show_hotkey_editor = false;
    };

    let on_reset = move |_: MouseEvent| {
        draft.set(HotkeyConfig::default());
    };

    // Key capture when recording
    let on_key = move |evt: KeyboardEvent| {
        let rec = *recording.read();
        let (action_idx, bind_idx) = match rec {
            Some(v) => v,
            None => return,
        };

        let ctrl = evt.modifiers().contains(Modifiers::CONTROL);
        let alt = evt.modifiers().contains(Modifiers::ALT);
        let shift = evt.modifiers().contains(Modifiers::SHIFT);

        let key_str = match evt.key() {
            Key::Character(ref s) => s.clone(),
            Key::ArrowUp => "ArrowUp".into(),
            Key::ArrowDown => "ArrowDown".into(),
            Key::ArrowLeft => "ArrowLeft".into(),
            Key::ArrowRight => "ArrowRight".into(),
            Key::Tab => "Tab".into(),
            Key::Enter => "Enter".into(),
            Key::Escape => {
                recording.set(None);
                return;
            }
            _ => return,
        };

        let action = Action::ALL[action_idx];
        let mut cfg = draft.read().clone();
        if let Some(binds) = cfg.bindings.get_mut(&action) {
            if bind_idx < binds.len() {
                let bind = &mut binds[bind_idx];
                bind.ctrl = ctrl;
                bind.alt = alt;
                bind.shift = shift;
                bind.key = key_str;
            }
        }
        draft.set(cfg);
        recording.set(None);
    };

    let draft_read = draft.read().clone();

    // Build a global action index for each (group_action) to match Action::ALL indices
    let global_idx = |action: &Action| -> usize {
        Action::ALL.iter().position(|a| a == action).unwrap_or(0)
    };

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; \
                    background: rgba(0,0,0,0.75); z-index: 2000; display: flex; \
                    align-items: center; justify-content: center;",
            onclick: on_close,
            onkeydown: on_key,

            div {
                style: "background: #0f0f1a; border: 1px solid #1e1e3a; border-radius: 12px; \
                        width: 720px; max-width: 95vw; max-height: 85vh; display: flex; \
                        flex-direction: column; overflow: hidden; box-shadow: 0 8px 32px rgba(0,0,0,0.6);",
                onclick: move |evt| evt.stop_propagation(),
                tabindex: "0",

                // ── Title ──
                div {
                    style: "padding: 1.25rem 1.5rem 1rem; border-bottom: 1px solid #1a1a2e;",
                    h2 {
                        style: "margin: 0; font-size: 1.1rem; color: #eaeaf0;",
                        "Configure Hotkeys"
                    }
                    p {
                        style: "margin: 0.3rem 0 0 0; color: #555; font-size: 0.78rem;",
                        "Click a binding to re-record it. Press Escape to cancel."
                    }
                }

                // ── Bindings grouped by category ──
                div {
                    style: "flex: 1; overflow-y: auto; padding: 0.5rem 1.5rem 1rem;",

                    for (group_name , actions) in GROUPS.iter() {
                        div {
                            key: "{group_name}",
                            style: "margin-bottom: 1rem;",

                            // Section header
                            div {
                                style: "font-size: 0.68rem; font-weight: 600; text-transform: uppercase; \
                                        letter-spacing: 0.08em; color: #6366f1; padding: 0.4rem 0 0.3rem; \
                                        border-bottom: 1px solid #1a1a2e; margin-bottom: 0.25rem;",
                                "{group_name}"
                            }

                            for action in actions.iter() {
                                {
                                    let action_idx = global_idx(action);
                                    let action_val = *action;
                                    let action_label = action.to_string();
                                    let binds = draft_read.bindings.get(action).cloned().unwrap_or_default();

                                    rsx! {
                                        div {
                                            key: "{action_idx}",
                                            style: "display: flex; align-items: center; padding: 0.45rem 0; \
                                                    border-bottom: 1px solid #111125;",

                                            // Action label — fixed width
                                            span {
                                                style: "width: 200px; flex-shrink: 0; font-size: 0.82rem; \
                                                        color: #c0c0d0;",
                                                "{action_label}"
                                            }

                                            // Keybinds area
                                            div {
                                                style: "display: flex; flex-wrap: wrap; align-items: center; \
                                                        gap: 0.35rem; flex: 1;",

                                                for (bind_idx , bind) in binds.iter().enumerate() {
                                                    {
                                                        let is_rec = recording.read().map_or(false, |(a, b)| a == action_idx && b == bind_idx);
                                                        let display = bind.to_string();

                                                        rsx! {
                                                            div {
                                                                key: "{bind_idx}",
                                                                style: "display: inline-flex; align-items: center; gap: 0;",

                                                                // Prefix toggle
                                                                button {
                                                                    style: if bind.prefix {
                                                                        "padding: 0.2rem 0.35rem; background: #6366f1; border: none; \
                                                                         color: white; cursor: pointer; font-size: 0.6rem; \
                                                                         border-radius: 4px 0 0 4px; font-weight: 600;"
                                                                    } else {
                                                                        "padding: 0.2rem 0.35rem; background: #1a1a2e; border: 1px solid #252540; \
                                                                         color: #555; cursor: pointer; font-size: 0.6rem; \
                                                                         border-radius: 4px 0 0 4px;"
                                                                    },
                                                                    title: "Toggle prefix mode",
                                                                    onclick: move |evt| {
                                                                        evt.stop_propagation();
                                                                        let mut cfg = draft.read().clone();
                                                                        if let Some(binds) = cfg.bindings.get_mut(&action_val) {
                                                                            if let Some(b) = binds.get_mut(bind_idx) {
                                                                                b.prefix = !b.prefix;
                                                                            }
                                                                        }
                                                                        draft.set(cfg);
                                                                    },
                                                                    "P"
                                                                }

                                                                // Keybind button — click to record
                                                                button {
                                                                    style: if is_rec {
                                                                        "padding: 0.2rem 0.55rem; background: #6366f1; border: 1px solid #818cf8; \
                                                                         color: white; cursor: pointer; font-size: 0.75rem; \
                                                                         border-radius: 0; font-family: monospace;"
                                                                    } else {
                                                                        "padding: 0.2rem 0.55rem; background: #1a1a2e; border: 1px solid #252540; \
                                                                         color: #d0d0e0; cursor: pointer; font-size: 0.75rem; \
                                                                         border-radius: 0; font-family: monospace;"
                                                                    },
                                                                    onclick: move |evt| {
                                                                        evt.stop_propagation();
                                                                        recording.set(Some((action_idx, bind_idx)));
                                                                    },
                                                                    if is_rec { "..." } else { "{display}" }
                                                                }

                                                                // Remove binding
                                                                button {
                                                                    style: "padding: 0.2rem 0.35rem; background: #1a1a2e; border: 1px solid #252540; \
                                                                            color: #555; cursor: pointer; font-size: 0.6rem; \
                                                                            border-radius: 0 4px 4px 0;",
                                                                    title: "Remove binding",
                                                                    onclick: move |evt| {
                                                                        evt.stop_propagation();
                                                                        let mut cfg = draft.read().clone();
                                                                        if let Some(binds) = cfg.bindings.get_mut(&action_val) {
                                                                            if bind_idx < binds.len() {
                                                                                binds.remove(bind_idx);
                                                                            }
                                                                        }
                                                                        draft.set(cfg);
                                                                        recording.set(None);
                                                                    },
                                                                    "\u{00d7}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                // Add binding
                                                button {
                                                    style: "padding: 0.15rem 0.45rem; background: transparent; border: 1px dashed #252540; \
                                                            color: #555; cursor: pointer; font-size: 0.7rem; border-radius: 4px;",
                                                    title: "Add new binding",
                                                    onclick: move |evt| {
                                                        evt.stop_propagation();
                                                        let mut cfg = draft.read().clone();
                                                        let binds = cfg.bindings.entry(action_val).or_default();
                                                        let new_idx = binds.len();
                                                        binds.push(Keybind {
                                                            prefix: false,
                                                            ctrl: false,
                                                            alt: false,
                                                            shift: false,
                                                            key: String::new(),
                                                        });
                                                        draft.set(cfg);
                                                        recording.set(Some((action_idx, new_idx)));
                                                    },
                                                    "+"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Footer ──
                div {
                    style: "padding: 0.75rem 1.5rem; border-top: 1px solid #1a1a2e; \
                            display: flex; justify-content: space-between; align-items: center; \
                            background: #0c0c16;",

                    button {
                        style: "padding: 0.45rem 0.9rem; background: transparent; color: #ef4444; \
                                border: 1px solid #3b1111; border-radius: 6px; cursor: pointer; \
                                font-size: 0.8rem;",
                        onclick: on_reset,
                        "Reset to Defaults"
                    }

                    div {
                        style: "display: flex; gap: 0.5rem; align-items: center;",

                        span {
                            style: "font-size: 0.7rem; color: #444; margin-right: 0.5rem;",
                            "~/.config/muxspace/hotkeys.json"
                        }

                        button {
                            style: "padding: 0.45rem 0.9rem; background: transparent; color: #a0a0b0; \
                                    border: 1px solid #1e1e3a; border-radius: 6px; cursor: pointer; \
                                    font-size: 0.8rem;",
                            onclick: on_close,
                            "Cancel"
                        }
                        button {
                            style: "padding: 0.45rem 0.9rem; background: #6366f1; color: white; \
                                    border: none; border-radius: 6px; cursor: pointer; font-weight: 500; \
                                    font-size: 0.8rem;",
                            onclick: on_save,
                            "Save"
                        }
                    }
                }
            }
        }
    }
}

/// Simple modal showing the default keyboard shortcuts reference.
#[component]
pub fn ShortcutsHelpModal() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let config = use_context::<Signal<HotkeyConfig>>();

    let on_close = move |_: MouseEvent| {
        state.write().show_shortcuts_help = false;
    };

    let cfg = config.read();

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; \
                    background: rgba(0,0,0,0.75); z-index: 2000; display: flex; \
                    align-items: center; justify-content: center;",
            onclick: on_close,

            div {
                style: "background: #0f0f1a; border: 1px solid #1e1e3a; border-radius: 12px; \
                        width: 580px; max-width: 95vw; max-height: 85vh; overflow-y: auto; \
                        padding: 1.5rem; box-shadow: 0 8px 32px rgba(0,0,0,0.6);",
                onclick: move |evt| evt.stop_propagation(),

                h2 {
                    style: "margin: 0 0 1rem 0; font-size: 1.1rem; color: #eaeaf0;",
                    "Keyboard Shortcuts"
                }

                for (group_name , actions) in GROUPS.iter() {
                    div {
                        key: "{group_name}",
                        style: "margin-bottom: 1rem;",

                        div {
                            style: "font-size: 0.68rem; font-weight: 600; text-transform: uppercase; \
                                    letter-spacing: 0.08em; color: #6366f1; padding: 0.3rem 0; \
                                    border-bottom: 1px solid #1a1a2e; margin-bottom: 0.2rem;",
                            "{group_name}"
                        }

                        for action in actions.iter() {
                            {
                                let binds = cfg.bindings.get(action).cloned().unwrap_or_default();
                                let label = action.to_string();
                                let keys: String = binds.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("  /  ");

                                rsx! {
                                    div {
                                        style: "display: flex; padding: 0.3rem 0; font-size: 0.82rem; \
                                                font-family: system-ui, sans-serif;",

                                        span {
                                            style: "width: 200px; flex-shrink: 0; color: #888;",
                                            "{label}"
                                        }
                                        span {
                                            style: "color: #d0d0e0; font-family: monospace; font-size: 0.8rem;",
                                            "{keys}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    style: "margin-top: 1rem; display: flex; justify-content: space-between; \
                            align-items: center; padding-top: 0.75rem; border-top: 1px solid #1a1a2e;",

                    p {
                        style: "margin: 0; color: #444; font-size: 0.72rem;",
                        "Edit > Configure Hotkeys  or  ~/.config/muxspace/hotkeys.json"
                    }

                    button {
                        style: "padding: 0.45rem 0.9rem; background: #6366f1; color: white; \
                                border: none; border-radius: 6px; cursor: pointer; font-size: 0.8rem;",
                        onclick: on_close,
                        "Close"
                    }
                }
            }
        }
    }
}
