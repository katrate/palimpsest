use crate::storage;
use crate::types::FileStatus;

/// Execute `palin blame <file> [name]`
pub fn execute(
    file_path: &str,
    name: Option<&str>,
) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epochs = storage::list_epochs(&conn)?;

    if epochs.is_empty() {
        println!("No snapshots yet for '{}'.", resolved.name);
        return Ok(());
    }

    // Walk backwards through epochs to find when each line changed
    // Build a map of line -> epoch number
    let mut line_map: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    let mut current_lines: Option<Vec<String>> = None;

    // Process epochs from latest to earliest
    for epoch in epochs.iter().rev() {
        let entries = storage::get_file_entries(&conn, epoch.id, "epoch")?;

        // Find the file in this epoch
        if let Some(entry) = entries.iter().find(|e| e.file_path == file_path) {
            match entry.status {
                FileStatus::Deleted => {
                    // All current lines were last changed before this epoch
                    // (they don't exist here, so we skip)
                }
                _ => {
                    // Get file content at this epoch
                    if let Some(ref hash) = entry.ink_hash {
                        if let Ok(content) = storage::ink::read_ink(&resolved.name, hash) {
                            let lines: Vec<String> = String::from_utf8_lossy(&content)
                                .lines()
                                .map(|l| l.to_string())
                                .collect();

                            let epoch_display = if epoch.is_origin {
                                "origin".to_string()
                            } else {
                                format!("epoch-{}", epoch.epoch_num)
                            };

                            if current_lines.is_none() {
                                // First (latest) epoch sets initial blame
                                current_lines = Some(lines.clone());
                                for (i, _) in lines.iter().enumerate() {
                                    line_map.entry(i).or_insert_with(|| epoch_display.clone());
                                }
                            } else if let Some(ref cur) = current_lines {
                                // Compare with current lines — if a line matches, it was last changed here
                                for (i, line) in lines.iter().enumerate() {
                                    if i < cur.len() && cur[i] == *line {
                                        line_map.entry(i).or_insert_with(|| epoch_display.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Output the blame
    if let Some(lines) = current_lines {
        for (i, line) in lines.iter().enumerate() {
            let epoch = line_map.get(&i).map(|s| s.as_str()).unwrap_or("?");
            println!("{:<12} {:>4} │ {}", epoch, i + 1, line);
        }
    } else {
        eprintln!("File '{}' not found in any epoch.", file_path);
    }

    Ok(())
}
