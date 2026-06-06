use crate::storage;
use std::path::Path;

/// Execute `palin restore <epoch_id> [name] [--to dir] [-y] [--dry-run]`
pub fn execute(
    epoch_id: &str,
    name: Option<&str>,
    to_dir: Option<&Path>,
    yes: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    // Clean up expired phantoms
    storage::cleanup_expired_phantoms(&conn)?;

    // Resolve the target (epoch or phantom)
    let (snapshot_id, snapshot_type, display_name) = if epoch_id.starts_with("phantom-")
        || epoch_id.starts_with("phantom_")
        || epoch_id.starts_with('p')
    {
        let num = epoch_id
            .strip_prefix("phantom-")
            .or_else(|| epoch_id.strip_prefix("phantom_"))
            .or_else(|| epoch_id.strip_prefix('p'))
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid phantom identifier: '{}'", epoch_id))?;
        let phantom = storage::get_phantom_by_num(&conn, num)?
            .ok_or_else(|| anyhow::anyhow!("Phantom '{}' not found", epoch_id))?;
        (phantom.id, "phantom", phantom.display_name())
    } else {
        let epoch = storage::resolve_epoch(&conn, epoch_id)?;
        (epoch.id, "epoch", epoch.display_name())
    };

    // Get file entries for this snapshot
    let entries = storage::get_file_entries(&conn, snapshot_id, snapshot_type)?;

    let target_dir = match to_dir {
        Some(d) => {
            // Safe mode — restore to different directory
            if !d.exists() {
                std::fs::create_dir_all(d)?;
            }
            d.to_path_buf()
        }
        None => {
            // Create phantom first (unless --to is used)
            if !dry_run {
                let latest_phantom_num = storage::get_latest_phantom_num(&conn)?;
                let phantom_num = latest_phantom_num + 1;
                let phantom_msg = format!("before restore to {}", display_name);

                // Walk current state
                let current_entries = {
                    let latest_epoch = storage::get_latest_epoch(&conn)?;
                    if let Some(epoch) = latest_epoch {
                        storage::get_file_entries(&conn, epoch.id, "epoch")?
                    } else {
                        Vec::new()
                    }
                };

                let phantom_id = storage::create_phantom(
                    &conn,
                    phantom_num,
                    Some(&phantom_msg),
                )?;

                storage::insert_file_entries(&conn, phantom_id, "phantom", &current_entries)?;

                println!("  ○ Created phantom-{} for safety", phantom_num);
            }

            // Confirm
            if !yes && !dry_run {
                let (restore_count, delete_count) = storage::restore_engine::count_restore_changes(&entries);
                println!(
                    "  Restore to {}? This will restore {} files and delete {} files.",
                    display_name, restore_count, delete_count
                );
                print!("  Continue? [y/N]: ");
                use std::io::Write;
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("  Restore cancelled.");
                    return Ok(());
                }
            }

            resolved.path.clone()
        }
    };

    // Perform the restore
    if dry_run {
        let actions = storage::restore_engine::preview_restore(&target_dir, &entries)?;
        println!("✦ Dry run — restore to {} ({})", display_name, target_dir.display());
        if actions.is_empty() {
            println!("  No changes needed.");
        } else {
            for action in actions {
                println!("{}", action);
            }
        }
    } else {
        let actions = storage::restore_engine::restore_to_snapshot(
            &target_dir,
            &entries,
            &resolved.name,
            false,
        )?;
        println!("✦ Restored to {} ({})", display_name, target_dir.display());
        println!("  {} file(s) affected", actions.len());
    }

    Ok(())
}
