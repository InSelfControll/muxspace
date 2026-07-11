mod lifecycle;
mod navigation;
mod persistence;
mod runtime;
mod types;

pub use types::{AppState, Pane, PaneKind, Project, Workspace};

fn new_pane_id() -> String {
    static NEXT_PANE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "pane-{}-{}",
        chrono::Utc::now().timestamp_millis(),
        NEXT_PANE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaneCleanup {
    pane_id: String,
    is_browser: bool,
    pty_id: Option<String>,
}

impl From<&Pane> for PaneCleanup {
    fn from(pane: &Pane) -> Self {
        Self {
            pane_id: pane.id.clone(),
            is_browser: matches!(pane.kind, PaneKind::Browser { .. }),
            pty_id: pane.pty_id.clone(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new_blocking()
    }
}

#[cfg(test)]
mod tests;
