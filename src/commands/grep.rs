use crate::storage;
use crate::types::FileStatus;
use regex::Regex;

/// Execute `palin grep <pattern> [name]`
pub fn execute(
    pattern: &str,
    name: Option<&str>,
) -> anyhow::Result<()> {
    let re = Regex::new(pattern)
        .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;

    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epochs = storage::list_epochs(&conn)?;

    if epochs.is_empty() {
        println!("No snapshots yet for '{}'.", resolved.name);
        return Ok(());
    }

    let mut total_matches = 0;

    for epoch in &epochs {
        let entries = storage::get_file_entries(&conn, epoch.id, "epoch")?;

        for entry in &entries {
            if entry.status == FileStatus::Deleted || entry.ink_hash.is_none() {
                continue;
            }

            // Check if file extension is text-like (skip binary)
            if entry.is_binary {
                continue;
            }

            if let Some(ref hash) = entry.ink_hash {
                if let Ok(content) = storage::ink::read_ink(&resolved.name, hash) {
                    let text = String::from_utf8_lossy(&content);
                    for (line_num, line) in text.lines().enumerate() {
                        if re.is_match(line) {
                            let epoch_display = if epoch.is_origin {
                                "origin".to_string()
                            } else {
                                format!("epoch-{}", epoch.epoch_num)
                            };
                            println!(
                                "{}:{}:{}: {}",
                                epoch_display,
                                entry.file_path,
                                line_num + 1,
                                line
                            );
                            total_matches += 1;
                        }
                    }
                }
            }
        }
    }

    if total_matches == 0 {
        println!("No matches found for pattern '{}'.", pattern);
    } else {
        println!("\n{} match(es) found across {} epoch(s)", total_matches, epochs.len());
    }

    Ok(())
}
