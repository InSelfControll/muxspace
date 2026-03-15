use dioxus::prelude::*;

use crate::browser;
use crate::pty::{PTY_MANAGER, ScreenBuffer};
use crate::state::{AppState, Pane, PaneKind};

/// A single pane — either a terminal or an embedded browser
#[component]
pub fn PaneView(pane: Pane, focused: bool) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let pane_id_focus = pane.id.clone();
    let pane_id_close = pane.id.clone();
    let pane_id_rename = pane.id.clone();
    let pane_id_rename2 = pane.id.clone();
    let is_browser = matches!(pane.kind, PaneKind::Browser { .. });
    let custom_name = pane.custom_name.clone();
    let pty_id_for_title = pane.pty_id.clone().unwrap_or_else(|| pane.id.clone());

    let mut editing_name = use_signal(|| false);
    let mut name_input = use_signal(|| String::new());

    // Resolve display title: custom_name > OSC title > "Terminal"
    let display_title = if let Some(ref name) = custom_name {
        name.clone()
    } else {
        let osc_title = state.read().screen_buffers.get(&pty_id_for_title)
            .map(|b| b.title.clone())
            .unwrap_or_default();
        if !osc_title.is_empty() {
            osc_title
        } else {
            match &pane.kind {
                PaneKind::Terminal { command: Some(cmd) } => format!("Terminal: {cmd}"),
                _ => "Terminal".to_string(),
            }
        }
    };
    let display_title_for_rename = display_title.clone();

    let border_color = if focused { "#6366f1" } else { "#2e2e4a" };

    rsx! {
        div {
            class: "pane-container",
            style: "background: #16162a; border: 2px solid {border_color}; border-radius: 8px; display: flex; flex-direction: column; overflow: hidden; height: 100%; min-height: 300px;",

            onclick: move |_| {
                state.write().focused_pane_id = Some(pane_id_focus.clone());
            },

            // Pane header
            div {
                class: "pane-header",
                style: "padding: 0.35rem 0.75rem; background: #1e1e3a; border-bottom: 1px solid #2e2e4a; display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;",

                if !is_browser {
                    if *editing_name.read() {
                        input {
                            style: "font-weight: 500; font-size: 0.85rem; padding: 0.1rem 0.3rem; \
                                    background: #0f0f1a; border: 1px solid #6366f1; border-radius: 4px; \
                                    color: #eaeaf0; outline: none; width: 180px;",
                            r#type: "text",
                            value: "{name_input}",
                            autofocus: true,
                            onclick: |evt| evt.stop_propagation(),
                            oninput: move |evt| name_input.set(evt.value()),
                            onkeydown: move |evt: KeyboardEvent| {
                                if evt.key() == Key::Enter {
                                    let new = name_input.read().trim().to_string();
                                    state.write().rename_pane(&pane_id_rename, &new);
                                    editing_name.set(false);
                                }
                                if evt.key() == Key::Escape {
                                    editing_name.set(false);
                                }
                            },
                            onfocusout: move |_| {
                                let new = name_input.read().trim().to_string();
                                state.write().rename_pane(&pane_id_rename2, &new);
                                editing_name.set(false);
                            },
                        }
                    } else {
                        span {
                            style: "font-weight: 500; font-size: 0.85rem; cursor: default; overflow: hidden; \
                                    text-overflow: ellipsis; white-space: nowrap; max-width: 300px;",
                            ondoubleclick: move |evt| {
                                evt.stop_propagation();
                                name_input.set(display_title.clone());
                                editing_name.set(true);
                            },
                            "{display_title}"
                        }
                    }
                }

                // Rename button for terminal panes
                if !is_browser && !*editing_name.read() {
                    button {
                        style: "padding: 0 0.3rem; background: transparent; border: none; \
                                color: #666; cursor: pointer; font-size: 0.75rem; opacity: 0.6; flex-shrink: 0;",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            name_input.set(display_title_for_rename.clone());
                            editing_name.set(true);
                        },
                        "\u{270E}"
                    }
                }

                if is_browser {
                    BrowserNavBar { pane_id: pane.id.clone() }
                }

                // Focus indicator + close button — flex-shrink: 0 keeps it visible
                div {
                    style: "display: flex; align-items: center; gap: 0.5rem; flex-shrink: 0; margin-left: 0.5rem;",

                    if focused {
                        span {
                            style: "width: 8px; height: 8px; background: #6366f1; border-radius: 50%; display: inline-block;",
                        }
                    }

                    button {
                        style: "padding: 0.15rem 0.4rem; background: transparent; border: none; color: #ff5555; cursor: pointer; font-size: 0.95rem; opacity: 0.8; line-height: 1;",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            state.write().remove_pane(&pane_id_close);
                        },
                        "x"
                    }
                }
            }

            // Tab bar (browsers only)
            if is_browser {
                BrowserTabBar { pane_id: pane.id.clone() }
            }

            // Pane content
            match &pane.kind {
                PaneKind::Terminal { .. } => {
                    rsx! { TerminalContent { pane_id: pane.id.clone(), pty_id: pane.pty_id.clone(), focused: focused } }
                }
                PaneKind::Browser { tabs, active_tab, .. } => {
                    rsx! { BrowserContent { pane_id: pane.id.clone(), initial_urls: tabs.clone(), initial_active: *active_tab } }
                }
            }
        }
    }
}

