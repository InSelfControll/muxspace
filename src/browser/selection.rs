//! Browser selection dialog to let users choose between installed browsers.
//!
//! This module provides functionality to detect all installed browsers
//! and present them in a selection dialog for the user.

use std::path::PathBuf;

use crate::browser::profile::{
    brave_profile, chrome_profile, chromium_profile, firefox_profile, BrowserKind, BrowserProfile,
};

#[derive(Debug, Clone, PartialEq)]
pub struct InstalledBrowser {
    pub kind: BrowserKind,
    pub name: String,
    pub profile_dir: PathBuf,
    pub executable: String,
}

impl InstalledBrowser {
    pub fn profile(&self) -> BrowserProfile {
        BrowserProfile {
            kind: self.kind.clone(),
            profile_dir: self.profile_dir.clone(),
        }
    }
}

/// Detect all installed browsers on the system.
pub fn detect_installed_browsers() -> Vec<InstalledBrowser> {
    let mut browsers = Vec::new();

    // Check Chrome
    if let Ok(profile) = chrome_profile() {
        browsers.push(InstalledBrowser {
            kind: BrowserKind::Chrome,
            name: "Google Chrome".to_string(),
            profile_dir: profile.profile_dir,
            executable: "google-chrome".to_string(),
        });
    }

    // Check Chromium
    if let Ok(profile) = chromium_profile() {
        browsers.push(InstalledBrowser {
            kind: BrowserKind::Chromium,
            name: "Chromium".to_string(),
            profile_dir: profile.profile_dir,
            executable: "chromium".to_string(),
        });
    }

    // Check Brave
    if let Ok(profile) = brave_profile() {
        browsers.push(InstalledBrowser {
            kind: BrowserKind::Brave,
            name: "Brave Browser".to_string(),
            profile_dir: profile.profile_dir,
            executable: "brave-browser".to_string(),
        });
    }

    // Check Firefox
    if let Ok(profile) = firefox_profile() {
        browsers.push(InstalledBrowser {
            kind: BrowserKind::Firefox,
            name: "Firefox".to_string(),
            profile_dir: profile.profile_dir,
            executable: "firefox".to_string(),
        });
    }

    browsers
}
