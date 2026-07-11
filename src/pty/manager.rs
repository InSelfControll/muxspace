use super::session::{kill_tmux_session, PtySession};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

pub static PTY_MANAGER: LazyLock<Mutex<PtyManager>> =
    LazyLock::new(|| Mutex::new(PtyManager::new()));

#[derive(Default)]
pub struct PtyManager {
    sessions: HashMap<String, PtySession>,
    receivers: HashMap<String, std::sync::mpsc::Receiver<Vec<u8>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn_for_pane(&mut self, pane_id: &str, cwd: &Path, cmd: Option<&str>) -> Result<()> {
        if self.sessions.contains_key(pane_id) {
            return Ok(());
        }
        let (session, rx) = PtySession::spawn(pane_id, cwd, cmd, 24, 80)?;
        self.sessions.insert(pane_id.to_string(), session);
        self.receivers.insert(pane_id.to_string(), rx);
        tracing::info!("Spawned PTY for pane {pane_id}");
        Ok(())
    }

    pub fn write_to_pane(&self, pane_id: &str, data: &[u8]) -> Result<()> {
        if let Some(session) = self.sessions.get(pane_id) {
            session.write(data)?;
        }
        Ok(())
    }

    pub fn drain_output(&self, pane_id: &str) -> Vec<u8> {
        let mut data = Vec::new();
        if let Some(rx) = self.receivers.get(pane_id) {
            while let Ok(chunk) = rx.try_recv() {
                data.extend(chunk);
            }
        }
        data
    }

    pub fn active_pane_ids(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    pub fn remove_pane(&mut self, pane_id: &str) {
        self.sessions.remove(pane_id);
        self.receivers.remove(pane_id);
        kill_tmux_session(pane_id);
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty() && self.receivers.is_empty()
    }
}
