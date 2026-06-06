use crate::types::*;
use std::path::Path;

/// Read the global registry
pub fn read_registry() -> anyhow::Result<Registry> {
    let path = registry_path()?;
    if !path.exists() {
        return Ok(Registry::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read registry at {}: {}", path.display(), e))?;
    if content.trim().is_empty() {
        return Ok(Registry::default());
    }
    toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse registry: {}", e))
}

/// Write the global registry
pub fn write_registry(registry: &Registry) -> anyhow::Result<()> {
    let path = registry_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(registry)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Register a new palin
pub fn register_palin(name: &str, dir: &Path, created: &str) -> anyhow::Result<()> {
    let mut registry = read_registry()?;
    if registry.palins.contains_key(name) {
        anyhow::bail!("Palin '{}' already exists", name);
    }
    registry.palins.insert(
        name.to_string(),
        RegistryEntry {
            path: dir.canonicalize()?.to_string_lossy().to_string(),
            created: created.to_string(),
        },
    );
    write_registry(&registry)?;
    Ok(())
}

/// Unregister a palin
pub fn unregister_palin(name: &str) -> anyhow::Result<()> {
    let mut registry = read_registry()?;
    if registry.palins.remove(name).is_none() {
        anyhow::bail!("Palin '{}' not found", name);
    }
    write_registry(&registry)?;
    Ok(())
}

/// Rename a palin in the registry
pub fn rename_palin_in_registry(old_name: &str, new_name: &str) -> anyhow::Result<()> {
    let mut registry = read_registry()?;
    let entry = registry
        .palins
        .remove(old_name)
        .ok_or_else(|| anyhow::anyhow!("Palin '{}' not found", old_name))?;
    if registry.palins.contains_key(new_name) {
        anyhow::bail!("Palin '{}' already exists", new_name);
    }
    registry.palins.insert(new_name.to_string(), entry);
    write_registry(&registry)?;
    Ok(())
}

/// Resolve a palin from the current directory
/// Returns the palin name and its tracked path
pub fn resolve_palin_from_dir(current_dir: &Path) -> anyhow::Result<(String, std::path::PathBuf)> {
    let registry = read_registry()?;
    let canonical_current = current_dir.canonicalize()?;
    let current_str = canonical_current.to_string_lossy().to_lowercase();

    let mut matches: Vec<(String, std::path::PathBuf)> = Vec::new();

    for (name, entry) in &registry.palins {
        let palin_path = std::path::PathBuf::from(&entry.path);
        let palin_canonical = if palin_path.exists() {
            palin_path.canonicalize()?
        } else {
            palin_path
        };
        let palin_str = palin_canonical.to_string_lossy().to_lowercase();

        if current_str == palin_str || current_str.starts_with(&format!("{}\\", palin_str)) {
            matches.push((name.clone(), palin_canonical));
        }
    }

    match matches.len() {
        0 => anyhow::bail!(
            "No palin found for this directory. Did you mean to `palin init <name>` here?"
        ),
        1 => Ok(matches.remove(0)),
        _ => anyhow::bail!(
            "Multiple palins match this directory. Specify one by name."
        ),
    }
}

/// Get the palin directory path for a given palin name
pub fn palin_dir(name: &str) -> anyhow::Result<std::path::PathBuf> {
    Ok(palimpsest_dir()?.join(name))
}

/// Read palin config
pub fn read_palin_config(name: &str) -> anyhow::Result<PalinConfig> {
    let path = palin_dir(name)?.join("config.toml");
    if !path.exists() {
        anyhow::bail!("Palin '{}' has no config file", name);
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&content)?)
}

/// Write palin config
pub fn write_palin_config(config: &PalinConfig) -> anyhow::Result<()> {
    let dir = palin_dir(&config.name)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("config.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}