/// Browser navigation bar — back/forward/reload buttons + editable URL bar
#[component]
fn BrowserNavBar(pane_id: String) -> Element {
    let mut url_input = use_signal(|| String::new());
    let mut initialized = use_signal(|| false);
    let mut is_editing = use_signal(|| false);

    // Sync current URL from native WebView periodically (only when not editing)
    {
        let pane_id = pane_id.clone();
        use_coroutine(move |_: UnboundedReceiver<()>| {
            let pane_id = pane_id.clone();
            async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if *is_editing.read() {
                        continue;
                    }
                    let current = browser::BROWSER_MGR.lock().ok()
                        .and_then(|mgr| mgr.get_url(&pane_id));
                    if let Some(u) = current {
                        if !u.is_empty() && u != *url_input.read() {
                            url_input.set(u);
                            if !*initialized.read() {
                                initialized.set(true);
                            }
                        }
                    }
                }
            }
        });
    }

    let nav_btn = "padding: 0.15rem 0.4rem; background: #2a2a4a; border: 1px solid #3e3e5a; color: #ccc; cursor: pointer; font-size: 0.8rem; border-radius: 4px; line-height: 1;";
    let nav_btn_disabled = "padding: 0.15rem 0.4rem; background: #1a1a2a; border: 1px solid #2e2e3a; color: #555; cursor: default; font-size: 0.8rem; border-radius: 4px; line-height: 1;";

    let can_back = browser::BROWSER_MGR.lock().ok()
        .map_or(false, |mgr| mgr.can_go_back(&pane_id));
    let can_fwd = browser::BROWSER_MGR.lock().ok()
        .map_or(false, |mgr| mgr.can_go_forward(&pane_id));

    let pane_id_back = pane_id.clone();
    let pane_id_fwd = pane_id.clone();
    let pane_id_reload = pane_id.clone();
    let pane_id_go = pane_id.clone();

    rsx! {
        div {
            style: "display: flex; align-items: center; gap: 0.3rem; flex: 1; min-width: 0;",

            button {
                style: if can_back { nav_btn } else { nav_btn_disabled },
                disabled: !can_back,
                onclick: move |evt| {
                    evt.stop_propagation();
                    if let Ok(mgr) = browser::BROWSER_MGR.lock() {
                        mgr.go_back(&pane_id_back);
                    }
                },
                "<"
            }

            button {
                style: if can_fwd { nav_btn } else { nav_btn_disabled },
                disabled: !can_fwd,
                onclick: move |evt| {
                    evt.stop_propagation();
                    if let Ok(mgr) = browser::BROWSER_MGR.lock() {
                        mgr.go_forward(&pane_id_fwd);
                    }
                },
                ">"
            }

            button {
                style: nav_btn,
                onclick: move |evt| {
                    evt.stop_propagation();
                    if let Ok(mgr) = browser::BROWSER_MGR.lock() {
                        mgr.reload(&pane_id_reload);
                    }
                },
                "R"
            }

            input {
                style: "flex: 1; min-width: 0; padding: 0.15rem 0.4rem; background: #0a0a14; border: 1px solid #3e3e5a; color: #eaeaf0; font-size: 0.8rem; border-radius: 4px; outline: none; font-family: inherit;",
                r#type: "text",
                value: "{url_input}",
                onfocus: move |_| is_editing.set(true),
                onblur: move |_| is_editing.set(false),
                oninput: move |evt| {
                    url_input.set(evt.value().clone());
                },
                onkeydown: {
                    let pane_id_nav = pane_id_go.clone();
                    move |evt: KeyboardEvent| {
                        if evt.key() == Key::Enter {
                            let mut url = url_input.read().clone();
                            if !url.starts_with("http://") && !url.starts_with("https://") {
                                url = format!("https://{url}");
                            }
                            if let Ok(mgr) = browser::BROWSER_MGR.lock() {
                                mgr.navigate(&pane_id_nav, &url);
                            }
                            is_editing.set(false);
                        }
                        if evt.key() == Key::Escape {
                            is_editing.set(false);
                        }
                    }
                },
            }
        }
    }
}

