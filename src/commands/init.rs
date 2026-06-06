use crate::storage;
use crate::types::PalinConfig;
use chrono::Utc;
use std::path::Path;

/// Execute `palin init <name> [dir]`
pub fn execute(name: &str, dir: Option<&Path>) -> anyhow::Result<()> {
    let target_dir = match dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
    };

    if !target_dir.is_dir() {
        anyhow::bail!("Directory does not exist: {}", target_dir.display());
    }

    let canonical_dir = target_dir.canonicalize()?;
    let created = Utc::now().to_rfc3339();

    // Register in global registry
    storage::register_palin(name, &canonical_dir, &created)?;

    // Create palin directory structure
    let palin_dir = crate::types::palimpsest_dir()?.join(name);
    std::fs::create_dir_all(palin_dir.join("inks"))?;
    std::fs::create_dir_all(palin_dir.join("epochs"))?;

    // Create config.toml
    let config = PalinConfig {
        name: name.to_string(),
        path: canonical_dir.to_string_lossy().to_string(),
        created: created.clone(),
        snapshots: None,
        excludes: None,
        binaries: None,
    };
    storage::write_palin_config(&config)?;

    // Initialize SQLite database
    let _conn = storage::open_db(name)?;

    println!(
        "✦ Initialized palin '{name}' tracking {}",
        canonical_dir.display()
    );
    println!("  Use `palin snap` to take your first snapshot");

    Ok(())
}
