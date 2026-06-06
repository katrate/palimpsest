use crate::storage;
use crate::types::FileStatus;

/// Execute `palin find [name] <filename>`
pub fn execute(filename: &str, name: Option<&str>) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epochs = storage::list_epochs(&conn)?;

    if epochs.is_empty() {
        println!("No snapshots yet for '{}'.", resolved.name);
        return Ok(());
    }

    let pattern_lower = filename.to_lowercase();

    let mut found = false;
    for epoch in &epochs {
        let entries = storage::get_file_entries(&conn, epoch.id, "epoch")?;

        for entry in &entries {
            if entry.status == FileStatus::Deleted {
                continue;
            }
            if entry.file_path.to_lowercase().contains(&pattern_lower) {
                found = true;
                let epoch_display = if epoch.is_origin {
                    "origin".to_string()
                } else {
                    format!("epoch-{}", epoch.epoch_num)
                };
                println!("  {}  {}  ({} bytes)", epoch_display, entry.file_path, entry.file_size.unwrap_or(0));
            }
        }
    }

    if !found {
        println!("No files matching '{}' found.", filename);
    }

    Ok(())
}
