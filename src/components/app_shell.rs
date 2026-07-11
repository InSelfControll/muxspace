use crate::browser;
use crate::components::create_project::CreateProjectModal;
use crate::components::welcome::WelcomeView;
use crate::components::Sidebar;
use crate::hotkeys::{Action, HotkeyConfig};
use crate::state::AppState;
use dioxus::prelude::*;

#[allow(non_snake_case)]
pub fn app() -> Element {
    use_context_provider(|| Signal::new(AppState::new_blocking()));
    use_context_provider(|| Signal::new(HotkeyConfig::load()));

    let mut state = use_context::<Signal<AppState>>();
    let hotkey_config = use_context::<Signal<HotkeyConfig>>();
    let window = dioxus_desktop::use_window();
    window.set_decorations(false);
    remove_gtk_decorations();

    let mut prefix_active = use_signal(|| false);
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;
            state.write().poll_pty_output();
        }
    });
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        let mut prev_modal = false;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let s = state.read();
            let modal = s.show_create_project || s.show_hotkey_editor || s.show_shortcuts_help;
            drop(s);
            if modal != prev_modal {
                browser::with_manager(|mgr| {
                    if modal {
                        mgr.hide_all();
                    } else {
                        mgr.show_active_tabs();
                    }
                });
                prev_modal = modal;
            }
        }
    });

    rsx! {
        div {
            class: "app",
            style: "background: #0f0f1a; color: #eaeaf0; font-family: system-ui, -apple-system, sans-serif; outline: none;",
            tabindex: "0",
            onkeydown: move |evt: KeyboardEvent| {
                handle_key(evt, &hotkey_config.read(), &mut prefix_active, &mut state);
            },
            Sidebar {}
            MainContent {}
        }
        if *prefix_active.read() {
            div {
                style: "position: fixed; bottom: 1rem; right: 1rem; padding: 0.5rem 1rem; background: #6366f1; color: white; border-radius: 0.375rem; font-size: 0.85rem; z-index: 2000; font-family: monospace;",
                "PREFIX  arrows=pane  n/p=ws  N/P=project  c=new ws  |=split  x=close"
            }
        }
        if state.read().show_create_project {
            CreateProjectModal {}
        }
        if state.read().show_hotkey_editor {
            crate::components::hotkey_editor::HotkeyEditorModal {}
        }
        if state.read().show_shortcuts_help {
            crate::components::hotkey_editor::ShortcutsHelpModal {}
        }
    }
}

#[component]
fn MainContent() -> Element {
    let state = use_context::<Signal<AppState>>();
    let state_read = state.read();
    let active_project = state_read.active_project();
    rsx! {
        main { class: "main-content", style: "display: flex; flex-direction: column; overflow: hidden;",
            div { class: "content-area", style: "flex: 1; display: flex; overflow: hidden;",
                if let Some(project) = active_project {
                    crate::components::project_view::ProjectView { project: project.clone() }
                } else {
                    WelcomeView {}
                }
            }
        }
    }
}

fn handle_key(
    evt: KeyboardEvent,
    config: &HotkeyConfig,
    prefix_active: &mut Signal<bool>,
    state: &mut Signal<AppState>,
) {
    let ctrl = evt.modifiers().contains(Modifiers::CONTROL);
    let shift = evt.modifiers().contains(Modifiers::SHIFT);
    let alt = evt.modifiers().contains(Modifiers::ALT);
    let Some(key) = key_name(evt.key()) else {
        return;
    };
    if config.is_prefix_key(ctrl, alt, shift, &key) {
        prefix_active.set(true);
        return;
    }
    let is_prefix = *prefix_active.read();
    if is_prefix {
        prefix_active.set(false);
    }
    if let Some(action) = config.match_action(is_prefix, ctrl, alt, shift, &key) {
        dispatch_action(action, state);
    }
}

fn key_name(key: Key) -> Option<String> {
    Some(match key {
        Key::Character(s) => s,
        Key::ArrowUp => "ArrowUp".into(),
        Key::ArrowDown => "ArrowDown".into(),
        Key::ArrowLeft => "ArrowLeft".into(),
        Key::ArrowRight => "ArrowRight".into(),
        Key::Tab => "Tab".into(),
        Key::Enter => "Enter".into(),
        Key::Escape => "Escape".into(),
        _ => return None,
    })
}

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

fn remove_gtk_decorations() {
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
