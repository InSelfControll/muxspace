use super::{write_netscape_cookies, RawCookie};
use std::fs;

#[cfg(unix)]
#[test]
fn cookie_store_is_owner_only() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("muxspace-cookie-test-{}", std::process::id()));
    let path = dir.join("cookies.txt");
    let cookie = RawCookie {
        domain: ".example.com".to_string(),
        path: "/".to_string(),
        secure: true,
        http_only: true,
        expiry: 1,
        name: "session".to_string(),
        value: "secret".to_string(),
    };

    write_netscape_cookies(&[cookie], &path)?;
    assert_eq!(fs::metadata(&path)?.permissions().mode() & 0o777, 0o600);
    fs::remove_dir_all(dir)?;
    Ok(())
}
