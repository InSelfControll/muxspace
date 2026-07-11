//! Detect the user's default browser and locate its profile directory.
//!
//! Uses `xdg-settings get default-web-browser` on Linux, falling back to
//! scanning common config directories when xdg-settings is unavailable.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserKind {
    Chrome,
    Chromium,
    Firefox,
    Brave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserProfile {
    pub kind: BrowserKind,
    pub profile_dir: PathBuf,
}

/// Detect the default browser via `xdg-settings` and resolve its profile path.
pub fn detect_default_browser() -> Result<BrowserProfile> {
    let output = std::process::Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let desktop = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            tracing::info!("[Profile] Default browser .desktop: {desktop}");

            if desktop.contains("google-chrome") {
                return chrome_profile();
            }
            if desktop.contains("chromium") {
                return chromium_profile();
            }
            if desktop.contains("brave") {
                return brave_profile();
            }
            if desktop.contains("firefox") {
                return firefox_profile();
            }
        }
    }

    // Fallback: try common browser directories in order of popularity
    detect_by_installed()
}

// ---------------------------------------------------------------------------
// Per-browser profile resolution
// ---------------------------------------------------------------------------

pub(crate) fn chrome_profile() -> Result<BrowserProfile> {
    let dir = first_profile_dir(
        &[
            config_dir().join("google-chrome/Default"),
            flatpak_dir().join("com.google.Chrome/config/google-chrome/Default"),
            flatpak_dir().join("com.google.ChromeDev/config/google-chrome-unstable/Default"),
        ],
        "Chrome",
    )?;
    Ok(BrowserProfile {
        kind: BrowserKind::Chrome,
        profile_dir: dir,
    })
}

pub(crate) fn chromium_profile() -> Result<BrowserProfile> {
    let dir = first_profile_dir(
        &[
            config_dir().join("chromium/Default"),
            flatpak_dir().join("org.chromium.Chromium/config/chromium/Default"),
        ],
        "Chromium",
    )?;
    Ok(BrowserProfile {
        kind: BrowserKind::Chromium,
        profile_dir: dir,
    })
}

pub(crate) fn brave_profile() -> Result<BrowserProfile> {
    let dir = first_profile_dir(
        &[
            config_dir().join("BraveSoftware/Brave-Browser/Default"),
            flatpak_dir().join("com.brave.Browser/config/BraveSoftware/Brave-Browser/Default"),
        ],
        "Brave",
    )?;
    Ok(BrowserProfile {
        kind: BrowserKind::Brave,
        profile_dir: dir,
    })
}

pub(crate) fn firefox_profile() -> Result<BrowserProfile> {
    let home = dirs::home_dir().unwrap_or_default();
    let roots = [
        home.join(".mozilla/firefox"),
        flatpak_dir().join("org.mozilla.firefox/.mozilla/firefox"),
    ];
    let mut profiles = roots
        .iter()
        .filter_map(|root| std::fs::read_dir(root).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("cookies.sqlite").is_file())
        .collect::<Vec<_>>();
    profiles.sort_by_key(|path| {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        (
            !name.ends_with(".default-release"),
            !name.ends_with(".default"),
        )
    });
    let profile_dir = profiles
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No Firefox profile with cookies found"))?;
    Ok(BrowserProfile {
        kind: BrowserKind::Firefox,
        profile_dir,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type BrowserResolver = fn() -> Result<BrowserProfile>;

fn detect_by_installed() -> Result<BrowserProfile> {
    let resolvers: &[(&str, BrowserResolver)] = &[
        ("Chrome", chrome_profile),
        ("Chromium", chromium_profile),
        ("Brave", brave_profile),
        ("Firefox", firefox_profile),
    ];

    for (name, resolve) in resolvers {
        if let Ok(profile) = resolve() {
            tracing::info!("[Profile] Fallback detected: {name}");
            return Ok(profile);
        }
    }

    Err(anyhow!("No supported browser installation found"))
}

fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".config")
    })
}

fn flatpak_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".var/app")
}

fn first_profile_dir(candidates: &[PathBuf], label: &str) -> Result<PathBuf> {
    candidates
        .iter()
        .find(|dir| dir.is_dir())
        .cloned()
        .ok_or_else(|| anyhow!("{label} profile not found"))
}
