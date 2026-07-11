use crate::state::AppState;
use dioxus::prelude::*;

#[component]
pub fn CreateProjectModal() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut name = use_signal(String::new);
    let mut dir = use_signal(|| {
        dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let on_create = move |_| {
        let n = name.read().clone();
        let d = dir.read().clone();
        if !n.is_empty() && !d.is_empty() {
            state
                .write()
                .create_project(&n, std::path::PathBuf::from(d));
            state.write().show_create_project = false;
        }
    };
    let on_cancel = move |_| state.write().show_create_project = false;
    let on_browse = move |_| {
        let mut dir = dir;
        spawn(async move {
            let start = std::path::PathBuf::from(dir.read().clone());
            let picked = rfd::AsyncFileDialog::new()
                .set_title("Choose working directory")
                .set_directory(&start)
                .pick_folder()
                .await;
            if let Some(handle) = picked {
                dir.set(handle.path().to_string_lossy().to_string());
            }
        });
    };

    rsx! {
        div { style: OVERLAY, onclick: on_cancel,
            div { style: MODAL, onclick: move |evt| evt.stop_propagation(),
                h2 { style: "margin: 0 0 1.25rem 0; font-size: 1.15rem; color: #eaeaf0;", "Create New Project" }
                Field { label: "Project Name".to_string(), value: name.read().clone(), on_input: move |v| name.set(v) }
                div { style: "margin-bottom: 1.25rem;",
                    label { style: LABEL, "Working Directory" }
                    div { style: "display: flex; gap: 0.5rem;",
                        input { style: INPUT, r#type: "text", value: "{dir}", oninput: move |evt| dir.set(evt.value()) }
                        button { style: SECONDARY, onclick: on_browse, "Browse..." }
                    }
                }
                div { style: "display: flex; gap: 0.5rem; justify-content: flex-end;",
                    button { style: SECONDARY, onclick: on_cancel, "Cancel" }
                    button { style: PRIMARY, onclick: on_create, "Create Project" }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct FieldProps {
    label: String,
    value: String,
    on_input: EventHandler<String>,
}

#[component]
fn Field(props: FieldProps) -> Element {
    rsx! {
        div { style: "margin-bottom: 1rem;",
            label { style: LABEL, "{props.label}" }
            input {
                style: INPUT,
                r#type: "text",
                value: "{props.value}",
                oninput: move |evt| props.on_input.call(evt.value()),
            }
        }
    }
}

const OVERLAY: &str = "position: fixed; inset: 0; background: rgba(0,0,0,0.75); z-index: 1000; display: flex; align-items: center; justify-content: center;";
const MODAL: &str = "background: #0f0f1a; border: 0.0625rem solid #1e1e3a; border-radius: 0.75rem; width: 32.5rem; padding: 1.75rem; box-shadow: 0 0.5rem 2rem rgba(0,0,0,0.6);";
const LABEL: &str = "display: block; margin-bottom: 0.4rem; color: #888; font-size: 0.8rem;";
const INPUT: &str = "width: 100%; padding: 0.55rem 0.7rem; background: #1a1a2e; border: 0.0625rem solid #252540; border-radius: 0.375rem; color: #eaeaf0; font-size: 0.95rem; box-sizing: border-box;";
const PRIMARY: &str = "padding: 0.5rem 1rem; background: #6366f1; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 500; font-size: 0.85rem;";
const SECONDARY: &str = "padding: 0.55rem 0.9rem; background: transparent; color: #a0a0b0; border: 0.0625rem solid #252540; border-radius: 0.375rem; cursor: pointer; font-size: 0.85rem; white-space: nowrap;";
