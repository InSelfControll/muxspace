use dioxus::prelude::*;

mod hotkey_editor;
mod sidebar;
mod terminal;
mod project_nav;

pub use sidebar::sidebar as Sidebar;
use terminal::PaneView;
use hotkey_editor::{HotkeyEditorModal, ShortcutsHelpModal};

use crate::browser;
use crate::hotkeys::{Action, HotkeyConfig};
use crate::state::{AppState, Project};

/// Main App component
#[allow(non_snake_case)]
pub fn app() -> Element {
    use_context_provider(|| Signal::new(AppState::new_blocking()));
    use_context_provider(|| Signal::new(HotkeyConfig::load()));

    let mut state = use_context::<Signal<AppState>>();
    let hotkey_config = use_context::<Signal<HotkeyConfig>>();

    // Remove native window decorations.  The tao builder flag does not always
    // take effect, so we also poke GTK/GDK directly.  The GDK window may not
    // exist yet on the first render, so we schedule an idle callback that
    // retries until the window is realised.
    let window = dioxus_desktop::use_window();
    window.set_decorations(false);
    {
        use gtk::prelude::{Cast, GtkWindowExt, WidgetExt};
        glib::idle_add_local_once(move || {
            for toplevel in gtk::Window::list_toplevels() {
                if let Ok(win) = toplevel.downcast::<gtk::Window>() {
                    win.set_decorated(false);
                    if let Some(gw) = WidgetExt::window(&win) {
                        gw.set_decorations(gdk::WMDecoration::empty());
                    }
                }
            }
        });
    }

    // Track prefix mode
    let mut prefix_active = use_signal(|| false);

    // PTY polling coroutine
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;
            state.write().poll_pty_output();
        }
    });

    // Hide native browser WebViews while any modal is open so the
    // HTML-based modal isn't obscured by the GTK overlay.
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut prev_modal = false;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let s = state.read();
            let any_modal = s.show_create_project || s.show_hotkey_editor || s.show_shortcuts_help;
            drop(s);

            if any_modal != prev_modal {
                if let Ok(mgr) = browser::BROWSER_MGR.lock() {
                    if any_modal { mgr.hide_all(); } else { mgr.show_active_tabs(); }
                }
                prev_modal = any_modal;
            }
        }
    });

    rsx! {
        div {
            class: "app",
            style: "display: flex; height: 100vh; background: #0f0f1a; color: #eaeaf0; font-family: system-ui, -apple-system, sans-serif; outline: none;",
            tabindex: "0",

            // Global keyboard shortcut handler — uses configurable hotkeys
            onkeydown: move |evt: KeyboardEvent| {
                let ctrl = evt.modifiers().contains(Modifiers::CONTROL);
                let shift = evt.modifiers().contains(Modifiers::SHIFT);
                let alt = evt.modifiers().contains(Modifiers::ALT);

                let key_str = match evt.key() {
                    Key::Character(ref s) => s.clone(),
                    Key::ArrowUp => "ArrowUp".into(),
                    Key::ArrowDown => "ArrowDown".into(),
                    Key::ArrowLeft => "ArrowLeft".into(),
                    Key::ArrowRight => "ArrowRight".into(),
                    Key::Tab => "Tab".into(),
                    Key::Enter => "Enter".into(),
                    Key::Escape => "Escape".into(),
                    _ => return,
                };

                let config = hotkey_config.read();

                // Check if this is the prefix key
                if config.is_prefix_key(ctrl, alt, shift, &key_str) {
                    prefix_active.set(true);
                    return;
                }

                let is_prefix = *prefix_active.read();
                if is_prefix {
                    prefix_active.set(false);
                }

                // Try to match an action
                if let Some(action) = config.match_action(is_prefix, ctrl, alt, shift, &key_str) {
                    dispatch_action(action, &mut state);
                }
            },

            Sidebar {}
            MainContent {}
        }

        // Prefix mode indicator
        if *prefix_active.read() {
            div {
                style: "position: fixed; bottom: 1rem; right: 1rem; padding: 0.5rem 1rem; background: #6366f1; color: white; border-radius: 6px; font-size: 0.85rem; z-index: 2000; font-family: monospace;",
                "PREFIX  arrows=pane  n/p=ws  N/P=project  c=new ws  |=split  x=close"
            }
        }

        if state.read().show_create_project {
            CreateProjectModal {}
        }
        if state.read().show_hotkey_editor {
            HotkeyEditorModal {}
        }
        if state.read().show_shortcuts_help {
            ShortcutsHelpModal {}
        }
    }
}

