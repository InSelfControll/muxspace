use crate::browser::profile::BrowserProfile;
use crate::browser::TabInfo;

pub struct BrowserManager;

impl BrowserManager {
    pub(crate) fn new() -> Self {
        Self
    }

    pub fn show_or_create(
        &mut self,
        _pane_id: &str,
        _initial_urls: &[String],
        _initial_active: usize,
        _selected_profile: Option<&BrowserProfile>,
    ) {
    }

    pub fn add_tab(&mut self, _pane_id: &str, _url: &str) {}
    pub fn close_tab(&mut self, _pane_id: &str, _tab_idx: usize) {}
    pub fn switch_tab(&mut self, _pane_id: &str, _tab_idx: usize) {}
    pub fn get_tabs_info(&self, _pane_id: &str) -> Option<(Vec<TabInfo>, usize)> {
        None
    }
    pub fn update_bounds(&self, _pane_id: &str, _x: i32, _y: i32, _w: i32, _h: i32) {}
    pub fn hide_all(&self) {}
    pub fn show_active_tabs(&self) {}
    pub fn destroy(&mut self, _pane_id: &str) {}
    pub fn navigate(&self, _pane_id: &str, _url: &str) {}
    pub fn go_back(&self, _pane_id: &str) {}
    pub fn go_forward(&self, _pane_id: &str) {}
    pub fn reload(&self, _pane_id: &str) {}
    pub fn get_url(&self, _pane_id: &str) -> Option<String> {
        None
    }
    pub fn can_go_back(&self, _pane_id: &str) -> bool {
        false
    }
    pub fn can_go_forward(&self, _pane_id: &str) -> bool {
        false
    }
}
