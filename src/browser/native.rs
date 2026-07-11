use crate::browser::geometry::BrowserBounds;
use crate::browser::profile::BrowserProfile;
use crate::browser::TabInfo;
use gtk::prelude::*;
use std::collections::HashMap;
use webkit2gtk::{WebContext, WebView, WebViewExt};

pub struct BrowserManager {
    pub(super) overlay: Option<gtk::Overlay>,
    pub(super) browsers: HashMap<String, EmbeddedBrowser>,
    pub(super) initialized: bool,
    pub(super) web_context: Option<WebContext>,
    pub(super) bounds: HashMap<String, BrowserBounds>,
}

pub(super) struct EmbeddedBrowser {
    pub(super) tabs: Vec<BrowserTab>,
    pub(super) active_tab: usize,
}

pub(super) struct BrowserTab {
    pub(super) webview: WebView,
}

impl EmbeddedBrowser {
    pub(super) fn active_webview(&self) -> Option<&WebView> {
        self.tabs.get(self.active_tab).map(|tab| &tab.webview)
    }
}

impl BrowserManager {
    pub(crate) fn new() -> Self {
        Self {
            overlay: None,
            browsers: HashMap::new(),
            initialized: false,
            web_context: None,
            bounds: HashMap::new(),
        }
    }

    pub fn show_or_create(
        &mut self,
        pane_id: &str,
        initial_urls: &[String],
        initial_active: usize,
        selected_profile: Option<&BrowserProfile>,
    ) {
        if let Some(browser) = self.browsers.get(pane_id) {
            if let Some(webview) = browser.active_webview() {
                webview.show();
            }
            return;
        }
        let Some(bounds) = self.bounds.get(pane_id).copied() else {
            return;
        };
        self.ensure_container();
        self.ensure_web_context(selected_profile);
        let urls = if initial_urls.is_empty() {
            vec!["about:blank".to_string()]
        } else {
            initial_urls.to_vec()
        };
        let active = initial_active.min(urls.len().saturating_sub(1));
        let tabs = urls
            .iter()
            .enumerate()
            .filter_map(|(idx, url)| {
                self.create_webview(url, bounds)
                    .map(|webview| (idx, webview))
            })
            .map(|(idx, webview)| {
                if idx == active {
                    webview.show();
                }
                BrowserTab { webview }
            })
            .collect::<Vec<_>>();
        if !tabs.is_empty() {
            self.browsers.insert(
                pane_id.to_string(),
                EmbeddedBrowser {
                    tabs,
                    active_tab: active,
                },
            );
            self.apply_bounds(pane_id);
        }
    }

    pub fn add_tab(&mut self, pane_id: &str, url: &str) {
        self.ensure_container();
        self.ensure_web_context(None);
        let Some(bounds) = self.bounds.get(pane_id).copied() else {
            return;
        };
        let Some(webview) = self.create_webview(url, bounds) else {
            return;
        };
        let Some(browser) = self.browsers.get_mut(pane_id) else {
            return;
        };
        if let Some(current) = browser.active_webview() {
            current.hide();
        }
        webview.show();
        browser.tabs.push(BrowserTab { webview });
        browser.active_tab = browser.tabs.len() - 1;
        self.apply_bounds(pane_id);
    }

    pub fn close_tab(&mut self, pane_id: &str, tab_idx: usize) {
        let overlay = self.overlay.clone();
        let Some(browser) = self.browsers.get_mut(pane_id) else {
            return;
        };
        if browser.tabs.len() <= 1 || tab_idx >= browser.tabs.len() {
            return;
        }
        let removed = browser.tabs.remove(tab_idx);
        if let Some(overlay) = &overlay {
            overlay.remove(&removed.webview);
        }
        browser.active_tab = browser.active_tab.min(browser.tabs.len() - 1);
        if let Some(webview) = browser.active_webview() {
            webview.show();
        }
        self.apply_bounds(pane_id);
    }

    pub fn switch_tab(&mut self, pane_id: &str, tab_idx: usize) {
        let Some(browser) = self.browsers.get_mut(pane_id) else {
            return;
        };
        if tab_idx >= browser.tabs.len() || tab_idx == browser.active_tab {
            return;
        }
        if let Some(webview) = browser.active_webview() {
            webview.hide();
        }
        browser.active_tab = tab_idx;
        if let Some(webview) = browser.active_webview() {
            webview.show();
        }
        self.apply_bounds(pane_id);
    }

    pub fn get_tabs_info(&self, pane_id: &str) -> Option<(Vec<TabInfo>, usize)> {
        let browser = self.browsers.get(pane_id)?;
        let tabs = browser
            .tabs
            .iter()
            .map(|tab| {
                let url = tab.webview.uri().map(|s| s.to_string()).unwrap_or_default();
                let title = tab
                    .webview
                    .title()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                TabInfo {
                    title: if title.is_empty() { url.clone() } else { title },
                    url,
                }
            })
            .collect();
        Some((tabs, browser.active_tab))
    }

    pub fn hide_all(&self) {
        for browser in self.browsers.values() {
            for tab in &browser.tabs {
                tab.webview.hide();
            }
        }
    }

    pub fn show_active_tabs(&self) {
        for browser in self.browsers.values() {
            if let Some(webview) = browser.active_webview() {
                webview.show();
            }
        }
    }

    pub fn destroy(&mut self, pane_id: &str) {
        self.bounds.remove(pane_id);
        if let Some(browser) = self.browsers.remove(pane_id) {
            if let Some(overlay) = &self.overlay {
                for tab in &browser.tabs {
                    overlay.remove(&tab.webview);
                }
            }
        }
    }

    pub fn navigate(&self, pane_id: &str, url: &str) {
        self.with_active(pane_id, |webview| webview.load_uri(url));
    }

    pub fn go_back(&self, pane_id: &str) {
        self.with_active(pane_id, WebViewExt::go_back);
    }

    pub fn go_forward(&self, pane_id: &str) {
        self.with_active(pane_id, WebViewExt::go_forward);
    }

    pub fn reload(&self, pane_id: &str) {
        self.with_active(pane_id, WebViewExt::reload);
    }

    pub fn get_url(&self, pane_id: &str) -> Option<String> {
        self.browsers
            .get(pane_id)
            .and_then(EmbeddedBrowser::active_webview)
            .and_then(|webview| webview.uri().map(|s| s.to_string()))
    }

    pub fn can_go_back(&self, pane_id: &str) -> bool {
        self.browsers
            .get(pane_id)
            .and_then(EmbeddedBrowser::active_webview)
            .is_some_and(WebViewExt::can_go_back)
    }

    pub fn can_go_forward(&self, pane_id: &str) -> bool {
        self.browsers
            .get(pane_id)
            .and_then(EmbeddedBrowser::active_webview)
            .is_some_and(WebViewExt::can_go_forward)
    }

    fn with_active(&self, pane_id: &str, f: impl FnOnce(&WebView)) {
        if let Some(webview) = self
            .browsers
            .get(pane_id)
            .and_then(EmbeddedBrowser::active_webview)
        {
            f(webview);
        }
    }
}