/// Terminal pane content with PTY I/O
#[component]
fn TerminalContent(pane_id: String, pty_id: Option<String>, focused: bool) -> Element {
    let state = use_context::<Signal<AppState>>();
    let pty_id = pty_id.unwrap_or_else(|| pane_id.clone());
    let term_div_id = format!("term-{}", pane_id);

    let screen = state.read().screen_buffers.get(&pty_id).cloned();

    // Auto-focus the terminal div when this pane is focused.
    // Skip if user is typing in an input (e.g. rename field) so we don't steal focus.
    if focused {
        let focus_id = term_div_id.clone();
        eval(&format!(
            "setTimeout(function(){{ var el=document.getElementById('{}'); \
             if(el && document.activeElement!==el && \
             document.activeElement.tagName!=='INPUT' && \
             document.activeElement.tagName!=='TEXTAREA') el.focus(); }}, 30)",
            focus_id
        ));
    }

    rsx! {
        div {
            id: "{term_div_id}",
            class: "terminal-screen",
            style: "flex: 1; overflow-y: auto; padding: 2px; font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Fira Code', monospace; font-size: 13px; line-height: 1.2; background: #0a0a14; color: #eaeaf0; cursor: text; white-space: pre; outline: none;",
            tabindex: "0",
            prevent_default: "onkeydown",

            onkeydown: {
                let pty_id = pty_id.clone();
                move |evt: KeyboardEvent| {
                    let bytes = key_to_bytes(&evt);
                    if !bytes.is_empty() {
                        if let Ok(mgr) = PTY_MANAGER.lock() {
                            let _ = mgr.write_to_pane(&pty_id, &bytes);
                        }
                    }
                }
            },

            if let Some(screen) = screen {
                ScreenBufferView { screen }
            } else {
                span {
                    style: "color: #666;",
                    "Starting terminal..."
                }
            }
        }
    }
}

