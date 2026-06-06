use crate::storage;

/// Execute tag operations
pub fn execute_add(name: Option<&str>, epoch_id: &str, tag: &str) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epoch = storage::resolve_epoch(&conn, epoch_id)?;
    storage::create_tag(&conn, epoch.id, tag)?;

    println!("✦ Tagged {} as '{}'", epoch.display_name(), tag);
    Ok(())
}

pub fn execute_delete(name: Option<&str>, tag: &str) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    storage::delete_tag(&conn, tag)?;
    println!("✦ Deleted tag '{}'", tag);
    Ok(())
}

pub fn execute_list(name: Option<&str>) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let tags = storage::list_tags(&conn)?;

    if tags.is_empty() {
        println!("No tags for '{}'.", resolved.name);
        return Ok(());
    }

    println!("✦ Tags for {}", resolved.name);
    for tag in &tags {
        let epoch = storage::get_epoch_by_num(&conn, tag.epoch_id)?;
        let epoch_name = epoch.map(|e| e.display_name()).unwrap_or_default();
        println!("  {} → {}", tag.name, epoch_name);
    }

    Ok(())
}


