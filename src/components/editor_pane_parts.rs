use crate::editor::{self, EditorDocument, EditorFileEntry};
use dioxus::prelude::*;
use std::path::PathBuf;

#[derive(Props, Clone, PartialEq)]
pub struct EditorSidebarProps {
    pub entries: Vec<EditorFileEntry>,
    pub workspace_root: PathBuf,
    pub active_path: Option<PathBuf>,
    pub editor_cmd: String,
    pub editor_name: String,
    pub on_open: EventHandler<PathBuf>,
    pub on_status: EventHandler<String>,
}

#[component]
pub fn EditorSidebar(props: EditorSidebarProps) -> Element {
    rsx! {
        div { style: SIDEBAR_STYLE,
            div { style: "margin-bottom: 0.65rem;",
                div { style: LABEL_STYLE, "Workspace" }
                div { style: PATH_STYLE, "{props.workspace_root.display()}" }
            }
            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.5rem; gap: 0.5rem;",
                div { style: LABEL_STYLE, "Files" }
                button { style: BUTTON_STYLE, onclick: move |_| {
                    match editor::open_in_editor(&props.editor_cmd, &props.workspace_root, None) {
                        Ok(()) => props.on_status.call(format!("Opened in {}", props.editor_name)),
                        Err(err) => props.on_status.call(err.to_string()),
                    }
                }, "Open workspace" }
            }
            if props.entries.is_empty() {
                div { style: EMPTY_LIST, "No visible files in this workspace yet." }
            } else {
                for entry in props.entries.iter() {
                    FileTreeRow {
                        entry: entry.clone(),
                        active_path: props.active_path.clone(),
                        on_open: move |path| props.on_open.call(path),
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct EditorHeaderProps {
    pub editor_name: String,
    pub editor_cmd: String,
    pub workspace_root: PathBuf,
    pub active_path_label: String,
    pub current_doc: Option<EditorDocument>,
    pub dirty: bool,
    pub on_save: EventHandler<()>,
    pub on_status: EventHandler<String>,
}

#[component]
pub fn EditorHeader(props: EditorHeaderProps) -> Element {
    let title = props
        .current_doc
        .as_ref()
        .map(|doc| doc.title.clone())
        .unwrap_or_else(|| "Start editing in Muxspace".to_string());
    let current_doc = props.current_doc.clone();
    let save_disabled = current_doc.is_none() || !props.dirty;
    rsx! {
        div { style: HEADER_STYLE,
            div { style: "min-width: 0; display: flex; flex-direction: column; gap: 0.15rem;",
                div { style: TITLE_STYLE, "{title}" }
                div { style: PATH_STYLE, "{props.active_path_label}" }
            }
            div { style: "display: flex; align-items: center; gap: 0.5rem; flex-shrink: 0;",
                button { style: BUTTON_STYLE, onclick: move |_| {
                    let file = current_doc.as_ref().map(|doc| doc.path.as_path());
                    match editor::open_in_editor(&props.editor_cmd, &props.workspace_root, file) {
                        Ok(()) => props.on_status.call(format!("Handed off to {}", props.editor_name)),
                        Err(err) => props.on_status.call(err.to_string()),
                    }
                }, "Open in {props.editor_name}" }
                button {
                    style: SAVE_STYLE,
                    disabled: save_disabled,
                    onclick: move |_| props.on_save.call(()),
                    "Save"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct FileTreeRowProps {
    entry: EditorFileEntry,
    active_path: Option<PathBuf>,
    on_open: EventHandler<PathBuf>,
}

#[component]
fn FileTreeRow(props: FileTreeRowProps) -> Element {
    let is_active = props.active_path.as_ref() == Some(&props.entry.path);
    let label = if props.entry.is_dir {
        format!("[dir] {}", props.entry.name)
    } else {
        props.entry.name.clone()
    };
    rsx! {
        button {
            style: if is_active { FILE_ACTIVE_STYLE } else { FILE_STYLE },
            disabled: props.entry.is_dir,
            onclick: { let path = props.entry.path.clone(); move |_| props.on_open.call(path.clone()) },
            "{label}"
        }
    }
}

pub const EMPTY_STYLE: &str = "display: flex; flex: 1; align-items: center; justify-content: center; padding: 1.5rem; background: #0a0a14;";
pub const TEXTAREA_STYLE: &str = "flex: 1; width: 100%; resize: none; border: none; outline: none; background: #0a0a14; color: #eaeaf0; padding: 0.9rem; font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Fira Code', monospace; font-size: 0.8125rem; line-height: 1.55;";
pub const STATUS_STYLE: &str = "display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; padding: 0.4rem 0.75rem; border-top: 0.0625rem solid #2e2e4a; color: #8c90ad; font-size: 0.78rem; background: #10101c;";
const SIDEBAR_STYLE: &str = "width: 17.5rem; border-right: 0.0625rem solid #2e2e4a; overflow: auto; padding: 0.75rem 0.6rem; background: #0d0d18;";
const LABEL_STYLE: &str =
    "font-size: 0.72rem; color: #7f7fa4; text-transform: uppercase; letter-spacing: 0.06em;";
const PATH_STYLE: &str = "margin-top: 0.2rem; font-size: 0.82rem; color: #e5e7ef; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;";
const BUTTON_STYLE: &str = "padding: 0.35rem 0.7rem; background: #1d1d33; color: #d7d8ea; border: 0.0625rem solid #303053; border-radius: 0.375rem; cursor: pointer; font-size: 0.78rem;";
const SAVE_STYLE: &str = "padding: 0.35rem 0.75rem; background: #6366f1; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-size: 0.8rem;";
const HEADER_STYLE: &str = "display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; padding: 0.55rem 0.75rem; border-bottom: 0.0625rem solid #2e2e4a; background: #121224;";
const TITLE_STYLE: &str = "font-size: 0.88rem; color: #f3f4fb; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;";
const EMPTY_LIST: &str = "padding: 0.75rem; border: 0.0625rem dashed #2d2d4d; border-radius: 0.5rem; color: #7f7fa4; font-size: 0.82rem;";
const FILE_STYLE: &str = "display: block; width: 100%; text-align: left; padding: 0.45rem 0.55rem; background: transparent; color: #d4d4dc; border: 0.0625rem solid transparent; border-radius: 0.5rem; cursor: pointer; font-size: 0.84rem; margin-bottom: 0.2rem;";
const FILE_ACTIVE_STYLE: &str = "display: block; width: 100%; text-align: left; padding: 0.45rem 0.55rem; background: #1c1c34; color: #f2f3fd; border: 0.0625rem solid #34345c; border-radius: 0.5rem; cursor: pointer; font-size: 0.84rem; margin-bottom: 0.2rem;";