/// Browser tab strip — shows open tabs with close buttons and a "+" to add.
/// Polls `BROWSER_MGR` for tab info so tab operations don't trigger Dioxus
/// state writes (avoids the URL-bar-regeneration problem).
#[component]
fn BrowserTabBar(pane_id: String) -> Element {
    let mut tabs_info = use_signal(|| Vec::<(String, String)>::new()); // (title, url)
    let mut active_tab = use_signal(|| 0usize);

    // Poll tab info from BROWSER_MGR
    {
        let pane_id = pane_id.clone();
        use_coroutine(move |_: UnboundedReceiver<()>| {
            let pane_id = pane_id.clone();
            async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let info = browser::BROWSER_MGR
                        .lock()
                        .ok()
                        .and_then(|mgr| mgr.get_tabs_info(&pane_id));
                    if let Some((tabs, active)) = info {
                        let new_info: Vec<(String, String)> = tabs
                            .into_iter()
                            .map(|t| (t.title, t.url))
                            .collect();
                        tabs_info.set(new_info);
                        active_tab.set(active);
                    }
                }
            }
        });
    }

    let pane_id_add = pane_id.clone();
    let tab_count = tabs_info.read().len();
    let current_active = *active_tab.read();

    let tab_style_active = "display: flex; align-items: center; gap: 0.25rem; \
        padding: 0.2rem 0.6rem; background: #2a2a4a; color: #eaeaf0; \
        border-right: 1px solid #2e2e4a; cursor: pointer; font-size: 0.75rem; \
        max-width: 180px; white-space: nowrap; overflow: hidden; flex-shrink: 0;";
    let tab_style_inactive = "display: flex; align-items: center; gap: 0.25rem; \
        padding: 0.2rem 0.6rem; background: transparent; color: #666; \
        border-right: 1px solid #2e2e4a; cursor: pointer; font-size: 0.75rem; \
        max-width: 180px; white-space: nowrap; overflow: hidden; flex-shrink: 0;";

    rsx! {
        div {
            style: "display: flex; align-items: center; background: #1a1a2e; \
                    border-bottom: 1px solid #2e2e4a; overflow-x: auto; flex-shrink: 0;",

            for (idx , (title , _url)) in tabs_info.read().iter().enumerate() {
                {
                    let pane_id_switch = pane_id.clone();
                    let pane_id_close = pane_id.clone();
                    let is_active = idx == current_active;
                    let label = if title.len() > 22 {
                        format!("{}\u{2026}", &title[..21])
                    } else {
                        title.clone()
                    };

                    rsx! {
                        div {
                            key: "{idx}",
                            style: if is_active { tab_style_active } else { tab_style_inactive },
                            onclick: move |evt| {
                                evt.stop_propagation();
                                if let Ok(mut mgr) = browser::BROWSER_MGR.lock() {
                                    mgr.switch_tab(&pane_id_switch, idx);
                                }
                            },

                            span {
                                style: "overflow: hidden; text-overflow: ellipsis;",
                                "{label}"
                            }

                            if tab_count > 1 {
                                button {
                                    style: "padding: 0; margin-left: 0.2rem; background: transparent; \
                                            border: none; color: #ff5555; cursor: pointer; \
                                            font-size: 0.65rem; line-height: 1; opacity: 0.6;",
                                    onclick: move |evt| {
                                        evt.stop_propagation();
                                        if let Ok(mut mgr) = browser::BROWSER_MGR.lock() {
                                            mgr.close_tab(&pane_id_close, idx);
                                        }
                                    },
                                    "x"
                                }
                            }
                        }
                    }
                }
            }

            // Add-tab button
            button {
                style: "padding: 0.2rem 0.5rem; background: transparent; border: none; \
                        color: #6366f1; cursor: pointer; font-size: 0.85rem; flex-shrink: 0;",
                onclick: move |evt| {
                    evt.stop_propagation();
                    if let Ok(mut mgr) = browser::BROWSER_MGR.lock() {
                        mgr.add_tab(&pane_id_add, "about:blank");
                    }
                },
                "+"
            }
        }
    }
}