#[component]
fn MainContent() -> Element {
    let state = use_context::<Signal<AppState>>();
    let state_read = state.read();
    let active_project = state_read.active_project();

    rsx! {
        div {
            class: "main-content",
            style: "flex: 1; display: flex; flex-direction: column; overflow: hidden;",

            Header {}

            div {
                class: "content-area",
                style: "flex: 1; display: flex; overflow: hidden;",

                if let Some(project) = active_project {
                    ProjectView { project: project.clone() }
                } else {
                    WelcomeView {}
                }
            }
        }
    }
}

#[component]
fn Header() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut active_menu = use_signal(|| Option::<String>::None);
    let window = dioxus_desktop::use_window();

    let menu_btn = "padding: 0.3rem 0.7rem; background: transparent; border: none; \
                    color: #a0a0b0; cursor: pointer; font-size: 0.85rem; border-radius: 4px;";
    let menu_btn_open = "padding: 0.3rem 0.7rem; background: #2a2a4a; border: none; \
                         color: #eaeaf0; cursor: pointer; font-size: 0.85rem; border-radius: 4px;";
    let item_style = "display: block; width: 100%; padding: 0.4rem 0.8rem; background: transparent; \
                      border: none; color: #eaeaf0; cursor: pointer; font-size: 0.82rem; text-align: left; \
                      white-space: nowrap;";
    let sep_style = "height: 1px; background: #2e2e4a; margin: 0.25rem 0;";

    let is_window = active_menu.read().as_deref() == Some("window");
    let is_edit = active_menu.read().as_deref() == Some("edit");
    let is_help = active_menu.read().as_deref() == Some("help");

    let mut close_menu = move || active_menu.set(None);

    rsx! {
        header {
            class: "header",
            style: "height: 40px; background: #16162a; border-bottom: 1px solid #2e2e4a; \
                    display: flex; align-items: center; padding: 0 0.5rem; position: relative; z-index: 900;",

            // ── Drag region — double-click to maximize ──
            onmousedown: {
                let w = window.clone();
                move |_| { w.drag(); }
            },
            ondoubleclick: {
                let w = window.clone();
                move |_| { w.toggle_maximized(); }
            },

            // ── Menu bar ──
            div {
                style: "display: flex; gap: 0.15rem; align-items: center;",
                onmousedown: |evt| evt.stop_propagation(),

                // Window menu
                div {
                    style: "position: relative;",

                    button {
                        style: if is_window { menu_btn_open } else { menu_btn },
                        onclick: move |_| {
                            if is_window { active_menu.set(None) } else { active_menu.set(Some("window".into())) }
                        },
                        "Window"
                    }

                    if is_window {
                        div {
                            style: "position: absolute; top: 100%; left: 0; background: #1e1e3a; \
                                    border: 1px solid #2e2e4a; border-radius: 6px; padding: 0.3rem 0; \
                                    min-width: 200px; z-index: 1000; box-shadow: 0 4px 12px rgba(0,0,0,0.4);",

                            button {
                                style: item_style,
                                onmouseenter: |_| {},
                                onclick: move |_| {
                                    state.write().add_terminal_pane(None);
                                    close_menu();
                                },
                                "New Terminal Pane          Prefix |"
                            }
                            button {
                                style: item_style,
                                onclick: move |_| {
                                    let name = format!("ws-{}", chrono::Utc::now().timestamp_millis() % 10000);
                                    state.write().add_workspace(&name);
                                    close_menu();
                                },
                            "New Workspace              Prefix c"
                            }
                            div { style: sep_style }
                            button {
                                style: item_style,
                                onclick: move |_| {
                                    let focused = state.read().focused_pane_id.clone();
                                    if let Some(id) = focused { state.write().remove_pane(&id); }
                                    close_menu();
                                },
                                "Close Pane                 Prefix x"
                            }
                            button {
                                style: item_style,
                                onclick: move |_| {
                                    state.write().remove_active_workspace();
                                    close_menu();
                                },
                                "Close Workspace"
                            }
                            div { style: sep_style }
                            button {
                                style: item_style,
                                onclick: move |_| { state.write().next_workspace(); close_menu(); },
                                "Next Workspace             Prefix n"
                            }
                            button {
                                style: item_style,
                                onclick: move |_| { state.write().prev_workspace(); close_menu(); },
                                "Previous Workspace         Prefix p"
                            }
                            div { style: sep_style }
                            button {
                                style: item_style,
                                onclick: move |_| { state.write().next_project(); close_menu(); },
                                "Next Project               Prefix N"
                            }
                            button {
                                style: item_style,
                                onclick: move |_| { state.write().prev_project(); close_menu(); },
                                "Previous Project           Prefix P"
                            }
                        }
                    }
                }

                // Edit menu
                div {
                    style: "position: relative;",

                    button {
                        style: if is_edit { menu_btn_open } else { menu_btn },
                        onclick: move |_| {
                            if is_edit { active_menu.set(None) } else { active_menu.set(Some("edit".into())) }
                        },
                        "Edit"
                    }

                    if is_edit {
                        div {
                            style: "position: absolute; top: 100%; left: 0; background: #1e1e3a; \
                                    border: 1px solid #2e2e4a; border-radius: 6px; padding: 0.3rem 0; \
                                    min-width: 200px; z-index: 1000; box-shadow: 0 4px 12px rgba(0,0,0,0.4);",

                            button {
                                style: item_style,
                                onclick: move |_| {
                                    state.write().show_hotkey_editor = true;
                                    close_menu();
                                },
                                "Configure Hotkeys..."
                            }
                        }
                    }
                }

                // Help menu
                div {
                    style: "position: relative;",

                    button {
                        style: if is_help { menu_btn_open } else { menu_btn },
                        onclick: move |_| {
                            if is_help { active_menu.set(None) } else { active_menu.set(Some("help".into())) }
                        },
                        "Help"
                    }

                    if is_help {
                        div {
                            style: "position: absolute; top: 100%; left: 0; background: #1e1e3a; \
                                    border: 1px solid #2e2e4a; border-radius: 6px; padding: 0.3rem 0; \
                                    min-width: 200px; z-index: 1000; box-shadow: 0 4px 12px rgba(0,0,0,0.4);",

                            button {
                                style: item_style,
                                onclick: move |_| {
                                    state.write().show_shortcuts_help = true;
                                    close_menu();
                                },
                                "Keyboard Shortcuts"
                            }
                        }
                    }
                }
            }

            // Spacer + New Project + Window controls
            div {
                style: "margin-left: auto; display: flex; gap: 0.4rem; align-items: center;",
                onmousedown: |evt| evt.stop_propagation(),

                button {
                    style: "padding: 0.35rem 0.75rem; background: #6366f1; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.8rem;",
                    onclick: move |_| {
                        state.write().show_create_project = true;
                    },
                    "+ New Project"
                }

            }
        }

        // Backdrop: close menu when clicking outside
        if active_menu.read().is_some() {
            div {
                style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 899;",
                onclick: move |_| active_menu.set(None),
            }
        }
    }
}

