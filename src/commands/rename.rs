use crate::storage;

/// Execute `palin rename <old-name> <new-name>`
pub fn execute(old_name: &str, new_name: &str) -> anyhow::Result<()> {
    // 1. Rename the storage directory on disk first
    let old_dir = crate::types::palimpsest_dir()?.join(old_name);
    let new_dir = crate::types::palimpsest_dir()?.join(new_name);

    if new_dir.exists() {
        anyhow::bail!("Palin '{}' already exists", new_name);
    }

    if old_dir.exists() {
        std::fs::rename(&old_dir, &new_dir)?;
    }

    // 2. Update config.toml name field (inside the now-renamed directory)
    if new_dir.join("config.toml").exists() {
        let mut config = storage::read_palin_config(new_name)?;
        config.name = new_name.to_string();
        storage::write_palin_config(&config)?;
    }

    // 3. Update the registry LAST so a failure in steps 1-2 doesn't break state
    storage::rename_palin_in_registry(old_name, new_name)?;

    println!("✦ Renamed '{old_name}' → '{new_name}'");
    Ok(())
}
