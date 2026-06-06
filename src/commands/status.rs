use crate::storage;
use crate::types::FileStatus;

/// Execute `palin status [name]`
pub fn execute(name: Option<&str>) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let latest_epoch = storage::get_latest_epoch(&conn)?;

    if latest_epoch.is_none() {
        println!("No snapshots yet for '{}'. Use `palin snap`", resolved.name);
        return Ok(());
    }

    let epoch = latest_epoch.unwrap();
    let _latest_num = epoch.epoch_num;

    // Get file entries from the latest epoch
    let entries = storage::get_file_entries(&conn, epoch.id, "epoch")?;

    let added: Vec<_> = entries.iter().filter(|e| e.status == FileStatus::Added).collect();
    let modified: Vec<_> = entries.iter().filter(|e| e.status == FileStatus::Modified).collect();
    let deleted: Vec<_> = entries.iter().filter(|e| e.status == FileStatus::Deleted).collect();
    let unchanged: Vec<_> = entries.iter().filter(|e| e.status == FileStatus::Unchanged).collect();

    let epoch_display = if epoch.is_origin { "origin" } else { "latest epoch" };

    println!("✦ {} — status relative to {}", resolved.name, epoch_display);
    println!();

    if added.is_empty() && modified.is_empty() && deleted.is_empty() {
        println!("  No changes since last snapshot.");
        println!("  {} file(s) unchanged.", unchanged.len());
        return Ok(());
    }

    if !added.is_empty() {
        println!("  Added files (+{}):", added.len());
        for entry in &added {
            println!("    + {}", entry.file_path);
        }
        println!();
    }

    if !modified.is_empty() {
        println!("  Modified files (~{}):", modified.len());
        for entry in &modified {
            println!("    ~ {}", entry.file_path);
        }
        println!();
    }

    if !deleted.is_empty() {
        println!("  Deleted files (-{}):", deleted.len());
        for entry in &deleted {
            println!("    - {}", entry.file_path);
        }
        println!();
    }

    if !unchanged.is_empty() {
        println!("  {} unchanged file(s)", unchanged.len());
    }

    let total_changed = added.len() + modified.len() + deleted.len();
    println!();
    println!("  {} file(s) changed", total_changed);

    Ok(())
}
