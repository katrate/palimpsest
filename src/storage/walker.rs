use crate::types::{FileEntry, FileStatus};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::HashMap;
use std::path::Path;

/// Walk a directory and produce file entries, optionally comparing against previous entries
pub fn walk_directory(
    dir: &Path,
    exclude_patterns: &[String],
) -> anyhow::Result<Vec<WalkedFile>> {
    let glob_set = build_exclude_set(exclude_patterns)?;
    let mut files = Vec::new();
    walk_dir_recursive(dir, dir, &glob_set, &mut files, None, false)?;
    Ok(files)
}

/// Walk directory and produce file entries with status compared to previous epoch.
/// Skips reading content for files whose size+mtime match the previous entry,
/// and for the first snap (no previous entries) avoids hashing entirely.
pub fn walk_and_diff(
    dir: &Path,
    exclude_patterns: &[String],
    previous_entries: &[FileEntry],
) -> anyhow::Result<Vec<FileEntry>> {
    let is_first = previous_entries.is_empty();
    let glob_set = build_exclude_set(exclude_patterns)?;

    // Build a lookup: (path, size, mtime) -> previous entry's hash
    // Used to skip reading unchanged files in the walker
    let prev_meta: HashMap<(String, i64, String), String> = previous_entries
        .iter()
        .filter(|e| e.status != FileStatus::Deleted)
        .filter_map(|e| {
            e.modified_at.as_ref().map(|m| {
                ((e.file_path.clone(), e.file_size.unwrap_or(0), m.clone()),
                 e.ink_hash.clone().unwrap_or_default())
            })
        })
        .collect();

    let prev_meta_ref = if is_first { None } else { Some(&prev_meta) };

    let mut current_files = Vec::new();
    walk_dir_recursive(dir, dir, &glob_set, &mut current_files, prev_meta_ref, is_first)?;

    // Build map of previous entries by path for status comparison
    let prev_map: HashMap<&str, &FileEntry> = previous_entries
        .iter()
        .filter(|e| e.status != FileStatus::Deleted)
        .map(|e| (e.file_path.as_str(), e))
        .collect();

    let mut result = Vec::new();

    for wf in &current_files {
        let status = if let Some(prev) = prev_map.get(wf.relative_path.as_str()) {
            if prev.ink_hash.as_deref() != Some(&wf.hash) {
                FileStatus::Modified
            } else {
                FileStatus::Unchanged
            }
        } else {
            FileStatus::Added
        };

        result.push(FileEntry {
            id: 0,
            snapshot_id: 0,
            snapshot_type: crate::types::SnapshotType::Epoch,
            file_path: wf.relative_path.clone(),
            ink_hash: Some(wf.hash.clone()),
            file_size: Some(wf.size),
            modified_at: Some(wf.modified_at.clone()),
            status,
            is_binary: wf.is_binary,
        });
    }

    // Mark files that were deleted
    for (path, prev) in &prev_map {
        if !current_files.iter().any(|f| f.relative_path == **path) {
            result.push(FileEntry {
                id: 0,
                snapshot_id: 0,
                snapshot_type: crate::types::SnapshotType::Epoch,
                file_path: path.to_string(),
                ink_hash: prev.ink_hash.clone(),
                file_size: prev.file_size,
                modified_at: prev.modified_at.clone(),
                status: FileStatus::Deleted,
                is_binary: prev.is_binary,
            });
        }
    }

    Ok(result)
}

/// Simple walked file info
#[derive(Debug, Clone)]
pub struct WalkedFile {
    pub relative_path: String,
    pub hash: String,
    pub size: i64,
    pub modified_at: String,
    pub is_binary: bool,
}

fn build_exclude_set(patterns: &[String]) -> anyhow::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();

    // Always exclude these common directories
    for pattern in &[".git/**", ".svn/**", ".hg/**"] {
        if let Ok(glob) = GlobBuilder::new(pattern).literal_separator(true).build() {
            builder.add(glob);
        }
    }

    for pattern in patterns {
        if let Ok(glob) = GlobBuilder::new(pattern).literal_separator(true).build() {
            builder.add(glob);
        }
    }

    Ok(builder.build()?)
}

fn is_binary_content(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let check_len = std::cmp::min(data.len(), 8192);
    data[..check_len].contains(&0x00)
}

fn walk_dir_recursive(
    root: &Path,
    current_dir: &Path,
    exclude_set: &GlobSet,
    files: &mut Vec<WalkedFile>,
    prev_meta: Option<&HashMap<(String, i64, String), String>>,
    is_first_snap: bool,
) -> anyhow::Result<()> {
    if !current_dir.is_dir() {
        return Ok(());
    }

    let entries = match std::fs::read_dir(current_dir) {
        Ok(entries) => entries,
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

        if exclude_set.is_match(&relative) || exclude_set.is_match(path.to_string_lossy().as_ref()) {
            continue;
        }

        if path.is_dir() {
            walk_dir_recursive(root, &path, exclude_set, files, prev_meta, is_first_snap)?;
        } else if path.is_file() {
            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = metadata.len() as i64;
            let modified = metadata
                .modified()
                .ok()
                .map(|t| {
                    let duration = t
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default();
                    let secs = duration.as_secs();
                    chrono::DateTime::from_timestamp(secs as i64, 0)
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            // First snap: skip reading entirely — hash will be computed in snap.rs
            if is_first_snap {
                files.push(WalkedFile {
                    relative_path: relative,
                    hash: String::new(),
                    size,
                    modified_at: modified,
                    is_binary: false,
                });
                continue;
            }

            // Skip reading if size+mtime match previous entry (file is unchanged)
            if let Some(meta_map) = prev_meta {
                if let Some(prev_hash) = meta_map.get(&(relative.clone(), size, modified.clone())) {
                    files.push(WalkedFile {
                        relative_path: relative,
                        hash: prev_hash.clone(),
                        size,
                        modified_at: modified,
                        is_binary: false,
                    });
                    continue;
                }
            }

            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let is_bin = is_binary_content(&data);
            let hash = if is_bin {
                let meta_str = format!("{}:{}:{:?}", relative, metadata.len(), metadata.modified().ok());
                crate::storage::ink::hash_content(meta_str.as_bytes())
            } else {
                crate::storage::ink::hash_content(&data)
            };

            files.push(WalkedFile {
                relative_path: relative,
                hash,
                size,
                modified_at: modified,
                is_binary: is_bin,
            });
        }
    }

    Ok(())
}