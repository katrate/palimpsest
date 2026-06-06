use crate::storage;

/// Execute `palin ls`
pub fn execute() -> anyhow::Result<()> {
    let registry = storage::read_registry()?;

    if registry.palins.is_empty() {
        println!("No palins found. Use `palin init <name>` to create one.");
        return Ok(());
    }

    println!("✦ Palins:");
    println!();

    // Sort by name
    let mut palins: Vec<_> = registry.palins.iter().collect();
    palins.sort_by(|a, b| a.0.cmp(b.0));

    for (name, entry) in &palins {
        let dir_path = std::path::Path::new(&entry.path);
        let exists = dir_path.exists();
        let status = if exists { "" } else { "  ⚠ Directory missing" };
        println!("  {:<20}  {}{}", name, entry.path, status);
    }

    Ok(())
}
