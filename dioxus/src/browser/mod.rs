//! Embedded browser via native WebKitWebView overlaid on the Dioxus UI.
//!
//! Architecture: the main Dioxus WebView is wrapped in a GtkOverlay.
//! Each browser pane gets a real webkit2gtk::WebView added as an overlay child,
//! positioned to match a placeholder div in the Dioxus DOM.
//!
//! All browser panes share a **persistent WebContext** backed by a
//! `WebsiteDataManager` at `~/.local/share/muxspace/browser-data/`.
//! On first launch the user's default browser is detected and its cookies +
//! extension list are imported automatically.

pub mod cookies;
pub mod extensions;
pub mod profile;

use gtk::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use webkit2gtk::{
    CookieManagerExt, CookiePersistentStorage, SettingsExt, WebContext, WebContextExt,
    WebView, WebViewExt, WebsiteDataManager,
};

/// Realistic user-agent — prevents sites from flagging us as a bot.
const CHROME_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

pub static BROWSER_MGR: LazyLock<Mutex<BrowserManager>> =
    LazyLock::new(|| Mutex::new(BrowserManager::new()));

/// Metadata about a single browser tab, returned to the UI layer.
pub struct TabInfo {
    pub title: String,
    pub url: String,
}

pub struct BrowserManager {
    /// The gtk::Fixed layered on top of the Dioxus WebView.
    fixed: Option<gtk::Fixed>,
    browsers: HashMap<String, EmbeddedBrowser>,
    initialized: bool,
    /// Shared persistent WebContext — all browser panes use this.
    web_context: Option<WebContext>,
}

struct EmbeddedBrowser {
    tabs: Vec<BrowserTab>,
    active_tab: usize,
}

struct BrowserTab {
    webview: WebView,
}

impl EmbeddedBrowser {
    fn active_webview(&self) -> Option<&WebView> {
        self.tabs.get(self.active_tab).map(|t| &t.webview)
    }
}

// Safety: all GTK operations are performed on the GTK main thread only.
// The Mutex prevents concurrent access; callers ensure they're on the main thread.
unsafe impl Send for BrowserManager {}

impl BrowserManager {
    fn new() -> Self {
        Self {
            fixed: None,
            browsers: HashMap::new(),
            initialized: false,
            web_context: None,
        }
    }

    /// Wrap the Dioxus WebView in a GtkOverlay with a gtk::Fixed on top.
    /// The Fixed container gives us direct pixel control over browser WebView
    /// positions and sizes — no signal-handler indirection.
    fn ensure_container(&mut self) {
        if self.initialized {
            return;
        }

        let toplevels = gtk::Window::list_toplevels();
        let main_window = toplevels
            .iter()
            .filter_map(|w| w.downcast_ref::<gtk::Window>())
            .find(|w| {
                w.title()
                    .map_or(false, |t| t.as_str().contains("Muxspace"))
            })
            .cloned();

        let main_window = match main_window {
            Some(w) => w,
            None => {
                tracing::warn!("[Browser] Main window not found yet");
                return;
            }
        };

        // tao wraps content in a GtkBox (vbox): [MenuBar?, WebView]
        // We wrap ONLY the WebView so coordinates from getBoundingClientRect
        // match the overlay space exactly.
        let vbox = match main_window.child() {
            Some(c) => match c.downcast::<gtk::Box>() {
                Ok(b) => b,
                Err(_) => return,
            },
            None => return,
        };

        // Find the Dioxus WebView inside the vbox.
        let children = vbox.children();
        let dioxus_wv = children.into_iter().find(|c| {
            let tn = c.type_().name();
            tn.contains("WebKit") || tn.contains("WebView")
        });

        let dioxus_wv = match dioxus_wv {
            Some(w) => w,
            None => {
                tracing::warn!("[Browser] Dioxus WebView widget not found in vbox");
                return;
            }
        };

        // Reparent: vbox → […, Overlay → [Dioxus WV (main), Fixed (overlay)]]
        // The Fixed sits on top of the Dioxus WebView and holds browser WebViews.
        vbox.remove(&dioxus_wv);

        let overlay = gtk::Overlay::new();
        overlay.add(&dioxus_wv); // main child = Dioxus WebView

        let fixed = gtk::Fixed::new();
        // Make the Fixed transparent to input on areas without browser WebViews
        fixed.set_can_focus(false);
        overlay.add_overlay(&fixed);
        // Let the Fixed pass through clicks to the Dioxus WebView underneath
        overlay.set_overlay_pass_through(&fixed, true);

        vbox.pack_start(&overlay, true, true, 0);
        overlay.show_all();

        self.fixed = Some(fixed);
        self.initialized = true;
        tracing::info!("[Browser] Container initialized (Overlay + Fixed)");
    }

