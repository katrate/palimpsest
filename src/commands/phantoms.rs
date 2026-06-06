use crate::storage;

/// Execute `palin phantoms [name]`
pub fn execute(name: Option<&str>) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    // Clean up expired phantoms
    let cleaned = storage::cleanup_expired_phantoms(&conn)?;
    if cleaned > 0 {
        println!("  Cleaned {} expired phantom(s)", cleaned);
    }

    let phantoms = storage::list_phantoms(&conn)?;

    if phantoms.is_empty() {
        println!("No active phantoms for '{}'.", resolved.name);
        return Ok(());
    }

    println!("✦ Active phantoms for {}", resolved.name);
    println!();

    for phantom in &phantoms {
        let remaining = phantom.remaining_ttl();
        let hours = remaining.num_hours();
        let minutes = remaining.num_minutes() % 60;
        let time_str = phantom.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let msg = phantom
            .message
            .as_deref()
            .unwrap_or("(no message)");

        println!(
            "  ○ phantom-{}  {}  expires in {}h {}m",
            phantom.phantom_num, time_str, hours, minutes
        );
        println!("    └─ {}", msg);
    }

    Ok(())
}
