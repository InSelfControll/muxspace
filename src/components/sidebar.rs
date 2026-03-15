use dioxus::prelude::*;

use crate::state::AppState;
use crate::sync::SyncManager;

#[allow(non_snake_case)]
pub fn sidebar() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let state_read = state.read();
    let projects = state_read.projects.clone();
    let active_id = state_read.active_project_id.clone();
    drop(state_read);

    rsx! {
        aside {
            class: "sidebar",
            style: "width: 260px; background: #16162a; border-right: 1px solid #2e2e4a; display: flex; flex-direction: column; flex-shrink: 0;",

            div {
                class: "sidebar-header",
                style: "padding: 1rem; border-bottom: 1px solid #2e2e4a;",

                h2 {
                    style: "margin: 0; font-size: 1.2rem; color: #6366f1;",
                    "Muxspace"
                }
            }

            div {
                class: "projects-section",
                style: "flex: 1; overflow-y: auto; padding: 0 1rem;",

                h3 {
                    style: "font-size: 0.75rem; text-transform: uppercase; color: #a0a0b0; margin: 1rem 0 0.5rem 0; letter-spacing: 0.05em;",
                    "Projects"
                }

                div {
                    class: "project-list",
                    style: "display: flex; flex-direction: column; gap: 0.25rem;",

                    for project in projects.iter() {
                        ProjectItem {
                            key: "{project.id}",
                            project: project.clone(),
                            active: active_id.as_ref() == Some(&project.id),
                        }
                    }

                    if projects.is_empty() {
                        div {
                            style: "padding: 1rem; color: #666; font-size: 0.85rem; text-align: center;",
                            "No projects yet"
                        }
                    }
                }

                button {
                    class: "new-project-btn",
                    style: "width: 100%; margin-top: 0.5rem; padding: 0.5rem; background: transparent; border: 1px dashed #2e2e4a; border-radius: 6px; color: #a0a0b0; cursor: pointer;",
                    onclick: move |_| {
                        state.write().show_create_project = true;
                    },
                    "+ New Project"
                }
            }

            div {
                class: "sidebar-footer",
                style: "padding: 1rem; border-top: 1px solid #2e2e4a; display: flex; gap: 0.5rem;",

                button {
                    style: "flex: 1; padding: 0.5rem; background: #1e1e3a; border: none; border-radius: 6px; color: #eaeaf0; cursor: pointer; font-size: 0.8rem;",
                    onclick: move |_| {
                        // Export projects to JSON file
                        if let Ok(mgr) = SyncManager::new() {
                            if let Ok(data) = mgr.export() {
                                let path = dirs::home_dir()
                                    .unwrap_or_default()
                                    .join("muxspace-export.json");
                                if std::fs::write(&path, &data).is_ok() {
                                    tracing::info!("Exported to {}", path.display());
                                }
                            }
                        }
                    },
                    "Export"
                }
                button {
                    style: "flex: 1; padding: 0.5rem; background: #1e1e3a; border: none; border-radius: 6px; color: #eaeaf0; cursor: pointer; font-size: 0.8rem;",
                    onclick: move |_| {
                        // Import projects from JSON file
                        let path = dirs::home_dir()
                            .unwrap_or_default()
                            .join("muxspace-export.json");
                        if let Ok(data) = std::fs::read(&path) {
                            if let Ok(mgr) = SyncManager::new() {
                                if mgr.import(&data).is_ok() {
                                    // Reload state
                                    *state.write() = AppState::new_blocking();
                                    tracing::info!("Imported from {}", path.display());
                                }
                            }
                        }
                    },
                    "Import"
                }
            }
        }
    }
}

#[component]
fn ProjectItem(project: crate::state::Project, active: bool) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let project_id_switch = project.id.clone();
    let project_id_rename = project.id.clone();
    let project_id_rename2 = project.id.clone();
    let project_id_del = project.id.clone();
    let mut editing = use_signal(|| false);
    let mut edit_name = use_signal(|| String::new());
    let project_name_for_edit = project.name.clone();

    rsx! {
        div {
            class: if active { "project-item active" } else { "project-item" },
            style: if active {
                "padding: 0.5rem 0.75rem; background: #6366f1; color: white; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: space-between;"
            } else {
                "padding: 0.5rem 0.75rem; background: transparent; color: #eaeaf0; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: space-between;"
            },

            onclick: move |_| {
                if !*editing.read() {
                    state.write().switch_project_blocking(&project_id_switch);
                }
            },

            if *editing.read() {
                input {
                    style: "flex: 1; padding: 0.15rem 0.4rem; background: #0f0f1a; border: 1px solid #6366f1; \
                            border-radius: 4px; color: #eaeaf0; font-size: 0.85rem; outline: none;",
                    r#type: "text",
                    value: "{edit_name}",
                    autofocus: true,
                    onclick: |evt| evt.stop_propagation(),
                    oninput: move |evt| edit_name.set(evt.value()),
                    onkeydown: move |evt: KeyboardEvent| {
                        if evt.key() == Key::Enter {
                            let new = edit_name.read().trim().to_string();
                            if !new.is_empty() {
                                state.write().rename_project(&project_id_rename, &new);
                            }
                            editing.set(false);
                        }
                        if evt.key() == Key::Escape {
                            editing.set(false);
                        }
                    },
                    onfocusout: move |_| {
                        let new = edit_name.read().trim().to_string();
                        if !new.is_empty() {
                            state.write().rename_project(&project_id_rename2, &new);
                        }
                        editing.set(false);
                    },
                }
            } else {
                span {
                    style: "flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                    ondoubleclick: move |evt| {
                        evt.stop_propagation();
                        edit_name.set(project.name.clone());
                        editing.set(true);
                    },
                    "{project.name}"
                }
            }

            // Rename button
            if !*editing.read() {
                button {
                    style: "padding: 0 0.25rem; background: transparent; border: none; \
                            color: #666; cursor: pointer; font-size: 0.7rem; opacity: 0.5; flex-shrink: 0;",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        edit_name.set(project_name_for_edit.clone());
                        editing.set(true);
                    },
                    "\u{270E}"
                }
            }

            span {
                style: "font-size: 0.75rem; opacity: 0.7; margin-left: 0.5rem;",
                "{project.workspaces.len()}"
            }

            // Delete button
            button {
                style: "margin-left: 0.5rem; padding: 0 0.3rem; background: transparent; border: none; color: #ff5555; cursor: pointer; font-size: 0.8rem; opacity: 0.5;",
                onclick: move |evt| {
                    evt.stop_propagation();
                    state.write().delete_project(&project_id_del);
                },
                "x"
            }
        }
    }
}
