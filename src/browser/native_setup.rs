use crate::browser::native::BrowserManager;
use crate::browser::profile::BrowserProfile;
use crate::browser::{cookies, extensions, profile};
use gtk::prelude::*;
use std::path::PathBuf;
use webkit2gtk::{
    CookieManagerExt, CookiePersistentStorage, SettingsExt, WebContext, WebContextExt, WebView,
    WebViewExt, WebsiteDataManager,
};

impl BrowserManager {
    pub fn update_bounds(
        &mut self,
        pane_id: &str,
        bounds: crate::browser::geometry::BrowserBounds,
    ) {
        if self.bounds.get(pane_id) != Some(&bounds) {
            tracing::info!(pane_id, ?bounds, "[Browser] Applying content bounds");
        }
        self.bounds.insert(pane_id.to_string(), bounds);
        self.apply_bounds(pane_id);
    }

    pub(super) fn apply_bounds(&self, pane_id: &str) {
        let Some(bounds) = self.bounds.get(pane_id) else {
            return;
        };
        if let Some(webview) = self
            .browsers
            .get(pane_id)
            .and_then(crate::browser::native::EmbeddedBrowser::active_webview)
        {
            webview.set_size_request(bounds.width, bounds.height);
            webview.set_margin_start(bounds.x);
            webview.set_margin_top(bounds.y);
        }
    }

    pub(super) fn ensure_container(&mut self) {
        if self.initialized {
            return;
        }
        let Some(window) = gtk::Window::list_toplevels()
            .iter()
            .filter_map(|w| w.downcast_ref::<gtk::Window>())
            .find(|w| w.title().is_some_and(|t| t.as_str().contains("Muxspace")))
            .cloned()
        else {
            tracing::warn!("[Browser] Main window not found yet");
            return;
        };
        let Some(child) = window.child() else {
            return;
        };
        let Ok(vbox) = child.downcast::<gtk::Box>() else {
            return;
        };
        let Some(dioxus_wv) = vbox
            .children()
            .into_iter()
            .find(|child| child.type_().name().contains("WebView"))
        else {
            tracing::warn!("[Browser] Dioxus WebView widget not found");
            return;
        };
        vbox.remove(&dioxus_wv);
        let overlay = gtk::Overlay::new();
        overlay.add(&dioxus_wv);
        vbox.pack_start(&overlay, true, true, 0);
        overlay.show_all();
        self.overlay = Some(overlay);
        self.initialized = true;
    }

    pub(super) fn ensure_web_context(&mut self, selected_profile: Option<&BrowserProfile>) {
        if self.web_context.is_some() {
            return;
        }
        let (data_dir, cache_dir) = Self::profile_dirs();
        if let Err(err) = std::fs::create_dir_all(&data_dir) {
            tracing::warn!("[Browser] Data dir create failed: {err}");
        }
        if let Err(err) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!("[Browser] Cache dir create failed: {err}");
        }
        let data_mgr = WebsiteDataManager::builder()
            .base_data_directory(data_dir.to_string_lossy().as_ref())
            .base_cache_directory(cache_dir.to_string_lossy().as_ref())
            .build();
        let context = WebContext::with_website_data_manager(&data_mgr);
        let cookies_path = data_dir.join("cookies.txt");
        Self::import_browser_profile(&cookies_path, selected_profile);
        if let Some(cm) = WebContextExt::cookie_manager(&context) {
            CookieManagerExt::set_persistent_storage(
                &cm,
                &cookies_path.to_string_lossy(),
                CookiePersistentStorage::Text,
            );
        }
        self.web_context = Some(context);
    }

    pub(super) fn create_webview(
        &self,
        url: &str,
        bounds: crate::browser::geometry::BrowserBounds,
    ) -> Option<WebView> {
        let overlay = self.overlay.as_ref()?;
        let context = self.web_context.as_ref()?;
        let webview = WebView::with_context(context);
        if let Some(settings) = WebViewExt::settings(&webview) {
            settings.set_enable_javascript(true);
            settings.set_enable_developer_extras(true);
            settings.set_javascript_can_open_windows_automatically(true);
            settings.set_enable_webgl(true);
            settings.set_enable_webaudio(true);
            settings.set_enable_media_stream(true);
            settings.set_enable_mediasource(true);
            settings.set_enable_encrypted_media(true);
            settings.set_enable_media_capabilities(true);
            settings
                .set_hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Never);
        }
        webview.connect_load_failed(|_, event, uri, error| {
            tracing::warn!("[Browser] Load failed at {event:?} for {uri}: {error}");
            false
        });
        webview.connect_web_process_terminated(|_, reason| {
            tracing::error!("[Browser] Web process terminated: {reason:?}");
        });
        webview.set_size_request(bounds.width, bounds.height);
        webview.set_margin_start(bounds.x);
        webview.set_margin_top(bounds.y);
        webview.set_halign(gtk::Align::Start);
        webview.set_valign(gtk::Align::Start);
        overlay.add_overlay(&webview);
        overlay.set_overlay_pass_through(&webview, false);
        webview.hide();
        let url = Self::normalize_url(url);
        let clone = webview.clone();
        glib::idle_add_local_once(move || clone.load_uri(&url));
        Some(webview)
    }

    fn import_browser_profile(cookies_path: &std::path::Path, selected: Option<&BrowserProfile>) {
        let detected;
        let browser_profile = match selected {
            Some(profile) => Ok(profile),
            None => {
                detected = profile::detect_default_browser();
                detected.as_ref()
            }
        };
        match browser_profile {
            Ok(browser_profile) => {
                match cookies::import_cookies(browser_profile, cookies_path) {
                    Ok(n) => tracing::info!(
                        "[Browser] Imported {n} cookies from {:?}",
                        browser_profile.kind
                    ),
                    Err(err) => tracing::warn!("[Browser] Cookie import failed: {err}"),
                }
                match extensions::detect_extensions(browser_profile) {
                    Ok(exts) => {
                        for ext in &exts {
                            tracing::info!("[Browser] Extension: {} v{}", ext.name, ext.version);
                        }
                    }
                    Err(err) => tracing::warn!("[Browser] Extension detection failed: {err}"),
                }
            }
            Err(err) => tracing::warn!("[Browser] Browser profile not detected: {err}"),
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

    fn normalize_url(url: &str) -> String {
        if url.is_empty()
            || url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("about:")
        {
            url.to_string()
        } else {
            format!("https://{url}")
        }
    }
}
