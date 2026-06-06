use crate::storage;

/// Execute `palin ignore <pattern> [name]`
pub fn execute(pattern: &str, name: Option<&str>) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;

    let mut config = storage::read_palin_config(&resolved.name)?;
    let excludes = config.excludes.get_or_insert_with(Default::default);
    excludes.patterns.push(pattern.to_string());
    storage::write_palin_config(&config)?;

    println!("✦ Added exclusion pattern '{}' to '{}'", pattern, resolved.name);
    Ok(())
}
