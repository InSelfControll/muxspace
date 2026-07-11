use super::*;
use std::path::PathBuf;

#[test]
fn project_creation() {
    let mut state = AppState::empty_for_tests();
    let project = state.create_project("test-project", PathBuf::from("/tmp"));
    assert_eq!(project.name, "test-project");
    assert_eq!(state.projects.len(), 1);
    assert_eq!(state.projects[0].workspaces.len(), 1);
}

#[test]
fn workspace_navigation() {
    let mut state = AppState::empty_for_tests();
    let project = state.create_project("test", PathBuf::from("/tmp"));
    state.active_project_id = Some(project.id);
    state.add_workspace("ws-1");
    state.add_workspace("ws-2");
    state.next_workspace();
    assert_eq!(
        state.active_project().map(|p| p.active_workspace_idx),
        Some(1)
    );
    state.prev_workspace();
    assert_eq!(
        state.active_project().map(|p| p.active_workspace_idx),
        Some(0)
    );
}

#[test]
fn workspace_wrapping() {
    let mut state = AppState::empty_for_tests();
    let project = state.create_project("test", PathBuf::from("/tmp"));
    state.active_project_id = Some(project.id);
    state.add_workspace("ws-1");
    state.next_workspace();
    state.next_workspace();
    assert_eq!(
        state.active_project().map(|p| p.active_workspace_idx),
        Some(0)
    );
    state.prev_workspace();
    assert_eq!(
        state.active_project().map(|p| p.active_workspace_idx),
        Some(1)
    );
}

#[test]
fn workspace_bounds_are_checked() {
    let mut state = AppState::empty_for_tests();
    let project = state.create_project("test", PathBuf::from("/tmp"));
    state.active_project_id = Some(project.id);
    state.goto_workspace(999);
    state.remove_workspace(999);
    assert_eq!(
        state.active_project().map(|p| p.active_workspace_idx),
        Some(0)
    );
}

#[test]
fn add_workspace_with_empty_project() {
    let mut state = AppState::empty_for_tests();
    state.projects.push(Project {
        id: "empty".to_string(),
        name: "Empty".to_string(),
        workspaces: Vec::new(),
        active_workspace_idx: 0,
    });
    state.active_project_id = Some("empty".to_string());
    state.add_workspace("new-ws");
    assert_eq!(state.active_project().map(|p| p.workspaces.len()), Some(1));
}

#[test]
fn focus_navigation() {
    let mut state = AppState::empty_for_tests();
    let project = state.create_project("test", PathBuf::from("/tmp"));
    state.active_project_id = Some(project.id);
    let first_pane = state.active_pane_ids().into_iter().next();
    state.add_terminal_pane(None);
    let focused_after_add = state.focused_pane_id.clone();
    assert_ne!(focused_after_add, first_pane);
    state.focus_next_pane();
    assert_eq!(state.focused_pane_id, first_pane);
}

#[test]
fn pane_cleanup_decision_marks_browser_and_pty() {
    let pane = Pane {
        id: "browser-pane".to_string(),
        kind: PaneKind::Browser {
            url: String::new(),
            tabs: vec!["https://example.com".to_string()],
            active_tab: 0,
            profile: None,
        },
        pty_id: Some("pty-1".to_string()),
        custom_name: None,
    };
    let cleanup = PaneCleanup::from(&pane);
    assert!(cleanup.is_browser);
    assert_eq!(cleanup.pty_id.as_deref(), Some("pty-1"));
}

#[test]
fn pane_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let pane = Pane {
        id: "test-pane".to_string(),
        kind: PaneKind::Terminal {
            command: Some("bash".to_string()),
        },
        pty_id: Some("pty-1".to_string()),
        custom_name: None,
    };
    let json = serde_json::to_string(&pane)?;
    let deserialized: Pane = serde_json::from_str(&json)?;
    assert_eq!(pane, deserialized);
    Ok(())
}

#[test]
fn browser_pane_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let pane = Pane {
        id: "browser-pane".to_string(),
        kind: PaneKind::Browser {
            url: String::new(),
            tabs: vec!["https://example.com".to_string()],
            active_tab: 0,
            profile: None,
        },
        pty_id: None,
        custom_name: None,
    };
    let json = serde_json::to_string(&pane)?;
    let deserialized: Pane = serde_json::from_str(&json)?;
    assert_eq!(pane, deserialized);
    Ok(())
}
