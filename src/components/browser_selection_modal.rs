use dioxus::prelude::*;

use crate::browser::selection::InstalledBrowser;

#[derive(Props, Clone, PartialEq)]
pub struct BrowserSelectionModalProps {
    pub show: bool,
    #[props(!optional)]
    pub on_select: EventHandler<InstalledBrowser>,
    #[props(!optional)]
    pub on_cancel: EventHandler<()>,
}

#[component]
pub fn BrowserSelectionModal(props: BrowserSelectionModalProps) -> Element {
    let browsers = use_memo(crate::browser::selection::detect_installed_browsers)
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
                    "Select Browser"
                }

                p {
                    style: "color: #888; margin-bottom: 1.5rem;",
                    "Choose which browser to use for new browser panes:"
                }

                div {
                    style: "margin-bottom: 1.5rem;",

                    for browser in browsers.iter() {
                        button {
                            style: "display: block; width: 100%; padding: 0.75rem; background: #1a1a2e; border: 0.0625rem solid #252540; \
                                    border-radius: 0.375rem; color: #eaeaf0; font-size: 0.95rem; text-align: left; margin-bottom: 0.5rem; \
                                    cursor: pointer;",
                            onclick: {
                                let browser_clone = browser.clone();
                                move |_| {
                                    props.on_select.call(browser_clone.clone());
                                }
                            },
                            "{browser.name}"
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
