use dioxus::prelude::*;

use crate::editor::{describe_launch_mode, InstalledEditor};

#[derive(Props, Clone, PartialEq)]
pub struct EditorSelectionModalProps {
    pub show: bool,
    #[props(!optional)]
    pub on_select: EventHandler<InstalledEditor>,
    #[props(!optional)]
    pub on_cancel: EventHandler<()>,
}

#[component]
pub fn EditorSelectionModal(props: EditorSelectionModalProps) -> Element {
    let editors = use_memo(crate::editor::detect_installed_editors)
        .read()
        .clone();

    if !props.show {
        return None;
    }

    let on_cancel = props.on_cancel;

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.75); z-index: 1000; display: flex; align-items: center; justify-content: center;",
            onclick: move |_| on_cancel.call(()),

            div {
                style: "background: #0f0f1a; border: 0.0625rem solid #1e1e3a; border-radius: 0.75rem; width: 32.5rem; padding: 1.75rem; \
                        box-shadow: 0 0.5rem 2rem rgba(0,0,0,0.6); max-height: 80vh; overflow-y: auto;",
                onclick: move |evt| evt.stop_propagation(),

                h2 {
                    style: "margin: 0 0 1.25rem 0; font-size: 1.15rem; color: #eaeaf0;",
                    "Select Code Editor"
                }

                p {
                    style: "color: #888; margin-bottom: 1.5rem;",
                    "Detected editors on this machine. Pick one and choose which folder should back this workspace pane."
                }

                if editors.is_empty() {
                    div {
                        style: "margin-bottom: 1.5rem; padding: 1rem; border: 0.0625rem solid #252540; border-radius: 0.5rem; background: #121224; color: #a0a0b0;",
                        "No supported editors were detected in PATH. Install Cursor, VS Code, T3 Code, OpenCode, Kate, Vim, or another supported editor and try again."
                    }
                } else {
                    div {
                        style: "margin-bottom: 1.5rem; display: grid; grid-template-columns: repeat(auto-fit, minmax(13.75rem, 1fr)); gap: 0.75rem;",

                        for editor in editors.iter() {
                            button {
                                style: "display: flex; width: 100%; padding: 0.9rem; background: linear-gradient(180deg, #1b1b31 0%, #171729 100%); \
                                        border: 0.0625rem solid #2b2b4d; border-radius: 0.625rem; color: #eaeaf0; text-align: left; cursor: pointer; \
                                        align-items: flex-start; gap: 0.75rem; min-height: 5.375rem;",
                                onclick: {
                                    let editor_clone = editor.clone();
                                    move |_| {
                                        props.on_select.call(editor_clone.clone());
                                    }
                                },

                                div {
                                    style: "width: 2.1rem; height: 2.1rem; border-radius: 0.5rem; background: #262648; display: flex; align-items: center; justify-content: center; font-size: 0.85rem; font-weight: 700; color: #b8b8ff; flex-shrink: 0;",
                                    "{editor.icon}"
                                }

                                div {
                                    style: "display: flex; flex-direction: column; gap: 0.18rem; min-width: 0;",

                                    span {
                                        style: "font-size: 0.96rem; font-weight: 600; color: #f2f2ff;",
                                        "{editor.name}"
                                    }

                                    span {
                                        style: "font-size: 0.78rem; color: #8d8db3; font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                        "{editor.executable}"
                                    }

                                    span {
                                        style: "font-size: 0.78rem; color: #6f6f94;",
                                        "{describe_launch_mode(&editor.launch_mode)}"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    style: "display: flex; gap: 0.5rem; justify-content: flex-end;",
                    button {
                        style: "padding: 0.5rem 1rem; background: transparent; color: #a0a0b0; \
                                border: 0.0625rem solid #1e1e3a; border-radius: 0.375rem; cursor: pointer; font-size: 0.85rem;",
                        onclick: move |_| props.on_cancel.call(()),
                        "Cancel"
                    }
                }
            }
        }
    }
}
