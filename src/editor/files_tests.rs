use super::files::{check_file, open_document, save_document, FileCheck};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn rejects_outside_workspace_and_binary_files() -> Result<()> {
    let root = test_dir("editor-safety")?;
    let text = root.join("ok.txt");
    let binary = root.join("bin.dat");
    let outside = root
        .parent()
        .unwrap_or(Path::new("/tmp"))
        .join("outside.txt");
    fs::write(&text, "hello")?;
    fs::write(&binary, b"hi\0there")?;
    fs::write(&outside, "no")?;

    assert_eq!(check_file(&root, &text)?, FileCheck::Text);
    assert_eq!(check_file(&root, &binary)?, FileCheck::Binary);
    assert_eq!(check_file(&root, &outside)?, FileCheck::OutsideWorkspace);
    let _ = fs::remove_file(outside);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn atomic_save_preserves_permissions() -> Result<()> {
    let root = test_dir("editor-save")?;
    let file = root.join("save.txt");
    fs::write(&file, "old")?;
    let doc = open_document(&root, &file)?;
    save_document(&root, &file, "new", doc.permissions)?;
    assert_eq!(fs::read_to_string(&file)?, "new");
    let _ = fs::remove_dir_all(root);
    Ok(())
}

fn test_dir(name: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