#[component]
fn ProjectView(project: Project) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let workspaces = project.workspaces.clone();
    let current_workspace = workspaces[project.active_workspace_idx].clone();
    let focused_pane_id = state.read().focused_pane_id.clone();
    let mut show_add_browser = use_signal(|| false);
    let mut browser_url_input = use_signal(|| String::from("http://localhost:3000"));
    let mut editing_project_name = use_signal(|| false);
    let mut project_name_input = use_signal(|| String::new());
    let mut editing_ws_idx = use_signal(|| Option::<usize>::None);
    let mut ws_name_input = use_signal(|| String::new());
    let project_name = project.name.clone();
    let project_name_for_edit = project_name.clone();
    let project_id_rename = project.id.clone();
    let project_id_rename2 = project.id.clone();
    let active_ws_idx = project.active_workspace_idx;

    rsx! {
        div {
            class: "project-view",
            style: "flex: 1; display: flex; flex-direction: column; padding: 1rem; overflow: hidden;",

            // Project title bar
            div {
                style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.75rem;",

                if *editing_project_name.read() {
                    input {
                        style: "margin: 0; font-size: 1.1rem; font-weight: bold; padding: 0.1rem 0.4rem; \
                                background: #0f0f1a; border: 1px solid #6366f1; border-radius: 4px; \
                                color: #eaeaf0; outline: none;",
                        r#type: "text",
                        value: "{project_name_input}",
                        autofocus: true,
                        oninput: move |evt| project_name_input.set(evt.value()),
                        onkeydown: move |evt: KeyboardEvent| {
                            if evt.key() == Key::Enter {
                                let new = project_name_input.read().trim().to_string();
                                if !new.is_empty() {
                                    state.write().rename_project(&project_id_rename, &new);
                                }
                                editing_project_name.set(false);
                            }
                            if evt.key() == Key::Escape {
                                editing_project_name.set(false);
                            }
                        },
                        onfocusout: move |_| {
                            let new = project_name_input.read().trim().to_string();
                            if !new.is_empty() {
                                state.write().rename_project(&project_id_rename2, &new);
                            }
                            editing_project_name.set(false);
                        },
                    }
                } else {
                    h2 {
                        style: "margin: 0; font-size: 1.1rem; cursor: default;",
                        ondoubleclick: move |_| {
                            project_name_input.set(project_name.clone());
                            editing_project_name.set(true);
                        },
                        "{project_name}"
                    }
                }

                // Rename project button
                if !*editing_project_name.read() {
                    button {
                        style: "padding: 0 0.3rem; background: transparent; border: none; \
                                color: #666; cursor: pointer; font-size: 0.8rem; opacity: 0.6; margin-left: 0.3rem;",
                        onclick: move |_| {
                            project_name_input.set(project_name_for_edit.clone());
                            editing_project_name.set(true);
                        },
                        "\u{270E}"
                    }
                }

                div {
                    style: "display: flex; gap: 0.5rem;",

                    button {
                        style: "padding: 0.4rem 0.8rem; background: #1e1e3a; color: #eaeaf0; border: 1px solid #2e2e4a; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                        onclick: move |_| {
                            state.write().add_terminal_pane(None);
                        },
                        "+ Terminal"
                    }

                    button {
                        style: if *show_add_browser.read() {
                            "padding: 0.4rem 0.8rem; background: #6366f1; color: white; border: 1px solid #6366f1; border-radius: 6px; cursor: pointer; font-size: 0.85rem;"
                        } else {
                            "padding: 0.4rem 0.8rem; background: #1e1e3a; color: #eaeaf0; border: 1px solid #2e2e4a; border-radius: 6px; cursor: pointer; font-size: 0.85rem;"
                        },
                        onclick: move |_| {
                            let v = *show_add_browser.read();
                            show_add_browser.set(!v);
                        },
                        "+ Browser"
                    }

                    button {
                        style: "padding: 0.4rem 0.8rem; background: #1e1e3a; color: #eaeaf0; border: 1px solid #2e2e4a; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                        onclick: move |_| {
                            let name = format!("ws-{}", chrono::Utc::now().timestamp_millis() % 10000);
                            state.write().add_workspace(&name);
                        },
                        "+ Workspace"
                    }
                }
            }

            // Add browser URL input (inline, not a modal)
            if *show_add_browser.read() {
                div {
                    style: "display: flex; gap: 0.5rem; margin-bottom: 0.75rem; align-items: center;",

                    input {
                        style: "flex: 1; padding: 0.4rem 0.6rem; background: #0f0f1a; border: 1px solid #2e2e4a; border-radius: 6px; color: #eaeaf0; font-size: 0.9rem; font-family: monospace;",
                        r#type: "text",
                        placeholder: "http://localhost:3000",
                        value: "{browser_url_input}",
                        oninput: move |evt| browser_url_input.set(evt.value()),
                        onkeydown: move |evt: KeyboardEvent| {
                            if evt.key() == Key::Enter {
                                let url = browser_url_input.read().clone();
                                if !url.is_empty() {
                                    state.write().add_browser_pane(url);
                                    browser_url_input.set("http://localhost:3000".to_string());
                                    show_add_browser.set(false);
                                }
                            }
                            if evt.key() == Key::Escape {
                                show_add_browser.set(false);
                            }
                        },
                    }

                    button {
                        style: "padding: 0.4rem 0.8rem; background: #6366f1; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                        onclick: move |_| {
                            let url = browser_url_input.read().clone();
                            if !url.is_empty() {
                                state.write().add_browser_pane(url);
                                browser_url_input.set("http://localhost:3000".to_string());
                                show_add_browser.set(false);
                            }
                        },
                        "Add"
                    }

                    button {
                        style: "padding: 0.4rem 0.6rem; background: transparent; color: #a0a0b0; border: 1px solid #2e2e4a; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                        onclick: move |_| show_add_browser.set(false),
                        "Cancel"
                    }
                }
            }

            // Workspace tabs
            div {
                class: "workspace-tabs",
                style: "display: flex; gap: 0.25rem; margin-bottom: 0.75rem; flex-wrap: wrap;",

                for (idx, workspace) in workspaces.iter().enumerate() {
                    {
                        let ws_name = workspace.name.clone();
                        let ws_name_for_edit = ws_name.clone();
                        let ws_id = workspace.id.clone();
                        rsx! {
                            div {
                                key: "{ws_id}",
                                style: "display: flex; align-items: center; gap: 0;",

                                if *editing_ws_idx.read() == Some(idx) {
                                    input {
                                        style: "padding: 0.3rem 0.5rem; background: #0f0f1a; border: 1px solid #6366f1; \
                                                border-radius: 6px; color: #eaeaf0; font-size: 0.85rem; outline: none; \
                                                width: 120px;",
                                        r#type: "text",
                                        value: "{ws_name_input}",
                                        autofocus: true,
                                        oninput: move |evt| ws_name_input.set(evt.value()),
                                        onkeydown: move |evt: KeyboardEvent| {
                                            if evt.key() == Key::Enter {
                                                let new = ws_name_input.read().trim().to_string();
                                                if !new.is_empty() {
                                                    state.write().rename_workspace(idx, &new);
                                                }
                                                editing_ws_idx.set(None);
                                            }
                                            if evt.key() == Key::Escape {
                                                editing_ws_idx.set(None);
                                            }
                                        },
                                        onfocusout: move |_| {
                                            let new = ws_name_input.read().trim().to_string();
                                            if !new.is_empty() {
                                                state.write().rename_workspace(idx, &new);
                                            }
                                            editing_ws_idx.set(None);
                                        },
                                    }
                                } else {
                                    button {
                                        style: if idx == active_ws_idx {
                                            "padding: 0.4rem 0.8rem; background: #6366f1; color: white; border: none; border-radius: 6px 0 0 6px; cursor: pointer; font-size: 0.85rem;"
                                        } else {
                                            "padding: 0.4rem 0.8rem; background: #1e1e3a; color: #a0a0b0; border: none; border-radius: 6px 0 0 6px; cursor: pointer; font-size: 0.85rem;"
                                        },
                                        onclick: move |_| {
                                            state.write().switch_workspace(idx);
                                        },
                                        ondoubleclick: {
                                            let ws_name = ws_name.clone();
                                            move |evt: MouseEvent| {
                                                evt.stop_propagation();
                                                ws_name_input.set(ws_name.clone());
                                                editing_ws_idx.set(Some(idx));
                                            }
                                        },
                                        if idx < 9 {
                                            "{idx + 1}. {ws_name}"
                                        } else {
                                            "{ws_name}"
                                        }
                                    }
                                }

                                // Rename workspace button
                                if *editing_ws_idx.read() != Some(idx) {
                                    button {
                                        style: if idx == active_ws_idx {
                                            "padding: 0.4rem 0.25rem; background: #6366f1; color: rgba(255,255,255,0.5); border: none; cursor: pointer; font-size: 0.65rem;"
                                        } else {
                                            "padding: 0.4rem 0.25rem; background: #1e1e3a; color: #555; border: none; cursor: pointer; font-size: 0.65rem;"
                                        },
                                        onclick: {
                                            let ws_name = ws_name_for_edit.clone();
                                            move |evt: MouseEvent| {
                                                evt.stop_propagation();
                                                ws_name_input.set(ws_name.clone());
                                                editing_ws_idx.set(Some(idx));
                                            }
                                        },
                                        "\u{270E}"
                                    }
                                }

                                if workspaces.len() > 1 && *editing_ws_idx.read() != Some(idx) {
                                    button {
                                        style: if idx == active_ws_idx {
                                            "padding: 0.4rem 0.4rem; background: #6366f1; color: rgba(255,255,255,0.6); border: none; border-radius: 0 6px 6px 0; cursor: pointer; font-size: 0.7rem;"
                                        } else {
                                            "padding: 0.4rem 0.4rem; background: #1e1e3a; color: #666; border: none; border-radius: 0 6px 6px 0; cursor: pointer; font-size: 0.7rem;"
                                        },
                                        onclick: move |_| {
                                            state.write().remove_workspace(idx);
                                        },
                                        "\u{00d7}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Panes grid
            div {
                class: "panes",
                style: "flex: 1; display: grid; grid-template-columns: repeat(auto-fit, minmax(400px, 1fr)); gap: 0.75rem; overflow: auto;",

                for pane in current_workspace.panes.iter() {
                    PaneView {
                        key: "{pane.id}",
                        pane: pane.clone(),
                        focused: focused_pane_id.as_ref() == Some(&pane.id),
                    }
                }
            }
        }
    }
}

#[component]
fn CreateProjectModal() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let mut name = use_signal(|| String::new());
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
            let path = std::path::PathBuf::from(&d);
            let project = state.write().create_project(&n, path);
            let project_id = project.id.clone();
            state.write().switch_project_blocking(&project_id);
            state.write().show_create_project = false;
        }
    };

    let on_cancel = move |_| {
        state.write().show_create_project = false;
    };

    let on_browse = move |_| {
        let mut dir = dir.clone();
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
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.75); z-index: 1000; display: flex; align-items: center; justify-content: center;",
            onclick: on_cancel,

            div {
                style: "background: #0f0f1a; border: 1px solid #1e1e3a; border-radius: 12px; width: 520px; padding: 1.75rem; \
                        box-shadow: 0 8px 32px rgba(0,0,0,0.6);",
                onclick: move |evt| evt.stop_propagation(),

                h2 {
                    style: "margin: 0 0 1.25rem 0; font-size: 1.15rem; color: #eaeaf0;",
                    "Create New Project"
                }

                div {
                    style: "margin-bottom: 1rem;",
                    label {
                        style: "display: block; margin-bottom: 0.4rem; color: #888; font-size: 0.8rem;",
                        "Project Name"
                    }
                    input {
                        style: "width: 100%; padding: 0.55rem 0.7rem; background: #1a1a2e; border: 1px solid #252540; \
                                border-radius: 6px; color: #eaeaf0; font-size: 0.95rem; box-sizing: border-box;",
                        r#type: "text",
                        placeholder: "My Project",
                        value: "{name}",
                        oninput: move |evt| name.set(evt.value()),
                    }
                }

                div {
                    style: "margin-bottom: 1.25rem;",
                    label {
                        style: "display: block; margin-bottom: 0.4rem; color: #888; font-size: 0.8rem;",
                        "Working Directory"
                    }
                    div {
                        style: "display: flex; gap: 0.5rem;",

                        input {
                            style: "flex: 1; padding: 0.55rem 0.7rem; background: #1a1a2e; border: 1px solid #252540; \
                                    border-radius: 6px; color: #eaeaf0; font-size: 0.95rem; box-sizing: border-box; \
                                    font-family: monospace;",
                            r#type: "text",
                            placeholder: "/home/user/projects/my-project",
                            value: "{dir}",
                            oninput: move |evt| dir.set(evt.value()),
                        }

                        button {
                            style: "padding: 0.55rem 0.9rem; background: #1e1e3a; color: #a0a0b0; \
                                    border: 1px solid #252540; border-radius: 6px; cursor: pointer; \
                                    font-size: 0.8rem; white-space: nowrap;",
                            onclick: on_browse,
                            "Browse..."
                        }
                    }
                }

                div {
                    style: "display: flex; gap: 0.5rem; justify-content: flex-end;",
                    button {
                        style: "padding: 0.5rem 1rem; background: transparent; color: #a0a0b0; \
                                border: 1px solid #1e1e3a; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                        onclick: on_cancel,
                        "Cancel"
                    }
                    button {
                        style: "padding: 0.5rem 1rem; background: #6366f1; color: white; border: none; \
                                border-radius: 6px; cursor: pointer; font-weight: 500; font-size: 0.85rem;",
                        onclick: on_create,
                        "Create Project"
                    }
                }
            }
        }
    }
}

