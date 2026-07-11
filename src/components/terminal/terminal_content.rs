use super::screen_view::ScreenBufferView;
use crate::pty::PTY_MANAGER;
use crate::state::AppState;
use dioxus::prelude::*;

#[component]
pub fn TerminalContent(pane_id: String, pty_id: Option<String>, focused: bool) -> Element {
    let state = use_context::<Signal<AppState>>();
    let pty_id = pty_id.unwrap_or_else(|| pane_id.clone());
    let term_div_id = format!("term-{pane_id}");
    let screen = state.read().screen_buffers.get(&pty_id).cloned();
    if focused {
        eval(&format!(
            "setTimeout(function(){{var el=document.getElementById('{term_div_id}');if(el&&document.activeElement!==el&&document.activeElement.tagName!=='INPUT'&&document.activeElement.tagName!=='TEXTAREA')el.focus();}},30)"
        ));
    }
    rsx! {
        div {
            id: "{term_div_id}",
            class: "terminal-screen",
            style: "flex: 1; overflow-y: auto; padding: 0.125rem; font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Fira Code', monospace; font-size: 0.8125rem; line-height: 1.2; background: #0a0a14; color: #eaeaf0; cursor: text; white-space: pre; outline: none;",
            tabindex: "0",
            prevent_default: "onkeydown",
            onkeydown: move |evt: KeyboardEvent| {
                let bytes = key_to_bytes(&evt);
                if !bytes.is_empty() {
                    if let Ok(mgr) = PTY_MANAGER.lock() {
                        let _ = mgr.write_to_pane(&pty_id, &bytes);
                    }
                }
            },
            if let Some(screen) = screen {
                ScreenBufferView { screen }
            } else {
                span { style: "color: #666;", "Starting terminal..." }
            }
        }
    }
}

fn key_to_bytes(evt: &KeyboardEvent) -> Vec<u8> {
    let ctrl = evt.modifiers().contains(Modifiers::CONTROL);
    match evt.key() {
        Key::Enter => vec![b'\r'],
        Key::Backspace => vec![0x7f],
        Key::Tab => vec![b'\t'],
        Key::Escape => vec![0x1b],
        Key::ArrowUp => vec![0x1b, b'[', b'A'],
        Key::ArrowDown => vec![0x1b, b'[', b'B'],
        Key::ArrowRight => vec![0x1b, b'[', b'C'],
        Key::ArrowLeft => vec![0x1b, b'[', b'D'],
        Key::Home => vec![0x1b, b'[', b'H'],
        Key::End => vec![0x1b, b'[', b'F'],
        Key::Delete => vec![0x1b, b'[', b'3', b'~'],
        Key::Character(s) if ctrl && s.len() == 1 && s.as_bytes()[0].is_ascii_alphabetic() => {
            vec![s.as_bytes()[0].to_ascii_lowercase() - b'a' + 1]
        }
        Key::Character(s) => s.as_bytes().to_vec(),
        _ => vec![],
    }
}
