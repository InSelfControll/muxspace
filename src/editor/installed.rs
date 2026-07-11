use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledEditor {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub icon: String,
    pub launch_mode: EditorLaunchMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorLaunchMode {
    Gui,
    Terminal,
    Both,
}

pub fn detect_installed_editors() -> Vec<InstalledEditor> {
    let candidates = [
        ("cursor", "Cursor", "cursor", "[C]", EditorLaunchMode::Gui),
        ("vscode", "VS Code", "code", "[V]", EditorLaunchMode::Gui),
        (
            "vscode-insiders",
            "VS Code Insiders",
            "code-insiders",
            "[VI]",
            EditorLaunchMode::Gui,
        ),
        (
            "vscodium",
            "VSCodium",
            "codium",
            "[VC]",
            EditorLaunchMode::Gui,
        ),
        ("t3code", "T3 Code", "t3code", "[T3]", EditorLaunchMode::Gui),
        (
            "opencode",
            "OpenCode",
            "opencode",
            "[O]",
            EditorLaunchMode::Both,
        ),
        ("zed", "Zed", "zed", "[Z]", EditorLaunchMode::Gui),
        ("lapce", "Lapce", "lapce", "[L]", EditorLaunchMode::Gui),
        ("helix", "Helix", "hx", "[H]", EditorLaunchMode::Terminal),
        (
            "neovim",
            "Neovim",
            "nvim",
            "[N]",
            EditorLaunchMode::Terminal,
        ),
        ("vim", "Vim", "vim", "[Vim]", EditorLaunchMode::Terminal),
        ("emacs", "Emacs", "emacs", "[E]", EditorLaunchMode::Gui),
        ("kate", "Kate", "kate", "[K]", EditorLaunchMode::Gui),
        (
            "sublime",
            "Sublime Text",
            "subl",
            "[S]",
            EditorLaunchMode::Gui,
        ),
        ("gedit", "Gedit", "gedit", "[G]", EditorLaunchMode::Gui),
    ];

    let mut installed = candidates
        .into_iter()
        .filter_map(|(id, name, executable, icon, launch_mode)| {
            which::which(executable)
                .ok()
                .map(|resolved| InstalledEditor {
                    id: id.to_string(),
                    name: name.to_string(),
                    executable: resolved.to_string_lossy().to_string(),
                    icon: icon.to_string(),
                    launch_mode,
                })
        })
        .collect::<Vec<_>>();

    installed.sort_by(|a, b| a.name.cmp(&b.name));
    installed
}

pub fn describe_launch_mode(mode: &EditorLaunchMode) -> &'static str {
    match mode {
        EditorLaunchMode::Gui => "GUI app",
        EditorLaunchMode::Terminal => "Terminal app",
        EditorLaunchMode::Both => "GUI or terminal",
    }
}