#[component]
fn WelcomeView() -> Element {
    let mut state = use_context::<Signal<AppState>>();

    rsx! {
        div {
            class: "welcome-view",
            style: "flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 2rem; text-align: center;",

            h2 {
                style: "font-size: 2rem; margin-bottom: 1rem;",
                "Welcome to Muxspace"
            }

            p {
                style: "color: #a0a0b0; max-width: 500px; margin-bottom: 2rem;",
                "Select a project from the sidebar or create a new one to get started."
            }

            button {
                style: "padding: 0.75rem 1.5rem; background: #6366f1; color: white; border: none; border-radius: 8px; cursor: pointer; font-size: 1rem;",
                onclick: move |_| {
                    state.write().show_create_project = true;
                },
                "Create New Project"
            }

            div {
                style: "margin-top: 3rem; text-align: left; background: #16162a; padding: 1.5rem; border-radius: 8px; max-width: 600px;",

                h3 { style: "margin-top: 0;", "Keyboard Shortcuts" }

                div {
                    style: "color: #a0a0b0; font-family: monospace; font-size: 0.85rem; line-height: 2;",

                    div { "Ctrl+B, arrows    Move focus between panes" }
                    div { "Ctrl+B, n/p       Next/prev workspace" }
                    div { "Ctrl+B, 0-9       Jump to workspace" }
                    div { "Ctrl+B, c         New workspace" }
                    div { "Ctrl+B, |         New terminal pane" }
                    div { "Ctrl+B, x         Close focused pane" }
                    div { "Ctrl+B, N/P       Next/prev project" }
                    div { "Alt+1-9           Quick workspace switch" }
                    div { "Ctrl+Tab          Next workspace" }
                    div { "Ctrl+Shift+T      New workspace" }
                }

                p {
                    style: "margin-top: 1rem; color: #666; font-size: 0.8rem;",
                    "Edit ~/.config/muxspace/hotkeys.json to customize shortcuts"
                }
            }
        }
    }
}

