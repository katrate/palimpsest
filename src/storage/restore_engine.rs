use crate::types::{FileEntry, FileStatus};
use std::collections::HashSet;
use std::path::Path;

/// Restore a directory to the state described by file entries,
/// also deleting any files on disk that are not in the snapshot.
pub fn restore_to_snapshot(
    target_dir: &Path,
    entries: &[FileEntry],
    palin_name: &str,
    dry_run: bool,
) -> anyhow::Result<Vec<String>> {
    let mut actions = Vec::new();

    // Collect all paths tracked in this snapshot for quick lookup
    let tracked_paths: HashSet<&str> = entries.iter().map(|e| e.file_path.as_str()).collect();

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

    // Walk the target directory and delete files not tracked in this snapshot
    delete_untracked_files(target_dir, &tracked_paths, target_dir, dry_run, &mut actions)?;

    Ok(actions)
}

/// Recursively delete files in `current_dir` that are not in `tracked_paths`
fn delete_untracked_files(
    root: &Path,
    tracked_paths: &HashSet<&str>,
    current_dir: &Path,
    dry_run: bool,
    actions: &mut Vec<String>,
) -> anyhow::Result<()> {
    if !current_dir.is_dir() {
        return Ok(());
    }

    let entries = match std::fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");

        if path.is_dir() {
            // Skip .git and other common control directories
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == ".git" || name == ".svn" || name == ".hg" || name == "node_modules" || name == "target" {
                continue;
            }
            delete_untracked_files(root, tracked_paths, &path, dry_run, actions)?;
            // Try to remove the directory if it's empty after cleanup
            if !dry_run {
                let _ = std::fs::remove_dir(&path);
            }
        } else if path.is_file() {
            if !tracked_paths.contains(relative.as_str()) {
                if !dry_run {
                    let _ = std::fs::remove_file(&path);
                }
                actions.push(format!("  - prune   {}", relative));
            }
        }
    }

    Ok(())
}

/// Preview what would change during a restore (dry-run)
pub fn preview_restore(
    target_dir: &Path,
    entries: &[FileEntry],
    palin_name: &str,
) -> anyhow::Result<Vec<String>> {
    restore_to_snapshot(target_dir, entries, palin_name, true)
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