/// Embedded browser pane — a real WebKitWebView overlaid on the Dioxus UI.
/// The native WebView covers only the placeholder area.
#[component]
fn BrowserContent(pane_id: String, initial_urls: Vec<String>, initial_active: usize) -> Element {
    let placeholder_id = format!("browser-ph-{}", pane_id);

    // Show existing browser or create a new one on mount.
    // show_or_create keeps the current page when returning from a workspace switch.
    use_hook({
        let pid = pane_id.clone();
        let urls = initial_urls.clone();
        move || {
            browser::BROWSER_MGR.lock().unwrap().show_or_create(&pid, &urls, initial_active);
        }
    });

    // Coroutine: measure placeholder position, reposition native WebView
    {
        let pane_id = pane_id.clone();
        let ph_id = placeholder_id.clone();
        use_coroutine(move |_: UnboundedReceiver<()>| {
            let pane_id = pane_id.clone();
            let ph_id = ph_id.clone();
            async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

                    let js = format!(
                        r#"(function() {{
                            var el = document.getElementById('{ph_id}');
                            if (!el) {{ dioxus.send(null); return; }}
                            var r = el.getBoundingClientRect();
                            dioxus.send({{x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height)}});
                        }})()"#
                    );

                    let mut e = eval(&js);
                    if let Ok(val) = e.recv().await {
                        if val.is_null() {
                            continue;
                        }
                        let x = val.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let y = val.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let w = val.get("w").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let h = val.get("h").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

                        if w > 20 && h > 20 {
                            let mgr = browser::BROWSER_MGR.lock().unwrap();
                            mgr.update_bounds(&pane_id, x, y, w, h);
                        }
                    }
                }
            }
        });
    }


    rsx! {
        div {
            style: "flex: 1; position: relative; min-height: 0;",
            div {
                id: "{placeholder_id}",
                style: "position: absolute; top: 0; left: 0; right: 0; bottom: 0; background: #0a0a14;",
            }
        }
    }
}

/// Convert a Dioxus KeyboardEvent into raw bytes to send to the PTY
fn key_to_bytes(evt: &KeyboardEvent) -> Vec<u8> {
    let key = evt.key();
    let ctrl = evt.modifiers().contains(Modifiers::CONTROL);

    match key {
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
        Key::Character(ref s) => {
            if ctrl && s.len() == 1 {
                let c = s.as_bytes()[0];
                if c.is_ascii_alphabetic() {
                    vec![c.to_ascii_lowercase() - b'a' + 1]
                } else {
                    vec![]
                }
            } else {
                s.as_bytes().to_vec()
            }
        }
        _ => vec![],
    }
}

#[component]
fn ScreenBufferView(screen: ScreenBuffer) -> Element {
    let fg_color = |code: u8| -> &'static str {
        match code {
            0 => "#eaeaf0",
            1 => "#ff5555",
            2 => "#50fa7b",
            3 => "#f1fa8c",
            4 => "#6366f1",
            5 => "#ff79c6",
            6 => "#8be9fd",
            7 => "#bbbbbb",
            8 => "#666666",
            9 => "#ff6e6e",
            10 => "#69ff94",
            11 => "#ffffa5",
            12 => "#8b8ff1",
            13 => "#ff92df",
            14 => "#a4ffff",
            15 => "#ffffff",
            _ => "#eaeaf0",
        }
    };

    rsx! {
        pre {
            style: "margin: 0; white-space: pre; line-height: 1.2;",

            for (row_idx, row) in screen.grid.iter().enumerate() {
                div {
                    key: "{row_idx}",
                    style: "min-height: 1.2em; display: flex;",

                    for (col_idx, cell) in row.iter().enumerate() {
                        {
                            let is_cursor = row_idx == screen.cursor_row && col_idx == screen.cursor_col;
                            let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
                            let color = fg_color(cell.fg);
                            let bold = if cell.bold { "font-weight: bold;" } else { "" };

                            if is_cursor {
                                rsx! {
                                    span {
                                        style: "background: #6366f1; color: #0a0a14; {bold}",
                                        "{ch}"
                                    }
                                }
                            } else if cell.fg != 0 || cell.bold {
                                rsx! {
                                    span {
                                        style: "color: {color}; {bold}",
                                        "{ch}"
                                    }
                                }
                            } else {
                                rsx! {
                                    span { "{ch}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
