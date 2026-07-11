mod files;
#[cfg(test)]
mod files_tests;
mod installed;

pub use files::{
    is_text_file, list_workspace_entries, open_document, open_in_editor, relative_label,
    save_document, EditorDocument, EditorFileEntry, FileCheck, MAX_TEXT_BYTES,
};
pub use installed::{
    describe_launch_mode, detect_installed_editors, EditorLaunchMode, InstalledEditor,
};
