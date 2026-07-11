use crate::browser::selection::InstalledBrowser;
use crate::components::browser_selection_modal::BrowserSelectionModal;
use crate::components::editor_selection_modal::EditorSelectionModal;
use crate::components::terminal::PaneView;
use crate::editor::InstalledEditor;
use crate::state::{AppState, Project};
use dioxus::prelude::*;

fn pick_folder_blocking() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose editor workspace")
        .pick_folder()
}

#[component]
pub fn ProjectView(project: Project) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let workspaces = project.workspaces.clone();
    let Some(current_workspace) = workspaces.get(project.active_workspace_idx).cloned() else {
        return rsx! { div { style: "padding: 1rem;", "No workspace selected" } };
    };
    let focused_pane_id = state.read().focused_pane_id.clone();
    let mut show_add_browser = use_signal(|| false);
    let mut browser_url_input = use_signal(|| String::from("http://localhost:3000"));
    let mut show_browser_selection = use_signal(|| false);
    let mut selected_browser = use_signal(|| Option::<InstalledBrowser>::None);
    let mut show_editor_selection = use_signal(|| false);

    rsx! {
        div { class: "project-view", style: "flex: 1; min-width: 0; min-height: 0; width: 100%; display: flex; flex-direction: column; padding: 1rem; overflow: hidden;",
            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.75rem;",
                h2 { style: "margin: 0; font-size: 1.1rem;", "{project.name}" }
                div { style: "display: flex; gap: 0.5rem;",
                    button { style: BTN, onclick: move |_| state.write().add_terminal_pane(None), "+ Terminal" }
                    button { style: BTN, onclick: move |_| {
                        let browsers = crate::browser::selection::detect_installed_browsers();
                        if browsers.len() > 1 { show_browser_selection.set(true); } else { show_add_browser.toggle(); }
                    }, "+ Browser" }
                    button { style: BTN, onclick: move |_| {
                        let name = format!("ws-{}", chrono::Utc::now().timestamp_millis() % 10000);
                        state.write().add_workspace(&name);
                    }, "+ Workspace" }
                    button { style: BTN, onclick: move |_| {
                        if !crate::editor::detect_installed_editors().is_empty() {
                            show_editor_selection.set(true);
                        }
                    }, "+ Editor" }
                }
            }
            if *show_add_browser.read() {
                AddBrowserRow {
                    value: browser_url_input.read().clone(),
                    on_input: move |v| browser_url_input.set(v),
                    on_add: move |_| {
                        let url = browser_url_input.read().clone();
                        if !url.is_empty() {
                            let profile = selected_browser.read().as_ref().map(|b| b.profile());
                            state.write().add_browser_pane(url, profile);
                            browser_url_input.set("http://localhost:3000".to_string());
                            show_add_browser.set(false);
                        }
                    },
                    on_cancel: move |_| show_add_browser.set(false),
                }
            }
            WorkspaceTabs { workspaces: workspaces.clone(), active_idx: project.active_workspace_idx }
            div { class: "panes", style: "flex: 1; min-width: 0; min-height: 0; width: 100%; display: grid; grid-template-columns: repeat(auto-fit, minmax(min(25rem, 100%), 1fr)); grid-auto-rows: minmax(18.75rem, 1fr); align-items: stretch; gap: 0.75rem; overflow: auto;",
                for pane in current_workspace.panes.iter() {
                    PaneView { key: "{pane.id}", pane: pane.clone(), focused: focused_pane_id.as_ref() == Some(&pane.id) }
                }
            }
            BrowserSelectionModal {
                show: *show_browser_selection.read(),
                on_select: move |browser: InstalledBrowser| {
                    selected_browser.set(Some(browser));
                    show_browser_selection.set(false);
                    show_add_browser.set(true);
                },
                on_cancel: move |_| show_browser_selection.set(false),
            }
            EditorSelectionModal {
                show: *show_editor_selection.read(),
                on_select: move |editor: InstalledEditor| {
                    if let Some(path) = pick_folder_blocking() {
                        add_editor_or_terminal(&mut state, &editor, path);
                        show_editor_selection.set(false);
                    }
                },
                on_cancel: move |_| show_editor_selection.set(false),
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AddBrowserRowProps {
    value: String,
    on_input: EventHandler<String>,
    on_add: EventHandler<()>,
    on_cancel: EventHandler<()>,
}

#[component]
fn AddBrowserRow(props: AddBrowserRowProps) -> Element {
    rsx! {
        div { style: "display: flex; gap: 0.5rem; margin-bottom: 0.75rem; align-items: center;",
            input {
                style: "flex: 1; padding: 0.4rem 0.6rem; background: #0f0f1a; border: 0.0625rem solid #2e2e4a; border-radius: 0.375rem; color: #eaeaf0; font-size: 0.9rem; font-family: monospace;",
                r#type: "text",
                value: "{props.value}",
                oninput: move |evt| props.on_input.call(evt.value()),
                onkeydown: move |evt: KeyboardEvent| {
                    if evt.key() == Key::Enter { props.on_add.call(()); }
                    if evt.key() == Key::Escape { props.on_cancel.call(()); }
                },
            }
            button { style: PRIMARY, onclick: move |_| props.on_add.call(()), "Add" }
            button { style: BTN, onclick: move |_| props.on_cancel.call(()), "Cancel" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct WorkspaceTabsProps {
    workspaces: Vec<crate::state::Workspace>,
    active_idx: usize,
}

#[component]
fn WorkspaceTabs(props: WorkspaceTabsProps) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    rsx! {
        div { class: "workspace-tabs", style: "display: flex; gap: 0.25rem; margin-bottom: 0.75rem; flex-wrap: wrap;",
            for (idx, workspace) in props.workspaces.iter().enumerate() {
                div { key: "{workspace.id}", style: "display: flex; align-items: center;",
                    button {
                        style: if idx == props.active_idx { TAB_ACTIVE } else { TAB },
                        onclick: move |_| state.write().switch_workspace(idx),
                        if idx < 9 { "{idx + 1}. {workspace.name}" } else { "{workspace.name}" }
                    }
                    if props.workspaces.len() > 1 {
                        button {
                            style: if idx == props.active_idx { CLOSE_ACTIVE } else { CLOSE },
                            onclick: move |_| state.write().remove_workspace(idx),
                            "x"
                        }
                    }
                }
            }
        }
    }
}

fn add_editor_or_terminal(
    state: &mut Signal<AppState>,
    editor: &InstalledEditor,
    path: std::path::PathBuf,
) {
    if matches!(editor.id.as_str(), "vim" | "neovim" | "helix") {
        state
            .write()
            .add_terminal_pane(Some(format!("{} {}", editor.executable, path.display())));
    } else if editor.id == "opencode" {
        state.write().add_editor_pane(
            format!("{} App", editor.name),
            editor.executable.clone(),
            path.clone(),
        );
        state
            .write()
            .add_terminal_pane(Some(format!("{} {}", editor.executable, path.display())));
    } else {
        state
            .write()
            .add_editor_pane(editor.name.clone(), editor.executable.clone(), path);
    }
}

const BTN: &str = "padding: 0.4rem 0.8rem; background: #1e1e3a; color: #eaeaf0; border: 0.0625rem solid #2e2e4a; border-radius: 0.375rem; cursor: pointer; font-size: 0.85rem;";
const PRIMARY: &str = "padding: 0.4rem 0.8rem; background: #6366f1; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-size: 0.85rem;";
const TAB: &str = "padding: 0.4rem 0.8rem; background: #1e1e3a; color: #a0a0b0; border: none; border-radius: 0.375rem 0 0 0.375rem; cursor: pointer; font-size: 0.85rem;";
const TAB_ACTIVE: &str = "padding: 0.4rem 0.8rem; background: #6366f1; color: white; border: none; border-radius: 0.375rem 0 0 0.375rem; cursor: pointer; font-size: 0.85rem;";
const CLOSE: &str = "padding: 0.4rem; background: #1e1e3a; color: #666; border: none; border-radius: 0 0.375rem 0.375rem 0; cursor: pointer; font-size: 0.7rem;";
const CLOSE_ACTIVE: &str = "padding: 0.4rem; background: #6366f1; color: rgba(255,255,255,0.6); border: none; border-radius: 0 0.375rem 0.375rem 0; cursor: pointer; font-size: 0.7rem;";
