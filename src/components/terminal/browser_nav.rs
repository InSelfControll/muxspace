use crate::browser;
use dioxus::prelude::*;

#[component]
pub fn BrowserNavBar(pane_id: String) -> Element {
    let mut url_input = use_signal(String::new);
    let mut is_editing = use_signal(|| false);
    {
        let pane_id = pane_id.clone();
        use_coroutine(move |_: UnboundedReceiver<()>| async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if *is_editing.read() {
                    continue;
                }
                let current = browser::with_manager(|mgr| mgr.get_url(&pane_id)).flatten();
                if let Some(url) = current {
                    if !url.is_empty() && url != *url_input.read() {
                        url_input.set(url);
                    }
                }
            }
        });
    }
    let can_back = browser::with_manager(|mgr| mgr.can_go_back(&pane_id)).unwrap_or(false);
    let can_fwd = browser::with_manager(|mgr| mgr.can_go_forward(&pane_id)).unwrap_or(false);
    let back_pane_id = pane_id.clone();
    let forward_pane_id = pane_id.clone();
    let reload_pane_id = pane_id.clone();
    let navigate_pane_id = pane_id.clone();

    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.3rem; flex: 1; min-width: 0;",
            NavButton { label: "<".to_string(), disabled: !can_back, on_click: move |_| with_mgr(|mgr| mgr.go_back(&back_pane_id)) }
            NavButton { label: ">".to_string(), disabled: !can_fwd, on_click: move |_| with_mgr(|mgr| mgr.go_forward(&forward_pane_id)) }
            NavButton { label: "R".to_string(), disabled: false, on_click: move |_| with_mgr(|mgr| mgr.reload(&reload_pane_id)) }
            input {
                style: "flex: 1; min-width: 0; padding: 0.15rem 0.4rem; background: #0a0a14; border: 0.0625rem solid #3e3e5a; color: #eaeaf0; font-size: 0.8rem; border-radius: 0.25rem; outline: none; font-family: inherit;",
                value: "{url_input}",
                onfocus: move |_| is_editing.set(true),
                onblur: move |_| is_editing.set(false),
                oninput: move |evt| url_input.set(evt.value()),
                onkeydown: move |evt: KeyboardEvent| {
                    if evt.key() == Key::Enter {
                        let url = normalize_url(&url_input.read());
                        with_mgr(|mgr| mgr.navigate(&navigate_pane_id, &url));
                        is_editing.set(false);
                    }
                    if evt.key() == Key::Escape { is_editing.set(false); }
                },
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct NavButtonProps {
    label: String,
    disabled: bool,
    on_click: EventHandler<()>,
}

#[component]
fn NavButton(props: NavButtonProps) -> Element {
    rsx! {
        button {
            style: if props.disabled { NAV_DISABLED } else { NAV },
            disabled: props.disabled,
            onclick: move |evt| {
                evt.stop_propagation();
                props.on_click.call(());
            },
            "{props.label}"
        }
    }
}

fn with_mgr(f: impl FnOnce(&crate::browser::BrowserManager)) {
    browser::with_manager(f);
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

const NAV: &str = "padding: 0.15rem 0.4rem; background: #2a2a4a; border: 0.0625rem solid #3e3e5a; color: #ccc; cursor: pointer; font-size: 0.8rem; border-radius: 0.25rem; line-height: 1;";
const NAV_DISABLED: &str = "padding: 0.15rem 0.4rem; background: #1a1a2a; border: 0.0625rem solid #2e2e3a; color: #555; cursor: default; font-size: 0.8rem; border-radius: 0.25rem; line-height: 1;";
