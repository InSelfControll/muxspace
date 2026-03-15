//! Detect the user's default browser and locate its profile directory.
//!
//! Uses `xdg-settings get default-web-browser` on Linux, falling back to
//! scanning common config directories when xdg-settings is unavailable.

use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum BrowserKind {
    Chrome,
    Chromium,
    Firefox,
    Brave,
}

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

fn chrome_profile() -> Result<BrowserProfile> {
    let dir = config_dir().join("google-chrome/Default");
    require_dir(&dir, "Chrome")?;
    Ok(BrowserProfile { kind: BrowserKind::Chrome, profile_dir: dir })
}

fn chromium_profile() -> Result<BrowserProfile> {
    let dir = config_dir().join("chromium/Default");
    require_dir(&dir, "Chromium")?;
    Ok(BrowserProfile { kind: BrowserKind::Chromium, profile_dir: dir })
}

fn brave_profile() -> Result<BrowserProfile> {
    let dir = config_dir().join("BraveSoftware/Brave-Browser/Default");
    require_dir(&dir, "Brave")?;
    Ok(BrowserProfile { kind: BrowserKind::Brave, profile_dir: dir })
}

fn firefox_profile() -> Result<BrowserProfile> {
    let ff_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".mozilla/firefox");

    // Modern Firefox uses *.default-release; older ones use *.default
    for pattern in ["*.default-release", "*.default"] {
        let glob_expr = format!("{}/{pattern}", ff_dir.display());
        if let Some(Ok(path)) = glob::glob(&glob_expr).ok().and_then(|mut g| g.next()) {
            if path.is_dir() {
                return Ok(BrowserProfile {
                    kind: BrowserKind::Firefox,
                    profile_dir: path,
                });
            }
        }
    }

    Err(anyhow!("No Firefox profile directory found"))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn detect_by_installed() -> Result<BrowserProfile> {
    let resolvers: &[(&str, fn() -> Result<BrowserProfile>)] = &[
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

fn require_dir(dir: &PathBuf, label: &str) -> Result<()> {
    if dir.is_dir() {
        Ok(())
    } else {
        Err(anyhow!("{label} profile not found: {}", dir.display()))
    }
}
