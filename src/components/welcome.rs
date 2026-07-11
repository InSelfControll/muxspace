use crate::state::AppState;
use dioxus::prelude::*;

#[component]
pub fn WelcomeView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    rsx! {
        section {
            class: "welcome-view",
            style: "flex: 1; min-width: 0; overflow: auto; display: flex; align-items: center; justify-content: center; padding: 3rem 2rem;",
            div { style: "width: calc(100% - 2rem); max-width: 40rem; text-align: center;",
                h1 { style: "font-size: 1.75rem; line-height: 1.25; margin: 0 0 0.75rem; font-weight: 650;", "Welcome to Muxspace" }
                p { style: "color: #a0a0b0; line-height: 1.6; margin: 0 auto 1.75rem; max-width: 31.25rem;",
                    "Select a project from the sidebar or create a new one to get started."
                }
                button {
                    style: "padding: 0.7rem 1.1rem; background: #6366f1; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-size: 0.9rem; font-weight: 600;",
                    onclick: move |_| state.write().show_create_project = true,
                    "Create New Project"
                }
                section { style: "margin-top: 3rem; padding-top: 1.5rem; border-top: 0.0625rem solid #2e2e4a; text-align: left;",
                    h2 { style: "margin: 0 0 1rem; font-size: 0.8rem; color: #c8c8d4; text-transform: uppercase; letter-spacing: 0.08em;", "Keyboard shortcuts" }
                    div { style: "display: grid; grid-template-columns: minmax(8.125rem, 0.7fr) minmax(13.75rem, 1.3fr); column-gap: 1.5rem; row-gap: 0.7rem; color: #a0a0b0; font-size: 0.85rem; line-height: 1.4;",
                        Shortcut { keys: "Ctrl+B, arrows", action: "Move focus between panes" }
                        Shortcut { keys: "Ctrl+B, n/p", action: "Next or previous workspace" }
                        Shortcut { keys: "Ctrl+B, 0-9", action: "Jump to workspace" }
                        Shortcut { keys: "Ctrl+B, c", action: "Create workspace" }
                        Shortcut { keys: "Ctrl+B, |", action: "Open terminal pane" }
                        Shortcut { keys: "Ctrl+B, x", action: "Close focused pane" }
                    }
                }
            }
        }
    }
}

#[component]
fn Shortcut(keys: &'static str, action: &'static str) -> Element {
    rsx! {
        kbd { style: "color: #d8d8e4; font-family: ui-monospace, monospace; white-space: nowrap;", "{keys}" }
        span { "{action}" }
    }
}
