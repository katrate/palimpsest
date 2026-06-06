use crate::storage;
use crate::types::PalinInfo;

/// Execute `palin info [name]`
pub fn execute(name: Option<&str>) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    // Get epoch count
    let epochs = storage::list_epochs(&conn)?;
    let total_epochs = epochs.len();

    // Get phantom count
    let phantoms = storage::list_phantoms(&conn)?;
    let total_phantoms = phantoms.len();

    // Get ink stats
    let (total_inks, _total_ink_bytes) = storage::get_ink_stats(&conn)?;

    // Get total files in latest epoch
    let total_files = if let Some(latest) = epochs.last() {
        storage::get_file_entries(&conn, latest.id, "epoch")?.len()
    } else {
        0
    };

    // Calculate dedup ratio
    let dedup_ratio = if total_inks > 0 {
        // Count all file_entries that reference inks
        let total_refs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_entries WHERE ink_hash IS NOT NULL AND snapshot_type = 'epoch'",
                [],
                |row| row.get(0),
            )?;
        if total_refs > 0 && total_inks > 0 {
            total_refs as f64 / total_inks as f64
        } else {
            1.0
        }
    } else {
        1.0
    };

    // Get actual disk usage of palin directory
    let palin_dir = crate::types::palimpsest_dir()?.join(&resolved.name);
    let disk_usage = dir_size(&palin_dir);

    let info = PalinInfo {
        name: resolved.name.clone(),
        path: resolved.path.to_string_lossy().to_string(),
        total_epochs,
        total_phantoms,
        total_inks: total_inks as usize,
        total_files,
        disk_usage_bytes: disk_usage,
        dedup_ratio,
    };

    println!("✦ {}", info.name);
    println!("  Path:        {}", info.path);
    println!("  Epochs:      {}", info.total_epochs);
    println!("  Phantoms:    {}", info.total_phantoms);
    println!("  Files (latest): {}", info.total_files);
    println!("  Inks stored: {}", info.total_inks);
    println!("  Disk usage:  {} bytes ({:.2} KB)", info.disk_usage_bytes, info.disk_usage_bytes as f64 / 1024.0);
    println!("  Dedup ratio: {:.2}x", info.dedup_ratio);

    Ok(())
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}
