//! Detect browser extensions installed in the user's default browser profile.
//!
//! Chrome extensions are found in `<profile>/Extensions/<id>/<ver>/manifest.json`.
//! Firefox extensions live in `<profile>/extensions/` or are listed in `extensions.json`.

use anyhow::Result;
use std::path::Path;

use super::profile::{BrowserKind, BrowserProfile};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Scan the browser profile for installed extensions.
pub fn detect_extensions(profile: &BrowserProfile) -> Result<Vec<ExtensionInfo>> {
    match &profile.kind {
        BrowserKind::Chrome | BrowserKind::Chromium | BrowserKind::Brave => {
            detect_chrome_extensions(&profile.profile_dir)
        }
        BrowserKind::Firefox => detect_firefox_extensions(&profile.profile_dir),
    }
}

// ---------------------------------------------------------------------------
// Chrome / Chromium / Brave
// ---------------------------------------------------------------------------

/// `Extensions/<ext_id>/<version>/manifest.json`
fn detect_chrome_extensions(profile_dir: &Path) -> Result<Vec<ExtensionInfo>> {
    let ext_dir = profile_dir.join("Extensions");
    if !ext_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut extensions = Vec::new();

    for entry in std::fs::read_dir(&ext_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let ext_id = entry.file_name().to_string_lossy().to_string();

        // Each extension has version subdirectories — pick the latest
        let mut versions: Vec<_> = std::fs::read_dir(entry.path())?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map_or(false, |t| t.is_dir()))
            .collect();
        versions.sort_by_key(|e| e.file_name());

        if let Some(ver_dir) = versions.last() {
            let manifest_path = ver_dir.path().join("manifest.json");
            if manifest_path.exists() {
                if let Ok(info) = parse_manifest(&ext_id, &manifest_path) {
                    extensions.push(info);
                }
            }
        }
    }

    tracing::info!("[Extensions] Found {} Chrome extensions", extensions.len());
    Ok(extensions)
}

// ---------------------------------------------------------------------------
// Firefox
// ---------------------------------------------------------------------------

fn detect_firefox_extensions(profile_dir: &Path) -> Result<Vec<ExtensionInfo>> {
    let ext_dir = profile_dir.join("extensions");

    // Prefer scanning the actual directory first
    if ext_dir.is_dir() {
        let mut extensions = Vec::new();

        for entry in std::fs::read_dir(&ext_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();

            if name.ends_with(".xpi") {
                // XPI = ZIP archive — extract ID from filename
                let id = name.trim_end_matches(".xpi").to_string();
                extensions.push(ExtensionInfo {
                    id: id.clone(),
                    name: id,
                    version: "unknown".into(),
                    description: String::new(),
                });
            } else if entry.file_type()?.is_dir() {
                let manifest = entry.path().join("manifest.json");
                if manifest.exists() {
                    if let Ok(info) = parse_manifest(&name, &manifest) {
                        extensions.push(info);
                    }
                }
            }
        }

        tracing::info!("[Extensions] Found {} Firefox extensions", extensions.len());
        return Ok(extensions);
    }

    // Fallback: parse extensions.json (Firefox stores addon metadata here)
    read_firefox_extensions_json(profile_dir)
}

fn read_firefox_extensions_json(profile_dir: &Path) -> Result<Vec<ExtensionInfo>> {
    let json_path = profile_dir.join("extensions.json");
    if !json_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&json_path)?;
    let data: serde_json::Value = serde_json::from_str(&content)?;

    let mut extensions = Vec::new();

    if let Some(addons) = data.get("addons").and_then(|a| a.as_array()) {
        for addon in addons {
            let id = addon.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }

            let name = addon
                .get("defaultLocale")
                .and_then(|l| l.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();

            let version = addon
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0")
                .to_string();

            let description = addon
                .get("defaultLocale")
                .and_then(|l| l.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            extensions.push(ExtensionInfo { id, name, version, description });
        }
    }

    tracing::info!(
        "[Extensions] Found {} Firefox extensions from extensions.json",
        extensions.len()
    );
    Ok(extensions)
}

// ---------------------------------------------------------------------------
// Shared manifest parser (works for Chrome & directory-based Firefox extensions)
// ---------------------------------------------------------------------------

fn parse_manifest(ext_id: &str, path: &Path) -> Result<ExtensionInfo> {
    let content = std::fs::read_to_string(path)?;
    let manifest: serde_json::Value = serde_json::from_str(&content)?;

    let mut name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    // Chrome uses __MSG_*__ placeholders for i18n — fall back to extension ID
    if name.starts_with("__MSG_") {
        name = ext_id.to_string();
    }

    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0")
        .to_string();

    let description = manifest
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(ExtensionInfo {
        id: ext_id.to_string(),
        name,
        version,
        description,
    })
}
