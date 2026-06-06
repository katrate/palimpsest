use crate::storage;

/// Execute `palin rename <old-name> <new-name>`
pub fn execute(old_name: &str, new_name: &str) -> anyhow::Result<()> {
    // Rename in registry
    storage::rename_palin_in_registry(old_name, new_name)?;

    // Rename palin directory
    let old_dir = crate::types::palimpsest_dir()?.join(old_name);
    let new_dir = crate::types::palimpsest_dir()?.join(new_name);

    if old_dir.exists() {
        std::fs::rename(&old_dir, &new_dir)?;
    }

    // Update config.toml name field
    if new_dir.join("config.toml").exists() {
        let mut config = storage::read_palin_config(new_name)?;
        config.name = new_name.to_string();
        storage::write_palin_config(&config)?;
    }

    println!("✦ Renamed '{old_name}' → '{new_name}'");
    Ok(())
}
