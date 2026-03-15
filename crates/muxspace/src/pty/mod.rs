use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

use crate::ansi::ScreenBuffer;

/// A live PTY pane: owns a PTY process and an ANSI screen buffer.
/// A background blocking task drains PTY output and updates the buffer.
pub struct PtyPane {
    pub screen: Arc<Mutex<ScreenBuffer>>,
    pub pid: u32,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
}

impl PtyPane {
    /// Spawn a shell (or `command`) in `cwd`, start background reader.
    /// `redraw_tx` is signalled whenever new output arrives.
    pub fn spawn(
        cwd: &Path,
        command: Option<&str>,
        rows: u16,
        cols: u16,
        redraw_tx: watch::Sender<()>,
    ) -> Result<Arc<Self>> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .context("opening pty pair")?;

        let shell = command
            .map(|s| s.to_string())
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/sh".to_string());

        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);

        let child = pair.slave.spawn_command(cmd).context("spawning shell")?;
        let pid = child.process_id().unwrap_or(0);
        tracing::info!("PTY spawned '{}' cwd='{}' pid={}", shell, cwd.display(), pid);

        // Extract writer and reader before consuming master into Mutex.
        let master = pair.master;
        let writer = master.take_writer().context("taking PTY writer")?;
        let mut reader = master.try_clone_reader().context("cloning PTY reader")?;

        let screen = Arc::new(Mutex::new(ScreenBuffer::new(rows as usize, cols as usize)));
        let screen_bg = Arc::clone(&screen);

        // Spawn background reader task
        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        screen_bg.lock().unwrap().process(&buf[..n]);
                        let _ = redraw_tx.send(());
                    }
                }
            }
        });

        Ok(Arc::new(PtyPane {
            screen,
            pid,
            writer: Mutex::new(writer),
            master: Mutex::new(master),
            child: Mutex::new(child),
        }))
    }

    /// Write bytes to the PTY (e.g., from keyboard input)
    pub fn write(&self, data: &[u8]) -> Result<()> {
        self.writer
            .lock()
            .unwrap()
            .write_all(data)
            .context("writing to PTY")
    }

    /// Resize the PTY and screen buffer
    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .lock()
            .unwrap()
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .context("resizing PTY")?;
        self.screen.lock().unwrap().resize(rows as usize, cols as usize);
        Ok(())
    }

    /// Check if the child process is still alive
    pub fn is_alive(&self) -> bool {
        matches!(self.child.lock().unwrap().try_wait(), Ok(None))
    }
}