/// Execute a hotkey action
fn dispatch_action(action: Action, state: &mut Signal<AppState>) {
    match action {
        Action::FocusNextPane => state.write().focus_next_pane(),
        Action::FocusPrevPane => state.write().focus_prev_pane(),
        Action::NextWorkspace => state.write().next_workspace(),
        Action::PrevWorkspace => state.write().prev_workspace(),
        Action::NewWorkspace => {
            let name = format!("ws-{}", chrono::Utc::now().timestamp_millis() % 10000);
            state.write().add_workspace(&name);
        }
        Action::NewTerminalPane => state.write().add_terminal_pane(None),
        Action::ClosePane => {
            let focused = state.read().focused_pane_id.clone();
            if let Some(pane_id) = focused {
                state.write().remove_pane(&pane_id);
            }
        }
        Action::NextProject => state.write().next_project(),
        Action::PrevProject => state.write().prev_project(),
        Action::GotoWorkspace0 => state.write().goto_workspace(0),
        Action::GotoWorkspace1 => state.write().goto_workspace(1),
        Action::GotoWorkspace2 => state.write().goto_workspace(2),
        Action::GotoWorkspace3 => state.write().goto_workspace(3),
        Action::GotoWorkspace4 => state.write().goto_workspace(4),
        Action::GotoWorkspace5 => state.write().goto_workspace(5),
        Action::GotoWorkspace6 => state.write().goto_workspace(6),
        Action::GotoWorkspace7 => state.write().goto_workspace(7),
        Action::GotoWorkspace8 => state.write().goto_workspace(8),
        Action::GotoWorkspace9 => state.write().goto_workspace(9),
    }
}
