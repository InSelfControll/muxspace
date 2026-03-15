use dioxus::prelude::*;

use crate::state::{AppState, Project};

/// Project navigator overlay for quick switching
#[component]
pub fn ProjectNav(
    visible: bool,
    on_close: EventHandler<()>,
) -> Element {
    let state = use_context::<Signal<AppState>>();
    let selected_idx = use_signal(|| 0usize);
    
    if !visible {
        return rsx! { div {} };
    }
    
    // Clone projects for the iterator
    let projects: Vec<Project> = state.read().projects.clone();
    
    rsx! {
        div {
            class: "project-nav-overlay",
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.7); z-index: 1000; display: flex; align-items: center; justify-content: center;",
            
            onclick: move |_| on_close.call(()),
            
            div {
                class: "project-nav-modal",
                style: "background: #16162a; border: 1px solid #2e2e4a; border-radius: 16px; width: 600px; max-width: 90vw; max-height: 80vh; display: flex; flex-direction: column; overflow: hidden;",
                
                onclick: move |evt| evt.stop_propagation(),
                
                div {
                    class: "project-nav-header",
                    style: "padding: 1.5rem; border-bottom: 1px solid #2e2e4a;",
                    
                    h2 {
                        style: "margin: 0; font-size: 1.25rem;",
                        "Switch Project"
                    }
                    
                    p {
                        style: "margin: 0.5rem 0 0 0; color: #a0a0b0; font-size: 0.9rem;",
                        "Use arrow keys to navigate, Enter to select"
                    }
                }
                
                div {
                    class: "project-list",
                    style: "flex: 1; overflow-y: auto; padding: 1rem;",
                    
                    ProjectList {
                        projects: projects,
                        selected_idx: selected_idx.clone(),
                        state: state.clone(),
                        on_close: on_close.clone(),
                    }
                }
                
                div {
                    class: "project-nav-footer",
                    style: "padding: 1rem; border-top: 1px solid #2e2e4a; display: flex; justify-content: space-between; align-items: center;",
                    
                    span {
                        style: "color: #a0a0b0; font-size: 0.8rem;",
                        "Press ESC to close"
                    }
                    
                    button {
                        style: "padding: 0.5rem 1rem; background: #6366f1; color: white; border: none; border-radius: 6px; cursor: pointer;",
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                }
            }
        }
    }
}

#[component]
fn ProjectList(
    projects: Vec<Project>,
    mut selected_idx: Signal<usize>,
    mut state: Signal<AppState>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        for (idx, project) in projects.into_iter().enumerate() {
            ProjectNavItem {
                key: "{project.id}",
                project: project.clone(),
                selected: idx == *selected_idx.read(),
                on_hover: move |_| selected_idx.set(idx),
                on_select: move |id: String| {
                    state.write().switch_project_blocking(&id);
                    on_close.call(());
                }
            }
        }
    }
}

#[component]
fn ProjectNavItem(
    project: Project,
    selected: bool,
    on_hover: EventHandler<()>,
    on_select: EventHandler<String>,
) -> Element {
    let project_id = project.id.clone();
    
    rsx! {
        div {
            class: if selected { "project-nav-item selected" } else { "project-nav-item" },
            style: if selected {
                "padding: 1rem; background: rgba(99, 102, 241, 0.2); border: 1px solid #6366f1; border-radius: 8px; cursor: pointer; margin-bottom: 0.5rem; display: flex; align-items: center; gap: 1rem;"
            } else {
                "padding: 1rem; background: #0f0f1a; border: 1px solid #2e2e4a; border-radius: 8px; cursor: pointer; margin-bottom: 0.5rem; display: flex; align-items: center; gap: 1rem;"
            },
            
            onmouseenter: move |_| on_hover.call(()),
            onclick: move |_| on_select.call(project_id.clone()),
            
            div {
                class: "project-icon",
                style: "width: 40px; height: 40px; background: #6366f1; border-radius: 8px; display: flex; align-items: center; justify-content: center; font-size: 1.2rem;",
                "📁"
            }
            
            div {
                class: "project-info",
                style: "flex: 1;",
                
                div {
                    style: "font-weight: 500; margin-bottom: 0.25rem;",
                    "{project.name}"
                }
                
                div {
                    style: "font-size: 0.8rem; color: #a0a0b0;",
                    "{project.workspaces.len()} workspaces"
                }
            }
            
            if selected {
                div {
                    style: "color: #6366f1;",
                    "→"
                }
            }
        }
    }
}
