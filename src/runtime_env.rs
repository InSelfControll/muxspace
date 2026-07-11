use std::path::{Path, PathBuf};

pub fn configure_runtime_modules() {
    // WebKitGTK's DMA-BUF renderer can produce blank or partially painted views when a
    // native WebView is composited over Dioxus' WebKit view. Select the shared-memory
    // renderer before either WebKit process is initialized.
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    if !env_has_module("GIO_EXTRA_MODULES", "libgiognutls.so") {
        if let Some(path) = find_gio_modules() {
            std::env::set_var("GIO_EXTRA_MODULES", path);
        }
    }
    std::env::set_var("GIO_USE_TLS", "gnutls");
    if !env_has_module("GST_PLUGIN_SYSTEM_PATH_1_0", "libgstapp.so") {
        if let Some(plugins) = option_env!("GST_PLUGIN_SYSTEM_PATH_1_0") {
            std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", plugins);
        }
    }
}

pub fn gio_modules_path() -> Option<PathBuf> {
    std::env::var_os("GIO_EXTRA_MODULES").and_then(|paths| {
        std::env::split_paths(&paths).find(|path| contains_module(path, "libgiognutls.so"))
    })
}

fn find_gio_modules() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = option_env!("GIO_EXTRA_MODULES") {
        candidates.extend(std::env::split_paths(path));
    }
    candidates.extend([
        PathBuf::from("/run/current-system/sw/lib/gio/modules"),
        PathBuf::from("/usr/lib/gio/modules"),
        PathBuf::from("/usr/lib/x86_64-linux-gnu/gio/modules"),
    ]);
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".nix-profile/lib/gio/modules"));
        candidates.push(
            PathBuf::from("/etc/profiles/per-user")
                .join(std::env::var("USER").unwrap_or_default())
                .join("lib/gio/modules"),
        );
    }
    if let Ok(paths) = glob::glob("/nix/store/*-glib-networking-*/lib/gio/modules") {
        candidates.extend(paths.flatten());
    }
    candidates
        .into_iter()
        .find(|path| contains_module(path, "libgiognutls.so"))
}

fn env_has_module(variable: &str, module: &str) -> bool {
    std::env::var_os(variable)
        .map(|paths| std::env::split_paths(&paths).any(|path| contains_module(&path, module)))
        .unwrap_or(false)
}

fn contains_module(path: &Path, module: &str) -> bool {
    path.join(module).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_gio_path_contains_tls_backend_when_available() {
        configure_runtime_modules();
        assert_eq!(
            std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").as_deref(),
            Ok("1")
        );
        if let Some(path) = gio_modules_path() {
            assert!(contains_module(&path, "libgiognutls.so"));
        }
    }
}
