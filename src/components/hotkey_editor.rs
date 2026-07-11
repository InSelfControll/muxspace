use crate::hotkeys::{Action, HotkeyConfig, Keybind};
use crate::state::AppState;
use dioxus::prelude::*;

#[component]
pub fn HotkeyEditorModal() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let config = use_context::<Signal<HotkeyConfig>>();
    rsx! {
        div { style: OVERLAY, onclick: move |_| state.write().show_hotkey_editor = false,
            div { style: MODAL, onclick: move |evt| evt.stop_propagation(),
                h2 { style: TITLE, "Configure Hotkeys" }
                div { style: "max-height: 58vh; overflow: auto; padding-right: 0.25rem;",
                    for action in Action::ALL {
                        HotkeyRow { action: *action, binds: config.read().bindings.get(action).cloned().unwrap_or_default() }
                    }
                }
                div { style: FOOTER,
                    button { style: SECONDARY, onclick: move |_| state.write().show_hotkey_editor = false, "Close" }
                    button { style: PRIMARY, onclick: move |_| config.read().save(), "Save" }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct HotkeyRowProps {
    action: Action,
    binds: Vec<Keybind>,
}

#[component]
fn HotkeyRow(props: HotkeyRowProps) -> Element {
    let binds = if props.binds.is_empty() {
        "Unassigned".to_string()
    } else {
        props
            .binds
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    rsx! {
        div { style: ROW,
            div { style: "font-size: 0.9rem; color: #f2f2ff;", "{props.action}" }
            div { style: "font-size: 0.8rem; color: #a0a0c0; font-family: monospace;", "{binds}" }
        }
    }
}

#[component]
pub fn ShortcutsHelpModal() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    rsx! {
        div { style: OVERLAY, onclick: move |_| state.write().show_shortcuts_help = false,
            div { style: MODAL, onclick: move |evt| evt.stop_propagation(),
                h2 { style: TITLE, "Keyboard Shortcuts" }
                div { style: "display: grid; gap: 0.55rem; margin-bottom: 1.25rem;",
                    HelpRow { keys: "Ctrl+B, arrows".to_string(), action: "Move focus between panes".to_string() }
                    HelpRow { keys: "Ctrl+B, n/p".to_string(), action: "Next or previous workspace".to_string() }
                    HelpRow { keys: "Ctrl+B, 0-9".to_string(), action: "Jump to workspace".to_string() }
                    HelpRow { keys: "Ctrl+B, c".to_string(), action: "New workspace".to_string() }
                    HelpRow { keys: "Ctrl+B, |".to_string(), action: "New terminal pane".to_string() }
                    HelpRow { keys: "Ctrl+B, x".to_string(), action: "Close focused pane".to_string() }
                    HelpRow { keys: "Alt+1-9".to_string(), action: "Quick workspace switch".to_string() }
                }
                div { style: FOOTER,
                    button { style: PRIMARY, onclick: move |_| state.write().show_shortcuts_help = false, "Close" }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct HelpRowProps {
    keys: String,
    action: String,
}

#[component]
fn HelpRow(props: HelpRowProps) -> Element {
    rsx! {
        div { style: ROW,
            code { style: "color: #c4c6ff; font-family: monospace;", "{props.keys}" }
            span { style: "color: #d7d8ea;", "{props.action}" }
        }
    }
}

const OVERLAY: &str = "position: fixed; inset: 0; background: rgba(0,0,0,0.75); z-index: 1000; display: flex; align-items: center; justify-content: center;";
const MODAL: &str = "background: #0f0f1a; border: 0.0625rem solid #1e1e3a; border-radius: 0.75rem; width: 38.75rem; padding: 1.5rem; box-shadow: 0 0.5rem 2rem rgba(0,0,0,0.6);";
const TITLE: &str = "margin: 0 0 1rem 0; font-size: 1.15rem; color: #eaeaf0;";
const ROW: &str = "display: flex; justify-content: space-between; gap: 1rem; padding: 0.6rem 0; border-bottom: 0.0625rem solid #20203a;";
const FOOTER: &str = "display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem;";
const PRIMARY: &str = "padding: 0.5rem 1rem; background: #6366f1; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-size: 0.85rem;";
const SECONDARY: &str = "padding: 0.5rem 1rem; background: transparent; color: #a0a0b0; border: 0.0625rem solid #1e1e3a; border-radius: 0.375rem; cursor: pointer; font-size: 0.85rem;";
