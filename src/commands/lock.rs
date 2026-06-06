use crate::storage;

/// Execute `palin lock <name> <epoch>`
pub fn execute_lock(name: Option<&str>, epoch_id: &str) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epoch = storage::resolve_epoch(&conn, epoch_id)?;
    storage::set_epoch_lock(&conn, epoch.id, true)?;

    println!("✦ Locked {}", epoch.display_name());
    Ok(())
}

/// Execute `palin unlock <name> <epoch>`
pub fn execute_unlock(name: Option<&str>, epoch_id: &str) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    let epoch = storage::resolve_epoch(&conn, epoch_id)?;
    storage::set_epoch_lock(&conn, epoch.id, false)?;

    println!("✦ Unlocked {}", epoch.display_name());
    Ok(())
}
