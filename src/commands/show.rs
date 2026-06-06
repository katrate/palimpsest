use crate::storage;
use crate::types::FileStatus;

/// Execute `palin show <epoch> <file> [name]`
pub fn execute(
    epoch_id: &str,
    file_path: &str,
    name: Option<&str>,
) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epoch = storage::resolve_epoch(&conn, epoch_id)?;
    let entries = storage::get_file_entries(&conn, epoch.id, "epoch")?;

    let entry = entries.iter().find(|e| e.file_path == file_path);

    match entry {
        Some(e) if e.status != FileStatus::Deleted => {
            match &e.ink_hash {
                Some(hash) => {
                    let content = storage::ink::read_ink(&resolved.name, hash)?;
                    // Write to stdout
                    use std::io::Write;
                    std::io::stdout().write_all(&content)?;
                }
                None => {
                    println!("(empty file)");
                }
            }
        }
        Some(_) => {
            println!("File '{}' was deleted at {}", file_path, epoch.display_name());
        }
        None => {
            eprintln!("File '{}' not found at {}", file_path, epoch.display_name());
        }
    }

    Ok(())
}
