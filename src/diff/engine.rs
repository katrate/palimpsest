use crate::types::{DiffChangeType, DiffHunk, FileDiff, FileStatus, FileEntry};
use similar::{DiffTag, TextDiff};

/// Compute a diff between two sets of lines
pub fn diff_lines(old_lines: &[String], new_lines: &[String]) -> Vec<DiffHunk> {
    let old_text = old_lines.join("\n");
    let new_text = new_lines.join("\n");
    let diff = TextDiff::from_lines(&old_text, &new_text);
    let ops = diff.ops();
    let mut hunks = Vec::new();

    for op in ops {
        let old_range = op.old_range();
        let new_range = op.new_range();
        let old_start = old_range.start;
        let old_end = old_range.end;
        let new_start = new_range.start;
        let new_end = new_range.end;

        let change_type = match op.tag() {
            DiffTag::Equal => DiffChangeType::Equal,
            DiffTag::Insert => DiffChangeType::Insert,
            DiffTag::Delete => DiffChangeType::Delete,
            DiffTag::Replace => DiffChangeType::Replace,
        };

        if matches!(change_type, DiffChangeType::Equal) {
            continue;
        }

        let old_slice: Vec<String> = old_lines
            .get(old_start..old_end)
            .map(|s| s.to_vec())
            .unwrap_or_default();

        let new_slice: Vec<String> = new_lines
            .get(new_start..new_end)
            .map(|s| s.to_vec())
            .unwrap_or_default();

        hunks.push(DiffHunk {
            old_start: old_start + 1,
            old_lines: old_slice,
            new_start: new_start + 1,
            new_lines: new_slice,
            change_type,
        });
    }

    hunks
}

/// Compare a file between two sets of entries (from two epochs)
pub fn diff_file(
    file_path: &str,
    old_entries: &[FileEntry],
    new_entries: &[FileEntry],
    palin_name: &str,
) -> anyhow::Result<Option<FileDiff>> {
    let old_entry = old_entries.iter().find(|e| e.file_path == file_path);
    let new_entry = new_entries.iter().find(|e| e.file_path == file_path);

    match (old_entry, new_entry) {
        (None, None) => return Ok(None),
        (Some(old), None) => {
            let old_content = get_entry_content(palin_name, old)?;
            let lines = content_to_lines(&old_content);
            return Ok(Some(FileDiff {
                file_path: file_path.to_string(),
                status: FileStatus::Deleted,
                hunks: vec![DiffHunk {
                    old_start: 1,
                    old_lines: lines.clone(),
                    new_start: 0,
                    new_lines: vec![],
                    change_type: DiffChangeType::Delete,
                }],
                additions: 0,
                deletions: lines.len(),
            }));
        }
        (None, Some(new)) => {
            let new_content = get_entry_content(palin_name, new)?;
            let lines = content_to_lines(&new_content);
            return Ok(Some(FileDiff {
                file_path: file_path.to_string(),
                status: FileStatus::Added,
                hunks: vec![DiffHunk {
                    old_start: 0,
                    old_lines: vec![],
                    new_start: 1,
                    new_lines: lines.clone(),
                    change_type: DiffChangeType::Insert,
                }],
                additions: lines.len(),
                deletions: 0,
            }));
        }
        (Some(old), Some(new)) => {
            if old.ink_hash == new.ink_hash {
                return Ok(Some(FileDiff {
                    file_path: file_path.to_string(),
                    status: FileStatus::Unchanged,
                    hunks: vec![],
                    additions: 0,
                    deletions: 0,
                }));
            }

            let old_content = get_entry_content(palin_name, old)?;
            let new_content = get_entry_content(palin_name, new)?;
            let old_lines = content_to_lines(&old_content);
            let new_lines = content_to_lines(&new_content);

            let hunks = diff_lines(&old_lines, &new_lines);

            let additions: usize = hunks.iter().map(|h| h.new_lines.len()).sum();
            let deletions: usize = hunks.iter().map(|h| h.old_lines.len()).sum();

            return Ok(Some(FileDiff {
                file_path: file_path.to_string(),
                status: FileStatus::Modified,
                hunks,
                additions,
                deletions,
            }));
        }
    }
}

/// Get content for a file entry
fn get_entry_content(palin_name: &str, entry: &FileEntry) -> anyhow::Result<Vec<u8>> {
    match &entry.ink_hash {
        Some(hash) => crate::storage::ink::read_ink(palin_name, hash),
        None => Ok(Vec::new()),
    }
}

/// Convert raw content to lines
fn content_to_lines(content: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(content);
    text.lines().map(|l| l.to_string()).collect()
}
