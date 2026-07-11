use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MAX_TEXT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorFileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorDocument {
    pub path: PathBuf,
    pub title: String,
    pub contents: String,
    pub permissions: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCheck {
    Text,
    Directory,
    OutsideWorkspace,
    TooLarge,
    Binary,
}

pub fn list_workspace_entries(root: &Path) -> Result<Vec<EditorFileEntry>> {
    let root = canonical_root(root)?;
    let mut entries = fs::read_dir(&root)
        .with_context(|| format!("failed to read workspace directory {}", root.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip(&name) {
                return None;
            }
            Some(EditorFileEntry {
                name,
                is_dir: path.is_dir(),
                path,
            })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(entries)
}

pub fn open_document(root: &Path, path: &Path) -> Result<EditorDocument> {
    match check_file(root, path)? {
        FileCheck::Text => {}
        FileCheck::Directory => bail!("directories cannot be opened in the built-in editor"),
        FileCheck::OutsideWorkspace => bail!("file is outside the workspace"),
        FileCheck::TooLarge => bail!("file is larger than {} bytes", MAX_TEXT_BYTES),
        FileCheck::Binary => bail!("file does not look like text"),
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read file {}", path.display()))?;
    let permissions = permissions(path).ok();
    Ok(EditorDocument {
        path: path.to_path_buf(),
        title: title_for(path),
        contents,
        permissions,
    })
}

pub fn save_document(
    root: &Path,
    path: &Path,
    contents: &str,
    permissions: Option<u32>,
) -> Result<()> {
    let root = canonical_root(root)?;
    let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !target.starts_with(&root) {
        bail!("refusing to save outside workspace");
    }
    if contents.len() as u64 > MAX_TEXT_BYTES {
        bail!("refusing to save more than {} bytes", MAX_TEXT_BYTES);
    }

    let parent = target
        .parent()
        .ok_or_else(|| anyhow!("file has no parent directory"))?;
    let tmp = tempfile_path(parent, &target)?;
    let write_result = write_temp(&tmp, contents, permissions).and_then(|_| {
        fs::rename(&tmp, &target).with_context(|| format!("failed to replace {}", target.display()))
    });

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_result
}

pub fn open_in_editor(editor_cmd: &str, workspace_root: &Path, file: Option<&Path>) -> Result<()> {
    let root = canonical_root(workspace_root)?;
    let mut command = Command::new(editor_cmd);
    command.current_dir(&root);

    if let Some(path) = file {
        let target = path
            .canonicalize()
            .with_context(|| format!("failed to resolve file before opening {}", path.display()))?;
        if !target.starts_with(&root) {
            bail!("refusing to open file outside workspace");
        }
        command.arg(target);
    } else {
        command.arg(root);
    }

    command
        .spawn()
        .with_context(|| format!("failed to open {editor_cmd}"))?;
    Ok(())
}

pub fn is_text_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "rs" | "toml"
                | "md"
                | "txt"
                | "json"
                | "yaml"
                | "yml"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "html"
                | "css"
                | "go"
                | "py"
                | "sh"
                | "sql"
        ) | None
    )
}

pub fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

pub fn check_file(root: &Path, path: &Path) -> Result<FileCheck> {
    let root = canonical_root(root)?;
    let path = path
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", path.display()))?;
    if !path.starts_with(&root) {
        return Ok(FileCheck::OutsideWorkspace);
    }
    let meta = fs::metadata(&path)?;
    if meta.is_dir() {
        return Ok(FileCheck::Directory);
    }
    if meta.len() > MAX_TEXT_BYTES {
        return Ok(FileCheck::TooLarge);
    }
    if !is_text_file(&path) {
        return Ok(FileCheck::Binary);
    }
    let bytes = fs::read(&path)?;
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        return Ok(FileCheck::Binary);
    }
    Ok(FileCheck::Text)
}

fn canonical_root(root: &Path) -> Result<PathBuf> {
    root.canonicalize()
        .with_context(|| format!("failed to resolve workspace root {}", root.display()))
}

fn title_for(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn write_temp(path: &Path, contents: &str, permissions: Option<u32>) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    set_permissions(path, permissions)?;
    Ok(())
}

fn tempfile_path(parent: &Path, target: &Path) -> Result<PathBuf> {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("target has no file name"))?;
    Ok(parent.join(format!(".{name}.muxspace-tmp-{}", std::process::id())))
}

#[cfg(unix)]
fn permissions(path: &Path) -> Result<u32> {
    use std::os::unix::fs::PermissionsExt;
    Ok(fs::metadata(path)?.permissions().mode())
}

#[cfg(not(unix))]
fn permissions(_path: &Path) -> Result<u32> {
    Ok(0)
}

#[cfg(unix)]
fn set_permissions(path: &Path, permissions: Option<u32>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = permissions {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions(_path: &Path, _permissions: Option<u32>) -> Result<()> {
    Ok(())
}

fn should_skip(name: &str) -> bool {
    matches!(name, ".git" | "target" | "node_modules")
}
