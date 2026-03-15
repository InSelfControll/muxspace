//! Import cookies from Chrome/Firefox into a Netscape-format cookie file.
//!
//! WebKit loads this file via `CookieManager::set_persistent_storage(path, Text)`.
//! Firefox cookies are unencrypted. Chrome cookies use v10 (PBKDF2+AES) or
//! v11 (keyring) encryption — we handle v10 and skip v11 with a warning.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::profile::{BrowserKind, BrowserProfile};

struct RawCookie {
    domain: String,
    path: String,
    secure: bool,
    expiry: i64,
    name: String,
    value: String,
}

/// Import cookies from the detected browser and write a Netscape cookie file.
/// Returns the number of cookies imported.
pub fn import_cookies(profile: &BrowserProfile, output_path: &Path) -> Result<usize> {
    let cookies = match &profile.kind {
        BrowserKind::Firefox => read_firefox_cookies(&profile.profile_dir)?,
        BrowserKind::Chrome | BrowserKind::Chromium | BrowserKind::Brave => {
            read_chrome_cookies(&profile.profile_dir)?
        }
    };

    if cookies.is_empty() {
        return Ok(0);
    }

    write_netscape_cookies(&cookies, output_path)?;
    Ok(cookies.len())
}

// ---------------------------------------------------------------------------
// Firefox
// ---------------------------------------------------------------------------

fn read_firefox_cookies(profile_dir: &Path) -> Result<Vec<RawCookie>> {
    let db_path = profile_dir.join("cookies.sqlite");
    if !db_path.exists() {
        tracing::warn!("[Cookies] Firefox cookies.sqlite not found");
        return Ok(vec![]);
    }

    // Copy to temp file — Firefox holds an exclusive lock while running.
    let temp = std::env::temp_dir().join("muxspace-ff-cookies.sqlite");
    std::fs::copy(&db_path, &temp)?;

    let conn = rusqlite::Connection::open_with_flags(
        &temp,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    let mut stmt = conn.prepare(
        "SELECT name, value, host, path, expiry, isSecure FROM moz_cookies",
    )?;

    let cookies: Vec<RawCookie> = stmt
        .query_map([], |row| {
            Ok(RawCookie {
                name: row.get(0)?,
                value: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                expiry: row.get(4)?,
                secure: row.get::<_, i32>(5)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let _ = std::fs::remove_file(&temp);
    tracing::info!("[Cookies] Read {} Firefox cookies", cookies.len());
    Ok(cookies)
}

// ---------------------------------------------------------------------------
// Chrome / Chromium / Brave
// ---------------------------------------------------------------------------

fn read_chrome_cookies(profile_dir: &Path) -> Result<Vec<RawCookie>> {
    let db_path = profile_dir.join("Cookies");
    if !db_path.exists() {
        tracing::warn!("[Cookies] Chrome Cookies DB not found");
        return Ok(vec![]);
    }

    // Copy to temp file — Chrome also holds a lock.
    let temp = std::env::temp_dir().join("muxspace-chrome-cookies.sqlite");
    std::fs::copy(&db_path, &temp)?;

    let conn = rusqlite::Connection::open_with_flags(
        &temp,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    let mut stmt = conn.prepare(
        "SELECT name, value, encrypted_value, host_key, path, expires_utc, is_secure \
         FROM cookies",
    )?;

    let mut cookies = Vec::new();
    let mut skipped_v11 = 0u32;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Vec<u8>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i32>(6)?,
        ))
    })?;

    for row in rows.flatten() {
        let (name, value, encrypted_value, domain, path, expires_utc, is_secure) = row;

        // Chrome timestamps: microseconds since 1601-01-01 (Windows epoch)
        let expiry = if expires_utc > 0 {
            (expires_utc / 1_000_000) - 11_644_473_600
        } else {
            0
        };

        // Try plain value first, then attempt v10 decryption
        let final_value = if !value.is_empty() {
            value
        } else if !encrypted_value.is_empty() {
            match decrypt_chrome_cookie(&encrypted_value) {
                Some(v) => v,
                None => {
                    skipped_v11 += 1;
                    continue;
                }
            }
        } else {
            continue;
        };

        cookies.push(RawCookie { domain, path, secure: is_secure != 0, expiry, name, value: final_value });
    }

    let _ = std::fs::remove_file(&temp);

    if skipped_v11 > 0 {
        tracing::warn!(
            "[Cookies] Skipped {skipped_v11} Chrome cookies (v11 keyring encryption)"
        );
    }
    tracing::info!("[Cookies] Read {} Chrome cookies", cookies.len());
    Ok(cookies)
}

// ---------------------------------------------------------------------------
// Chrome v10 decryption: PBKDF2(SHA1, "peanuts", "saltysalt", 1) → AES-128-CBC
// ---------------------------------------------------------------------------

fn decrypt_chrome_cookie(encrypted: &[u8]) -> Option<String> {
    if encrypted.len() < 4 {
        return None;
    }
    match &encrypted[..3] {
        b"v10" => decrypt_v10(&encrypted[3..]),
        b"v11" => None, // Needs system keyring — not yet supported
        _ => None,
    }
}

fn decrypt_v10(ciphertext: &[u8]) -> Option<String> {
    use aes::Aes128;
    use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};

    type Aes128CbcDec = cbc::Decryptor<Aes128>;

    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(b"peanuts", b"saltysalt", 1, &mut key);

    let iv = [b' '; 16]; // 16 ASCII spaces
    let mut buf = ciphertext.to_vec();

    let pt = Aes128CbcDec::new_from_slices(&key, &iv)
        .ok()?
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .ok()?;

    String::from_utf8(pt.to_vec()).ok()
}

// ---------------------------------------------------------------------------
// Netscape cookie file writer
// ---------------------------------------------------------------------------

fn write_netscape_cookies(cookies: &[RawCookie], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut f = std::fs::File::create(path)?;
    writeln!(f, "# Netscape HTTP Cookie File")?;
    writeln!(f, "# Imported by Muxspace from user's default browser")?;
    writeln!(f)?;

    for c in cookies {
        let subdomain = if c.domain.starts_with('.') { "TRUE" } else { "FALSE" };
        let secure = if c.secure { "TRUE" } else { "FALSE" };
        writeln!(f, "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            c.domain, subdomain, c.path, secure, c.expiry, c.name, c.value)?;
    }

    Ok(())
}
