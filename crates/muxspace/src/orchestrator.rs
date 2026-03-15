use anyhow::Result;
use crate::types::{BrowserKind, ToolKind, WorkspaceConfig};
use std::process::{Child, Command};

/// Spawns all tools and browser windows for a workspace config.
/// Returns PIDs of all launched processes.
pub fn launch_external(cfg: &WorkspaceConfig) -> Result<Vec<u32>> {
    let mut pids = Vec::new();

    // ── Tools (AI assistants, CLI editors, GUI IDEs) ───────────────────────
    for tool in &cfg.tools {
        let cwd = shellexpand::tilde(&tool.path.to_string_lossy()).into_owned();
        let mut args: Vec<String> = tool.args.clone();

        match tool.kind {
            ToolKind::AiAssistant => {
                // Claude Code and similar CLI AI assistants run inside a terminal pane.
                // We launch them via the preferred terminal so they have a PTY.
                // For GUI assistants (Cursor) we launch directly.
                if is_gui_app(&tool.app) {
                    args.insert(0, cwd.clone());
                    let child = spawn_with_args(&tool.app, &args)?;
                    let pid = child.id();
                    tracing::info!("Launched AI assistant '{}' (gui, pid {})", tool.app, pid);
                    pids.push(pid);
                    std::mem::forget(child);
                } else {
                    // CLI AI tools (claude, aider, etc.) are spawned in a pane PTY;
                    // record intent but don't fork here — the PTY manager owns them.
                    tracing::info!(
                        "AI assistant '{}' will be launched in a PTY pane (cwd: {})",
                        tool.app, cwd
                    );
                }
            }

            ToolKind::CliEditor => {
                // CLI editors (nvim, helix, emacs -nw, micro, …) run inside PTY panes.
                tracing::info!(
                    "CLI editor '{}' will be launched in a PTY pane (cwd: {})",
                    tool.app, cwd
                );
            }

            ToolKind::GuiIde => {
                // GUI IDEs: open directly, passing the project path as the first arg.
                let mut full_args = vec![cwd.clone()];
                full_args.extend(args);
                let child = spawn_with_args(&tool.app, &full_args)?;
                let pid = child.id();
                tracing::info!("Launched GUI IDE '{}' (pid {})", tool.app, pid);
                pids.push(pid);
                std::mem::forget(child);
            }
        }
    }

    // ── Legacy single-editor field ─────────────────────────────────────────
    if let Some(editor) = &cfg.editor {
        let path = shellexpand::tilde(&editor.path.to_string_lossy()).into_owned();
        let child = spawn_with_args(&editor.app, &[path])?;
        let pid = child.id();
        tracing::info!("Launched legacy editor '{}' (pid {})", editor.app, pid);
        pids.push(pid);
        std::mem::forget(child);
    }

    // ── Browser with session/profile preservation ──────────────────────────
    if let Some(browser) = &cfg.browser {
        let browser_pids = launch_browser(browser)?;
        pids.extend(browser_pids);
    }

    Ok(pids)
}

/// Sends SIGTERM to each tracked external PID.
pub fn shutdown_external(pids: &[u32]) {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    for &pid in pids {
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
        tracing::info!("Sent SIGTERM to pid {}", pid);
    }
}

// ── Browser launch with profile preservation ──────────────────────────────────

fn launch_browser(cfg: &crate::types::BrowserConfig) -> Result<Vec<u32>> {
    let mut pids = Vec::new();
    let urls: Vec<&str> = cfg.urls.iter().map(String::as_str).collect();

    match &cfg.kind {
        BrowserKind::Chrome | BrowserKind::Chromium => {
            let bin = if cfg.kind == BrowserKind::Chromium { "chromium" } else { "google-chrome" };
            // Use the user's default profile unless overridden.
            let profile_dir = cfg.profile.as_deref().unwrap_or("Default");
            let user_data_dir = chrome_user_data_dir();
            let mut args = vec![
                format!("--user-data-dir={}", user_data_dir.display()),
                format!("--profile-directory={}", profile_dir),
            ];
            args.extend(urls.iter().map(|u| u.to_string()));
            let child = spawn_with_args(bin, &args)?;
            let pid = child.id();
            tracing::info!("Launched {} with profile '{}' (pid {})", bin, profile_dir, pid);
            pids.push(pid);
            std::mem::forget(child);
        }

        BrowserKind::Firefox => {
            let profile = cfg.profile.as_deref().unwrap_or("default");
            let mut args = vec!["-P".to_string(), profile.to_string(), "--no-remote".to_string()];
            args.extend(urls.iter().map(|u| u.to_string()));
            let child = spawn_with_args("firefox", &args)?;
            let pid = child.id();
            tracing::info!("Launched Firefox with profile '{}' (pid {})", profile, pid);
            pids.push(pid);
            std::mem::forget(child);
        }

        BrowserKind::Default => {
            // xdg-open each URL; the OS uses the user's default browser as-is,
            // which inherits its own session state naturally.
            for url in &cfg.urls {
                let child = spawn_with_args("xdg-open", &[url])?;
                let pid = child.id();
                tracing::info!("xdg-open '{}' (pid {})", url, pid);
                pids.push(pid);
                std::mem::forget(child);
            }
        }
    }

    Ok(pids)
}

/// Returns the Chrome/Chromium user-data directory for the current user.
fn chrome_user_data_dir() -> std::path::PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("google-chrome")
}

/// Returns true for apps that are GUI binaries (not CLI tools).
fn is_gui_app(app: &str) -> bool {
    matches!(app, "cursor" | "zed" | "windsurf" | "void")
}

fn spawn_with_args<S: AsRef<str>>(program: &str, args: &[S]) -> Result<Child> {
    Command::new(program)
        .args(args.iter().map(|a| a.as_ref()))
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn '{}': {}", program, e))
}
