#[path = "editor_pane_parts.rs"]
mod editor_pane_parts;

use self::editor_pane_parts::{
    EditorHeader, EditorSidebar, EMPTY_STYLE, STATUS_STYLE, TEXTAREA_STYLE,
};
use crate::editor::{self, EditorDocument};
use crate::state::{AppState, PaneKind};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn EditorPane(pane_id: String) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let workspace_root = state
        .read()
        .editor_root_dir(&pane_id)
        .unwrap_or_else(|| state.read().active_workspace_cwd());
    let pane_kind = state
        .read()
        .active_workspace()
        .and_then(|workspace| workspace.panes.iter().find(|pane| pane.id == pane_id))
        .map(|pane| pane.kind.clone());
    let Some(PaneKind::Editor {
        name: editor_name,
        executable: editor_cmd,
        active_file: saved_active_file,
        ..
    }) = pane_kind
    else {
        return rsx! { div { style: CENTER_STYLE, "Editor pane is unavailable" } };
    };

    let entries = use_memo({
        let workspace_root = workspace_root.clone();
        move || editor::list_workspace_entries(&workspace_root).unwrap_or_default()
    })
    .read()
    .clone();
    let mut current_doc = use_signal(|| Option::<EditorDocument>::None);
    let mut current_text = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut loaded_saved_file = use_signal(|| false);
    let mut dirty = use_signal(|| false);

    if !*loaded_saved_file.read() {
        loaded_saved_file.set(true);
        if let Some(path) = saved_active_file.as_ref() {
            load_document(
                &workspace_root,
                path,
                &mut current_doc,
                &mut current_text,
                &mut status,
            );
        }
    }

    let active_path_label = current_doc
        .read()
        .as_ref()
        .map(|doc| editor::relative_label(&workspace_root, &doc.path))
        .unwrap_or_else(|| workspace_root.display().to_string());
    let active_path = current_doc.read().as_ref().map(|doc| doc.path.clone());
    let header_doc = current_doc.read().clone();
    let is_dirty = *dirty.read();
    let has_document = header_doc.is_some();
    let status_text = status.read().clone();

    rsx! {
        div { style: "display: flex; flex: 1; min-height: 0; background: #0a0a14;",
            EditorSidebar {
                entries,
                workspace_root: workspace_root.clone(),
                active_path,
                editor_cmd: editor_cmd.clone(),
                editor_name: editor_name.clone(),
                on_status: move |msg| status.set(msg),
                on_open: open_handler(
                    &pane_id,
                    &workspace_root,
                    &mut state,
                    &mut current_doc,
                    &mut current_text,
                    &mut status,
                    &mut dirty,
                ),
            }
            div { style: "display: flex; flex-direction: column; flex: 1; min-width: 0;",
                EditorHeader {
                    editor_name: editor_name.clone(),
                    editor_cmd,
                    workspace_root: workspace_root.clone(),
                    active_path_label,
                    current_doc: header_doc,
                    dirty: is_dirty,
                    on_status: move |msg| status.set(msg),
                    on_save: save_handler(&pane_id, &workspace_root, &mut state, &current_doc, &current_text, &mut status, &mut dirty),
                }
                if !has_document {
                    EmptyEditor { editor_name: editor_name.clone() }
                } else {
                    textarea {
                        style: TEXTAREA_STYLE,
                        value: "{current_text}",
                        oninput: move |evt| {
                            current_text.set(evt.value());
                            dirty.set(true);
                        },
                    }
                }
                div { style: STATUS_STYLE,
                    span { "Profile: {editor_name}" }
                    if is_dirty { span { style: "color: #f0c674;", "Unsaved changes" } }
                    if !status_text.is_empty() {
                        span { style: "color: #c8cbdd; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{status_text}" }
                    }
                }
            }
        }
    }
}

#[component]
fn EmptyEditor(editor_name: String) -> Element {
    rsx! {
        div { style: EMPTY_STYLE,
            div { style: "max-width: 32.5rem; text-align: center;",
                div { style: "font-size: 1.05rem; color: #eef0ff; margin-bottom: 0.5rem;", "Edit here, hand off when needed" }
                div { style: "font-size: 0.88rem; color: #8a8dae; line-height: 1.6;",
                    "Use the file list to edit text files in Muxspace, then open the current file or whole workspace in {editor_name} for the full native app."
                }
            }
        }
    }
}

fn open_handler(
    pane_id: &str,
    root: &std::path::Path,
    state: &mut Signal<AppState>,
    current_doc: &mut Signal<Option<EditorDocument>>,
    current_text: &mut Signal<String>,
    status: &mut Signal<String>,
    dirty: &mut Signal<bool>,
) -> EventHandler<PathBuf> {
    let pane_id = pane_id.to_string();
    let root = root.to_path_buf();
    let mut state = *state;
    let mut current_doc = *current_doc;
    let mut current_text = *current_text;
    let mut status = *status;
    let mut dirty = *dirty;
    EventHandler::new(move |path: PathBuf| {
        if *dirty.read() {
            status.set("Save or discard current changes before opening another file".to_string());
            return;
        }
        match editor::open_document(&root, &path) {
            Ok(doc) => {
                current_text.set(doc.contents.clone());
                current_doc.set(Some(doc.clone()));
                dirty.set(false);
                state
                    .write()
                    .set_editor_active_file(&pane_id, Some(doc.path));
                status.set(String::new());
            }
            Err(err) => status.set(err.to_string()),
        }
    })
}

fn save_handler(
    pane_id: &str,
    root: &std::path::Path,
    state: &mut Signal<AppState>,
    current_doc: &Signal<Option<EditorDocument>>,
    current_text: &Signal<String>,
    status: &mut Signal<String>,
    dirty: &mut Signal<bool>,
) -> EventHandler<()> {
    let pane_id = pane_id.to_string();
    let root = root.to_path_buf();
    let mut state = *state;
    let current_doc = *current_doc;
    let current_text = *current_text;
    let mut status = *status;
    let mut dirty = *dirty;
    EventHandler::new(move |_| {
        if let Some(doc) = current_doc.read().as_ref() {
            match editor::save_document(&root, &doc.path, &current_text.read(), doc.permissions) {
                Ok(()) => {
                    state
                        .write()
                        .set_editor_active_file(&pane_id, Some(doc.path.clone()));
                    dirty.set(false);
                    status.set(format!("Saved {}", doc.title));
                }
                Err(err) => status.set(err.to_string()),
            }
        }
    })
}

fn load_document(
    root: &std::path::Path,
    path: &std::path::Path,
    current_doc: &mut Signal<Option<EditorDocument>>,
    current_text: &mut Signal<String>,
    status: &mut Signal<String>,
) {
    match editor::open_document(root, path) {
        Ok(doc) => {
            current_text.set(doc.contents.clone());
            current_doc.set(Some(doc));
            status.set(String::new());
        }
        Err(err) => status.set(err.to_string()),
    }
}

const CENTER_STYLE: &str =
    "display: flex; flex: 1; align-items: center; justify-content: center; color: #888;";
