use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

static TMUX_AVAILABLE: LazyLock<bool> = LazyLock::new(|| which::which("tmux").is_ok());
static TMUX_CONFIG: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("muxspace");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        tracing::warn!("Failed to create muxspace config dir: {err}");
    }
    let path = dir.join("tmux.conf");
    if !path.exists() {
        let config = "set -g status off\nset -g escape-time 0\nset -g prefix C-]\nunbind C-b\nset -g mouse on\nset -g history-limit 50000\n";
        if let Err(err) = std::fs::write(&path, config) {
            tracing::warn!("Failed to write tmux config: {err}");
        }
    }
    path
});

pub struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    #[allow(dead_code)]
    master: Box<dyn MasterPty + Send>,
}

pub fn kill_tmux_session(pane_id: &str) {
    if !*TMUX_AVAILABLE {
        return;
    }
    let session_name = format!("mux-{pane_id}");
    if let Err(err) = std::process::Command::new("tmux")
        .args(["-L", "muxspace", "kill-session", "-t", &session_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        tracing::debug!("Failed to kill tmux session {session_name}: {err}");
    }
}

impl PtySession {
    pub fn spawn(
        pane_id: &str,
        cwd: &Path,
        command: Option<&str>,
        rows: u16,
        cols: u16,
    ) -> Result<(Self, std::sync::mpsc::Receiver<Vec<u8>>)> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("opening PTY")?;
        let mut cmd = build_command(pane_id, command);
        cmd.cwd(cwd);
        let _child = pair.slave.spawn_command(cmd).context("spawning shell")?;
        let mut reader = pair.master.try_clone_reader().context("cloning reader")?;
        let writer = pair.master.take_writer().context("taking writer")?;
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) if tx.send(buf[..n].to_vec()).is_err() => break,
                    Ok(_) => {}
                }
            }
        });

        Ok((
            Self {
                writer: Arc::new(Mutex::new(writer)),
                master: pair.master,
            },
            rx,
        ))
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.writer
            .lock()
            .map_err(|err| anyhow::anyhow!("Mutex poisoned: {err}"))?
            .write_all(data)?;
        Ok(())
    }
}

fn build_command(pane_id: &str, command: Option<&str>) -> CommandBuilder {
    if *TMUX_AVAILABLE {
        let session_name = format!("mux-{pane_id}");
        let config = TMUX_CONFIG.to_string_lossy().to_string();
        let mut cmd = CommandBuilder::new("tmux");
        cmd.args([
            "-L",
            "muxspace",
            "-f",
            &config,
            "new-session",
            "-A",
            "-s",
            &session_name,
        ]);
        if let Some(initial_cmd) = command {
            cmd.arg(initial_cmd);
        }
        tracing::info!("PTY using tmux session '{session_name}'");
        return cmd;
    }
    let shell = command
        .map(str::to_string)
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/sh".to_string());
    CommandBuilder::new(&shell)
}
