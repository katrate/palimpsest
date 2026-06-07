use crate::storage;
use crate::types::{FileStatus, ResolvedPalin};

/// Resolve the palin from either a provided name or the current directory
pub fn resolve_palin(name: Option<&str>) -> anyhow::Result<ResolvedPalin> {
    match name {
        Some(n) => {
            let config = storage::read_palin_config(n)?;
            Ok(ResolvedPalin {
                name: n.to_string(),
                path: std::path::PathBuf::from(&config.path),
                config,
            })
        }
        None => {
            let cwd = std::env::current_dir()?;
            let (resolved_name, resolved_path) = storage::resolve_palin_from_dir(&cwd)?;
            let config = storage::read_palin_config(&resolved_name)?;
            Ok(ResolvedPalin {
                name: resolved_name,
                path: resolved_path,
                config,
            })
        }
    }
}

/// Execute `palin snap [name] [-m message]`
pub fn execute(name: Option<&str>, message: Option<&str>) -> anyhow::Result<()> {
    let resolved = resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    // Clean up expired phantoms
    storage::cleanup_expired_phantoms(&conn)?;

    let latest_num = storage::get_latest_epoch_num(&conn)?;
    let is_first = latest_num < 0;
    let new_epoch_num = if is_first { 0 } else { latest_num + 1 };

    // Get previous epoch's file entries for diff
    let previous_entries = if !is_first {
        let prev_epoch = storage::get_epoch_by_num(&conn, latest_num)?;
        if let Some(epoch) = prev_epoch {
            storage::get_file_entries(&conn, epoch.id, "epoch")?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Get exclude patterns
    let exclude_patterns = resolved
        .config
        .excludes
        .as_ref()
        .map(|e| e.patterns.clone())
        .unwrap_or_default();

    // Walk the directory and compute changes
    let file_entries = storage::walk_and_diff(
        &resolved.path,
        &exclude_patterns,
        &previous_entries,
    )?;

    // Create the epoch in the database
    let epoch_id = storage::create_epoch(
        &conn,
        new_epoch_num,
        message,
        is_first,
    )?;

    // Store inks and record references first, updating hashes to match
    // what was actually stored (important: the walker may compute a different
    // hash for binary files than the content hash used by store_ink)
    let mut entries = file_entries;
    for entry in &mut entries {
        if let Some(ref mut hash) = entry.ink_hash {
            let full_path = resolved.path.join(&entry.file_path);
            if full_path.exists() && entry.status != FileStatus::Deleted {
                let content = std::fs::read(&full_path)?;
                let stored_hash = crate::storage::ink::store_ink(&resolved.name, &content)?;
                *hash = stored_hash.clone();
                storage::upsert_ink(&conn, &stored_hash, content.len() as i64)?;
            } else if entry.status == FileStatus::Deleted {
                // Still reference the ink if it was previously stored
                if let Some(prev_entry) = previous_entries.iter().find(|e| e.file_path == entry.file_path) {
                    if let Some(prev_hash) = &prev_entry.ink_hash {
                        *hash = prev_hash.clone();
                        storage::upsert_ink(&conn, prev_hash, entry.file_size.unwrap_or(0))?;
                    }
                }
            }
        }
    }

    // Now insert file entries with correct hashes
    storage::insert_file_entries(&conn, epoch_id, "epoch", &entries)?;

    // Count changes
    let added = entries.iter().filter(|e| e.status == FileStatus::Added).count();
    let modified = entries.iter().filter(|e| e.status == FileStatus::Modified).count();
    let deleted = entries.iter().filter(|e| e.status == FileStatus::Deleted).count();
    let unchanged = entries.iter().filter(|e| e.status == FileStatus::Unchanged).count();

    let epoch_display = if is_first {
        "origin".to_string()
    } else {
        format!("epoch-{new_epoch_num}")
    };

    println!("✦ Snapshot saved as {epoch_display}");
    if let Some(msg) = message {
        println!("  \"{msg}\"");
    }
    println!(
        "  Files: +{added} ~{modified} -{deleted} ={unchanged} (total {})",
        entries.len()
    );

    Ok(())
}
