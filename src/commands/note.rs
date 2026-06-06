use crate::storage;

/// Execute `palin note <name> <epoch> <text>` or `palin note <name> <epoch>`
pub fn execute(name: Option<&str>, epoch_id: &str, text: Option<&str>) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epoch = storage::resolve_epoch(&conn, epoch_id)?;

    match text {
        Some(t) => {
            storage::create_note(&conn, epoch.id, t)?;
            println!("✦ Note added to {}", epoch.display_name());
        }
        None => {
            let notes = storage::list_notes(&conn, epoch.id)?;
            if notes.is_empty() {
                println!("No notes for {}.", epoch.display_name());
            } else {
                println!("✦ Notes for {}:", epoch.display_name());
                for note in &notes {
                    let time = note.created.format("%Y-%m-%d %H:%M").to_string();
                    println!("  [{}] {}", time, note.text);
                }
            }
        }
    }

    Ok(())
}
