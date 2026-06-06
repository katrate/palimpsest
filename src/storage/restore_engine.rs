use crate::types::{FileEntry, FileStatus};
use std::path::Path;

/// Restore a directory to the state described by file entries
pub fn restore_to_snapshot(
    target_dir: &Path,
    entries: &[FileEntry],
    palin_name: &str,
    dry_run: bool,
) -> anyhow::Result<Vec<String>> {
    let mut actions = Vec::new();

    for entry in entries {
        let full_path = target_dir.join(&entry.file_path);

        match entry.status {
            FileStatus::Added | FileStatus::Modified | FileStatus::Unchanged => {
                if let Some(ref hash) = entry.ink_hash {
                    let content = crate::storage::ink::read_ink(palin_name, hash)?;

                    if !dry_run {
                        if let Some(parent) = full_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&full_path, &content)?;
                    }

                    actions.push(format!("  + restore  {}", entry.file_path));
                }
            }
            FileStatus::Deleted => {
                if !dry_run && full_path.exists() {
                    std::fs::remove_file(&full_path)?;
                }
                actions.push(format!("  - remove  {}", entry.file_path));
            }
        }
    }

    Ok(actions)
}

/// Preview what would change during a restore (dry-run)
pub fn preview_restore(
    target_dir: &Path,
    entries: &[FileEntry],
) -> anyhow::Result<Vec<String>> {
    restore_to_snapshot(target_dir, entries, "", true)
}

/// Count how many files would be affected by a restore
pub fn count_restore_changes(entries: &[FileEntry]) -> (usize, usize) {
    let restore_count = entries
        .iter()
        .filter(|e| matches!(e.status, FileStatus::Added | FileStatus::Modified | FileStatus::Unchanged))
        .count();
    let delete_count = entries
        .iter()
        .filter(|e| matches!(e.status, FileStatus::Deleted))
        .count();
    (restore_count, delete_count)
}
