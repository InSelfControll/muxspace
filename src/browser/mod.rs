pub mod cookies;
pub mod extensions;
pub mod geometry;
#[cfg(feature = "desktop")]
mod native;
#[cfg(feature = "desktop")]
mod native_setup;
pub mod profile;
pub mod selection;
mod sqlite_snapshot;
#[cfg(not(feature = "desktop"))]
mod stub;

#[cfg(feature = "desktop")]
pub use native::BrowserManager;
#[cfg(not(feature = "desktop"))]
pub use stub::BrowserManager;

use std::cell::RefCell;

thread_local! {
    static BROWSER_MGR: RefCell<BrowserManager> = RefCell::new(BrowserManager::new());
}

pub fn with_manager<R>(f: impl FnOnce(&BrowserManager) -> R) -> Option<R> {
    BROWSER_MGR.with(|manager| manager.try_borrow().ok().map(|manager| f(&manager)))
}

pub fn with_manager_mut<R>(f: impl FnOnce(&mut BrowserManager) -> R) -> Option<R> {
    BROWSER_MGR.with(|manager| {
        manager
            .try_borrow_mut()
            .ok()
            .map(|mut manager| f(&mut manager))
    })
}

pub struct TabInfo {
    pub title: String,
    pub url: String,
}
