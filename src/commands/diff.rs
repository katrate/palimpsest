use crate::diff;
use crate::storage;
use crate::types::FileStatus;

/// Execute `palin diff <epoch1> <epoch2> [file] [name]`
pub fn execute(
    epoch1_id: &str,
    epoch2_id: &str,
    file_filter: Option<&str>,
    name: Option<&str>,
) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epoch1 = storage::resolve_epoch(&conn, epoch1_id)?;
    let epoch2 = storage::resolve_epoch(&conn, epoch2_id)?;

    let entries1 = storage::get_file_entries(&conn, epoch1.id, "epoch")?;
    let entries2 = storage::get_file_entries(&conn, epoch2.id, "epoch")?;

    // Get all unique file paths
    let mut all_paths: Vec<String> = entries1
        .iter()
        .chain(entries2.iter())
        .map(|e| e.file_path.clone())
        .collect();
    all_paths.sort();
    all_paths.dedup();

    // Filter if a specific file is requested
    let paths_to_diff: Vec<&str> = match file_filter {
        Some(f) => vec![f],
        None => all_paths.iter().map(|s| s.as_str()).collect(),
    };

    let mut total_additions = 0;
    let mut total_deletions = 0;
    let mut changed_files = 0;

    for path in &paths_to_diff {
        let result = diff::diff_file(path, &entries1, &entries2, &resolved.name)?;

        if let Some(file_diff) = result {
            if matches!(file_diff.status, FileStatus::Unchanged) {
                continue;
            }

            changed_files += 1;
            total_additions += file_diff.additions;
            total_deletions += file_diff.deletions;

            let status_char = match file_diff.status {
                FileStatus::Added => '+',
                FileStatus::Modified => '~',
                FileStatus::Deleted => '-',
                _ => ' ',
            };

            println!(
                "{status_char} {}  (+{} / -{})",
                file_diff.file_path, file_diff.additions, file_diff.deletions
            );

            for hunk in &file_diff.hunks {
                println!(
                    "  @@ -{},{} +{},{} @@",
                    hunk.old_start,
                    hunk.old_lines.len(),
                    hunk.new_start,
                    hunk.new_lines.len()
                );

                for line in &hunk.old_lines {
                    println!("  -{}", line);
                }
                for line in &hunk.new_lines {
                    println!("  +{}", line);
                }
                println!();
            }
        }
    }

    if changed_files == 0 {
        println!("No differences between {} and {}.", epoch1.display_name(), epoch2.display_name());
    } else {
        println!(
            "{} file(s) changed: +{} additions, -{} deletions",
            changed_files, total_additions, total_deletions
        );
    }

    Ok(())
}
