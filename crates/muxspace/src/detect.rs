use anyhow::{Context, Result};
use dialoguer::Select;
use std::collections::HashMap;
use which::which;

use crate::types::{GlobalConfig, ToolCategory, ToolInfo, config_path, workspaces_dir};

// ── Known Tools Registry ──────────────────────────────────────────────────────

const KNOWN_TERMINALS: &[&str] = &[
    "alacritty", "kitty", "wezterm", "konsole", "gnome-terminal",
    "xfce4-terminal", "xterm", "urxvt",
];

const KNOWN_CLI_EDITORS: &[&str] = &[
    "nvim", "vim", "helix", "hx", "emacs", "micro", "nano",
];

const KNOWN_GUI_IDES: &[&str] = &[
    "code", "cursor", "windsurf", "zed", "idea", "goland", "rustrover",
    "pycharm", "webstorm", "clion", "phpstorm", "rubymine",
];

const KNOWN_AI_ASSISTANTS: &[&str] = &[
    "claude", "aider", "codex",
];

const KNOWN_BROWSERS: &[&str] = &[
    "google-chrome", "chromium", "firefox", "brave", "edge",
];

// ── Config Loading/Saving ─────────────────────────────────────────────────────

pub fn load_or_init_config() -> Result<GlobalConfig> {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config from {}", path.display()))?;
        let cfg: GlobalConfig = serde_yaml::from_str(&content).context("parsing config YAML")?;
        return Ok(cfg);
    }
    
    // First run — detect tools and create config
    let cfg = first_time_setup()?;
    save_config(&cfg)?;
    Ok(cfg)
}

pub fn save_config(cfg: &GlobalConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating config directory")?;
    }
    // Ensure workspaces directory exists
    let _ = std::fs::create_dir_all(workspaces_dir());
    
    let yaml = serde_yaml::to_string(cfg).context("serializing config")?;
    std::fs::write(&path, yaml)
        .with_context(|| format!("writing config to {}", path.display()))?;
    Ok(())
}

// ── First Time Setup ──────────────────────────────────────────────────────────

fn first_time_setup() -> Result<GlobalConfig> {
    println!("Welcome to Muxspace! Detecting your development environment...\n");
    
    let detected_tools = detect_all_tools();
    
    // Show what was detected
    print_detected_summary(&detected_tools);
    
    // Ask for preferred terminal if multiple found
    let preferred_terminal = select_preferred_terminal(&detected_tools)?;
    
    Ok(GlobalConfig {
        preferred_terminal,
        workspaces: vec![],
        sync_dir: None,
        detected_tools,
    })
}

// ── Tool Detection ────────────────────────────────────────────────────────────

pub fn detect_all_tools() -> Vec<ToolInfo> {
    let mut tools = Vec::new();
    
    for name in KNOWN_TERMINALS {
        if let Ok(path) = which(name) {
            tools.push(ToolInfo {
                name: name.to_string(),
                category: ToolCategory::Terminal,
                path,
                args: vec![],
            });
        }
    }
    
    for name in KNOWN_CLI_EDITORS {
        if let Ok(path) = which(name) {
            tools.push(ToolInfo {
                name: name.to_string(),
                category: ToolCategory::CliEditor,
                path,
                args: vec![],
            });
        }
    }
    
    for name in KNOWN_GUI_IDES {
        if let Ok(path) = which(name) {
            tools.push(ToolInfo {
                name: name.to_string(),
                category: ToolCategory::GuiIde,
                path,
                args: vec![],
            });
        }
    }
    
    for name in KNOWN_AI_ASSISTANTS {
        if let Ok(path) = which(name) {
            tools.push(ToolInfo {
                name: name.to_string(),
                category: ToolCategory::AiAssistant,
                path,
                args: vec![],
            });
        }
    }
    
    for name in KNOWN_BROWSERS {
        if let Ok(path) = which(name) {
            tools.push(ToolInfo {
                name: name.to_string(),
                category: ToolCategory::Browser,
                path,
                args: vec![],
            });
        }
    }
    
    tools
}

pub fn detect_tools_by_category(category: ToolCategory) -> Vec<ToolInfo> {
    detect_all_tools()
        .into_iter()
        .filter(|t| t.category == category)
        .collect()
}

pub fn is_tool_available(name: &str) -> bool {
    which(name).is_ok()
}

// ── Utility Functions ────────────────────────────────────────────────────────

fn print_detected_summary(tools: &[ToolInfo]) {
    let by_category: HashMap<ToolCategory, Vec<&ToolInfo>> = 
        tools.iter().fold(HashMap::new(), |mut acc, tool| {
            acc.entry(tool.category.clone()).or_default().push(tool);
            acc
        });
    
    for (cat, cat_tools) in &by_category {
        let cat_name = match cat {
            ToolCategory::Terminal => "Terminals",
            ToolCategory::CliEditor => "CLI Editors",
            ToolCategory::GuiIde => "GUI IDEs",
            ToolCategory::AiAssistant => "AI Assistants",
            ToolCategory::Browser => "Browsers",
        };
        println!("  {}: {}", cat_name, 
            cat_tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", "));
    }
    println!();
}

fn select_preferred_terminal(tools: &[ToolInfo]) -> Result<Option<String>> {
    let terminals: Vec<&ToolInfo> = tools
        .iter()
        .filter(|t| t.category == ToolCategory::Terminal)
        .collect();
    
    match terminals.len() {
        0 => {
            eprintln!("No known terminal emulator found on PATH.");
            Ok(None)
        }
        1 => {
            println!("Detected terminal: {}", terminals[0].name);
            Ok(Some(terminals[0].name.clone()))
        }
        _ => {
            let names: Vec<&str> = terminals.iter().map(|t| t.name.as_str()).collect();
            let idx = Select::new()
                .with_prompt("Multiple terminals found — select your preferred one")
                .items(&names)
                .default(0)
                .interact()
                .context("terminal selection prompt")?;
            Ok(Some(terminals[idx].name.clone()))
        }
    }
}

/// Quick check for specific tool categories
pub fn get_preferred_editor(tools: &[ToolInfo]) -> Option<&ToolInfo> {
    tools.iter().find(|t| t.category == ToolCategory::CliEditor)
}

pub fn get_preferred_ide(tools: &[ToolInfo]) -> Option<&ToolInfo> {
    tools.iter().find(|t| t.category == ToolCategory::GuiIde)
}

pub fn get_preferred_ai_assistant(tools: &[ToolInfo]) -> Option<&ToolInfo> {
    tools.iter().find(|t| t.category == ToolCategory::AiAssistant)
}