    // -----------------------------------------------------------------------
    // Persistent WebContext — shared by all browser panes
    // -----------------------------------------------------------------------

    fn ensure_web_context(&mut self) {
        if self.web_context.is_some() {
            return;
        }

        let (data_dir, cache_dir) = Self::profile_dirs();
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(&cache_dir);

        let data_mgr = WebsiteDataManager::builder()
            .base_data_directory(data_dir.to_string_lossy().as_ref())
            .base_cache_directory(cache_dir.to_string_lossy().as_ref())
            .build();

        let context = WebContext::with_website_data_manager(&data_mgr);
        let cookies_path = data_dir.join("cookies.txt");

        // On first launch, import cookies + detect extensions
        if !cookies_path.exists() {
            Self::import_browser_profile(&cookies_path);
        }

        // Point WebKit at the persistent cookie file
        if let Some(cm) = WebContextExt::cookie_manager(&context) {
            CookieManagerExt::set_persistent_storage(
                &cm,
                &cookies_path.to_string_lossy(),
                CookiePersistentStorage::Text,
            );
        }

        tracing::info!(
            "[Browser] Persistent WebContext ready ({})",
            data_dir.display()
        );
        self.web_context = Some(context);
    }

    fn import_browser_profile(cookies_path: &std::path::Path) {
        match profile::detect_default_browser() {
            Ok(bp) => {
                match cookies::import_cookies(&bp, cookies_path) {
                    Ok(n) => tracing::info!("[Browser] Imported {n} cookies from {:?}", bp.kind),
                    Err(e) => tracing::warn!("[Browser] Cookie import failed: {e}"),
                }
                match extensions::detect_extensions(&bp) {
                    Ok(exts) => {
                        for ext in &exts {
                            tracing::info!(
                                "[Browser] Extension: {} v{}",
                                ext.name,
                                ext.version
                            );
                        }
                    }
                    Err(e) => tracing::warn!("[Browser] Extension detection failed: {e}"),
                }
            }
            Err(e) => tracing::warn!("[Browser] Default browser not detected: {e}"),
        }
    }

