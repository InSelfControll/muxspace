use super::browser_content::{BrowserContent, BrowserTabBar};
use super::browser_nav::BrowserNavBar;
use super::terminal_content::TerminalContent;
use crate::components::editor_pane::EditorPane;
use crate::state::{AppState, Pane, PaneKind};
use dioxus::prelude::*;

#[component]
pub fn PaneView(pane: Pane, focused: bool) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let pane_id_focus = pane.id.clone();
    let pane_id_close = pane.id.clone();
    let is_browser = matches!(pane.kind, PaneKind::Browser { .. });
    let is_editor = matches!(pane.kind, PaneKind::Editor { .. });
    let title = pane_title(&pane, &state.read());
    let border = if focused { "#6366f1" } else { "#2e2e4a" };

    rsx! {
        div {
            class: "pane-container",
            style: "background: #16162a; border: 0.125rem solid {border}; border-radius: 0.5rem; display: flex; flex-direction: column; overflow: hidden; width: 100%; height: 100%; min-width: 0; min-height: 18.75rem;",
            onclick: move |_| state.write().focused_pane_id = Some(pane_id_focus.clone()),
            div { class: "pane-header", style: HEADER,
                if is_browser {
                    BrowserNavBar { pane_id: pane.id.clone() }
                } else {
                    span { style: TITLE, "{title}" }
                }
                if is_editor {
                    span { style: "margin-left: auto; margin-right: 0.5rem; color: #7c7ca5; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.04em;", "Workspace Editor" }
                }
                div { style: "display: flex; align-items: center; gap: 0.5rem; flex-shrink: 0; margin-left: 0.5rem;",
                    if focused { span { style: "width: 0.5rem; height: 0.5rem; background: #6366f1; border-radius: 50%; display: inline-block;" } }
                    button {
                        style: CLOSE,
                        onclick: move |evt| {
                            evt.stop_propagation();
                            state.write().remove_pane(&pane_id_close);
                        },
                        "x"
                    }
                }
            }
            if is_browser { BrowserTabBar { pane_id: pane.id.clone() } }
            match &pane.kind {
                PaneKind::Terminal { .. } => rsx! {
                    TerminalContent { pane_id: pane.id.clone(), pty_id: pane.pty_id.clone(), focused }
                },
                PaneKind::Browser { tabs, active_tab, profile, .. } => rsx! {
                    BrowserContent {
                        pane_id: pane.id.clone(),
                        initial_urls: tabs.clone(),
                        initial_active: *active_tab,
                        profile: profile.clone(),
                    }
                },
                PaneKind::Editor { .. } => rsx! { EditorPane { pane_id: pane.id.clone() } },
            }
        }
    }
}

fn pane_title(pane: &Pane, state: &AppState) -> String {
    if let Some(name) = &pane.custom_name {
        return name.clone();
    }
    let pty_id = pane.pty_id.clone().unwrap_or_else(|| pane.id.clone());
    if let Some(title) = state.screen_buffers.get(&pty_id).map(|b| b.title.clone()) {
        if !title.is_empty() {
            return title;
        }
    }
    match &pane.kind {
        PaneKind::Terminal { command: Some(cmd) } => format!("Terminal: {cmd}"),
        PaneKind::Terminal { command: None } => "Terminal".to_string(),
        PaneKind::Browser { .. } => "Browser".to_string(),
        PaneKind::Editor { name, .. } => format!("Editor: {name}"),
    }
}

const HEADER: &str = "padding: 0.35rem 0.75rem; background: #1e1e3a; border-bottom: 0.0625rem solid #2e2e4a; display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;";
const TITLE: &str = "font-weight: 500; font-size: 0.85rem; cursor: default; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 18.75rem;";
const CLOSE: &str = "padding: 0.15rem 0.4rem; background: transparent; border: none; color: #ff5555; cursor: pointer; font-size: 0.95rem; opacity: 0.8; line-height: 1;";
