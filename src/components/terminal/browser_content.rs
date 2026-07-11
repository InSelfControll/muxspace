use crate::browser;
use dioxus::prelude::*;

#[component]
pub fn BrowserContent(
    pane_id: String,
    initial_urls: Vec<String>,
    initial_active: usize,
    profile: Option<crate::browser::profile::BrowserProfile>,
) -> Element {
    let placeholder_id = format!("browser-ph-{pane_id}");
    {
        let pane_id = pane_id.clone();
        let ph_id = placeholder_id.clone();
        let urls = initial_urls.clone();
        let profile = profile.clone();
        use_coroutine(move |_: UnboundedReceiver<()>| async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let js = format!(
                    "(function(){{var el=document.getElementById('{ph_id}');if(!el){{dioxus.send(null);return;}}var r=el.getBoundingClientRect();dioxus.send({{viewportWidth:window.innerWidth,windowWidth:window.outerWidth,x:r.x,y:r.y,w:r.width,h:r.height}});}})()"
                );
                let mut eval = eval(&js);
                if let Ok(val) = eval.recv().await {
                    update_bounds(&pane_id, &val);
                }
                browser::with_manager_mut(|mgr| {
                    mgr.show_or_create(&pane_id, &urls, initial_active, profile.as_ref());
                });
            }
        });
    }
    rsx! {
        div { style: "flex: 1 1 auto; position: relative; display: block; width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; align-self: stretch;",
            div { id: "{placeholder_id}", style: "position: absolute; inset: 0; display: block; width: 100%; height: 100%; min-width: 0; min-height: 0; background: #0a0a14;" }
        }
    }
}

#[component]
pub fn BrowserTabBar(pane_id: String) -> Element {
    let mut tabs = use_signal(Vec::<(String, String)>::new);
    let mut active = use_signal(|| 0usize);
    {
        let pane_id = pane_id.clone();
        use_coroutine(move |_: UnboundedReceiver<()>| async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let info = browser::with_manager(|mgr| mgr.get_tabs_info(&pane_id)).flatten();
                if let Some((new_tabs, new_active)) = info {
                    tabs.set(
                        new_tabs
                            .into_iter()
                            .map(|tab| (tab.title, tab.url))
                            .collect(),
                    );
                    active.set(new_active);
                }
            }
        });
    }
    rsx! {
        div { style: "display: flex; align-items: center; background: #1a1a2e; border-bottom: 0.0625rem solid #2e2e4a; overflow-x: auto; flex-shrink: 0;",
            for (idx, (title, _url)) in tabs.read().iter().enumerate() {
                TabButton { pane_id: pane_id.clone(), idx, title: title.clone(), active: idx == *active.read(), tab_count: tabs.read().len() }
            }
            button { style: ADD_TAB, onclick: move |evt| { evt.stop_propagation(); with_mgr_mut(|mgr| mgr.add_tab(&pane_id, "about:blank")); }, "+" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct TabButtonProps {
    pane_id: String,
    idx: usize,
    title: String,
    active: bool,
    tab_count: usize,
}

#[component]
fn TabButton(props: TabButtonProps) -> Element {
    let label = if props.title.chars().count() > 22 {
        format!("{}...", props.title.chars().take(21).collect::<String>())
    } else {
        props.title.clone()
    };
    let switch_pane_id = props.pane_id.clone();
    let close_pane_id = props.pane_id.clone();
    rsx! {
        div {
            style: if props.active { TAB_ACTIVE } else { TAB },
            onclick: move |evt| {
                evt.stop_propagation();
                with_mgr_mut(|mgr| mgr.switch_tab(&switch_pane_id, props.idx));
            },
            span { style: "overflow: hidden; text-overflow: ellipsis;", "{label}" }
            if props.tab_count > 1 {
                button {
                    style: CLOSE_TAB,
                    onclick: move |evt| {
                        evt.stop_propagation();
                        with_mgr_mut(|mgr| mgr.close_tab(&close_pane_id, props.idx));
                    },
                    "x"
                }
            }
        }
    }
}

fn update_bounds(pane_id: &str, val: &serde_json::Value) {
    if val.is_null() {
        return;
    }
    let number = |key| val.get(key).and_then(serde_json::Value::as_f64);
    let raw = (
        number("viewportWidth").unwrap_or_default(),
        number("windowWidth").unwrap_or_default(),
        number("x").unwrap_or_default(),
        number("y").unwrap_or_default(),
        number("w").unwrap_or_default(),
        number("h").unwrap_or_default(),
    );
    let Some(bounds) =
        browser::geometry::normalize_browser_bounds(raw.0, raw.1, raw.2, raw.3, raw.4, raw.5)
    else {
        return;
    };
    tracing::debug!(pane_id, ?raw, ?bounds, "[Browser] Measured content bounds");
    browser::with_manager_mut(|mgr| {
        mgr.update_bounds(pane_id, bounds);
    });
}

fn with_mgr_mut(f: impl FnOnce(&mut crate::browser::BrowserManager)) {
    browser::with_manager_mut(f);
}

const TAB: &str = "display: flex; align-items: center; gap: 0.25rem; padding: 0.2rem 0.6rem; background: transparent; color: #666; border-right: 0.0625rem solid #2e2e4a; cursor: pointer; font-size: 0.75rem; max-width: 11.25rem; white-space: nowrap; overflow: hidden; flex-shrink: 0;";
const TAB_ACTIVE: &str = "display: flex; align-items: center; gap: 0.25rem; padding: 0.2rem 0.6rem; background: #2a2a4a; color: #eaeaf0; border-right: 0.0625rem solid #2e2e4a; cursor: pointer; font-size: 0.75rem; max-width: 11.25rem; white-space: nowrap; overflow: hidden; flex-shrink: 0;";
const CLOSE_TAB: &str = "padding: 0; margin-left: 0.2rem; background: transparent; border: none; color: #ff5555; cursor: pointer; font-size: 0.65rem; line-height: 1; opacity: 0.6;";
const ADD_TAB: &str = "padding: 0.2rem 0.5rem; background: transparent; border: none; color: #6366f1; cursor: pointer; font-size: 0.85rem; flex-shrink: 0;";