    fn profile_dirs() -> (PathBuf, PathBuf) {
        let data = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("muxspace/browser-data");
        let cache = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("muxspace/browser-cache");
        (data, cache)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Create a fully-configured WebView and add it to the Fixed container.
    /// The WebView starts hidden at 1×1; call `.show()` on the active tab.
    fn create_configured_webview(&self, url: &str) -> Option<WebView> {
        let fixed = self.fixed.as_ref()?;
        let context = self.web_context.as_ref()?;
        let webview = WebView::with_context(context);

        Self::configure_webview_settings(&webview);

        webview.set_size_request(1, 1);
        fixed.put(&webview, 0, 0);
        webview.hide(); // hidden by default; caller shows the active tab

        let url_owned = Self::normalize_url(url);
        let webview_clone = webview.clone();
        glib::idle_add_local_once(move || {
            webview_clone.load_uri(&url_owned);
        });

        Some(webview)
    }

    fn configure_webview_settings(webview: &WebView) {
        if let Some(settings) = WebViewExt::settings(webview) {
            settings.set_user_agent(Some(CHROME_USER_AGENT));
            settings.set_enable_javascript(true);
            settings.set_enable_developer_extras(true);
            settings.set_javascript_can_open_windows_automatically(true);
            settings.set_enable_webgl(true);
            settings.set_enable_webaudio(true);
            settings.set_enable_media_stream(true);
            settings.set_enable_mediasource(true);
            settings.set_enable_encrypted_media(true);
            settings.set_enable_media_capabilities(true);
            settings.set_hardware_acceleration_policy(
                webkit2gtk::HardwareAccelerationPolicy::Always,
            );
        }
    }

    fn normalize_url(url: &str) -> String {
        let mut u = url.to_string();
        if !u.is_empty()
            && !u.starts_with("http://")
            && !u.starts_with("https://")
            && !u.starts_with("about:")
        {
            u = format!("https://{u}");
        }
        u
    }

    // -----------------------------------------------------------------------
    // Pane lifecycle
    // -----------------------------------------------------------------------

    /// Show an existing browser for this pane, or create a new one.
    /// If the pane already exists it is simply re-shown (preserving pages),
    /// so workspace switches don't reset navigation.
    pub fn show_or_create(&mut self, pane_id: &str, initial_urls: &[String], initial_active: usize) {
        if let Some(b) = self.browsers.get(pane_id) {
            if let Some(wv) = b.active_webview() {
                wv.show();
            }
            return;
        }

        self.ensure_container();
        self.ensure_web_context();

        // Remove any stale entry
        if let Some(old) = self.browsers.remove(pane_id) {
            if let Some(fixed) = &self.fixed {
                for tab in &old.tabs {
                    fixed.remove(&tab.webview);
                }
            }
        }

        let urls = if initial_urls.is_empty() {
            vec!["about:blank".to_string()]
        } else {
            initial_urls.to_vec()
        };
        let active = initial_active.min(urls.len().saturating_sub(1));

        let mut tabs = Vec::new();
        for (i, url) in urls.iter().enumerate() {
            if let Some(webview) = self.create_configured_webview(url) {
                if i == active {
                    webview.show();
                }
                tabs.push(BrowserTab { webview });
            }
        }

        if tabs.is_empty() {
            return;
        }

        self.browsers.insert(
            pane_id.to_string(),
            EmbeddedBrowser { tabs, active_tab: active },
        );
        tracing::info!("[Browser] Created browser for pane {} ({} tab(s))", pane_id, urls.len());
    }

    // -----------------------------------------------------------------------
    // Tab management
    // -----------------------------------------------------------------------

    /// Open a new tab in an existing browser pane and switch to it.
    pub fn add_tab(&mut self, pane_id: &str, url: &str) {
        self.ensure_container();
        self.ensure_web_context();

        let webview = match self.create_configured_webview(url) {
            Some(wv) => wv,
            None => return,
        };

        let b = match self.browsers.get_mut(pane_id) {
            Some(b) => b,
            None => return,
        };

        // Hide current active tab
        if let Some(wv) = b.active_webview() {
            wv.hide();
        }

        webview.show();
        b.tabs.push(BrowserTab { webview });
        b.active_tab = b.tabs.len() - 1;
        tracing::info!("[Browser] Added tab {} to pane {}", b.active_tab, pane_id);
    }

    /// Close a tab. If it's the last tab, this is a no-op (close the pane
    /// instead via `destroy`).
    pub fn close_tab(&mut self, pane_id: &str, tab_idx: usize) {
        let fixed = self.fixed.clone();
        let b = match self.browsers.get_mut(pane_id) {
            Some(b) => b,
            None => return,
        };
        if b.tabs.len() <= 1 || tab_idx >= b.tabs.len() {
            return;
        }

        let removed = b.tabs.remove(tab_idx);
        if let Some(f) = &fixed {
            f.remove(&removed.webview);
        }

        // Adjust active index
        if b.active_tab >= b.tabs.len() {
            b.active_tab = b.tabs.len() - 1;
        }
        // Ensure the new active tab is visible
        if let Some(wv) = b.active_webview() {
            wv.show();
        }
    }

    /// Switch to a different tab within a browser pane.
    pub fn switch_tab(&mut self, pane_id: &str, tab_idx: usize) {
        let b = match self.browsers.get_mut(pane_id) {
            Some(b) => b,
            None => return,
        };
        if tab_idx >= b.tabs.len() || tab_idx == b.active_tab {
            return;
        }

        if let Some(wv) = b.active_webview() {
            wv.hide();
        }
        b.active_tab = tab_idx;
        if let Some(wv) = b.active_webview() {
            wv.show();
        }
    }

    /// Return tab metadata + active index for the UI tab bar.
    pub fn get_tabs_info(&self, pane_id: &str) -> Option<(Vec<TabInfo>, usize)> {
        let b = self.browsers.get(pane_id)?;
        let tabs = b
            .tabs
            .iter()
            .map(|t| {
                let title = t.webview.title().map(|s| s.to_string()).unwrap_or_default();
                let url = t.webview.uri().map(|s| s.to_string()).unwrap_or_default();
                TabInfo {
                    title: if title.is_empty() { url.clone() } else { title },
                    url,
                }
            })
            .collect();
        Some((tabs, b.active_tab))
    }

    // -----------------------------------------------------------------------
    // Operations on the active tab
    // -----------------------------------------------------------------------

    /// Update pixel position and size of the *active* tab's WebView.
    pub fn update_bounds(&self, pane_id: &str, x: i32, y: i32, w: i32, h: i32) {
        if let Some(b) = self.browsers.get(pane_id) {
            if let Some(wv) = b.active_webview() {
                wv.set_size_request(w, h);
                if let Some(fixed) = &self.fixed {
                    fixed.move_(wv, x, y);
                }
            }
        }
    }

    /// Hide all browser WebViews across all panes (e.g., on workspace switch).
    pub fn hide_all(&self) {
        for b in self.browsers.values() {
            for tab in &b.tabs {
                tab.webview.hide();
            }
        }
    }

    /// Re-show the active tab's WebView for every browser pane.
    /// Used to restore browsers after a modal overlay is dismissed.
    pub fn show_active_tabs(&self) {
        for b in self.browsers.values() {
            if let Some(wv) = b.active_webview() {
                wv.show();
            }
        }
    }

    /// Destroy all tabs for a pane and remove them from the Fixed container.
    pub fn destroy(&mut self, pane_id: &str) {
        if let Some(b) = self.browsers.remove(pane_id) {
            if let Some(fixed) = &self.fixed {
                for tab in &b.tabs {
                    fixed.remove(&tab.webview);
                }
            }
            tracing::info!("[Browser] Destroyed browser for pane {}", pane_id);
        }
    }

    pub fn navigate(&self, pane_id: &str, url: &str) {
        if let Some(b) = self.browsers.get(pane_id) {
            if let Some(wv) = b.active_webview() {
                wv.load_uri(url);
            }
        }
    }

    pub fn go_back(&self, pane_id: &str) {
        if let Some(b) = self.browsers.get(pane_id) {
            if let Some(wv) = b.active_webview() {
                wv.go_back();
            }
        }
    }

    pub fn go_forward(&self, pane_id: &str) {
        if let Some(b) = self.browsers.get(pane_id) {
            if let Some(wv) = b.active_webview() {
                wv.go_forward();
            }
        }
    }

    pub fn reload(&self, pane_id: &str) {
        if let Some(b) = self.browsers.get(pane_id) {
            if let Some(wv) = b.active_webview() {
                wv.reload();
            }
        }
    }

    pub fn get_url(&self, pane_id: &str) -> Option<String> {
        self.browsers
            .get(pane_id)
            .and_then(|b| b.active_webview())
            .and_then(|wv| wv.uri().map(|s| s.to_string()))
    }

    pub fn can_go_back(&self, pane_id: &str) -> bool {
        self.browsers
            .get(pane_id)
            .and_then(|b| b.active_webview())
            .map_or(false, |wv| wv.can_go_back())
    }

    pub fn can_go_forward(&self, pane_id: &str) -> bool {
        self.browsers
            .get(pane_id)
            .and_then(|b| b.active_webview())
            .map_or(false, |wv| wv.can_go_forward())
    }
}
