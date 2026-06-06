use crate::storage;

/// Execute `palin log [name] [--oneline] [--phantoms]`
pub fn execute(
    name: Option<&str>,
    oneline: bool,
    show_phantoms: bool,
) -> anyhow::Result<()> {
    let resolved = super::snap::resolve_palin(name)?;
    let conn = storage::open_db(&resolved.name)?;

    // Clean up expired phantoms
    storage::cleanup_expired_phantoms(&conn)?;

    let epochs = storage::list_epochs(&conn)?;
    let phantoms = if show_phantoms {
        storage::list_phantoms(&conn)?
    } else {
        Vec::new()
    };

    if epochs.is_empty() && phantoms.is_empty() {
        println!("No snapshots yet for '{}'. Use `palin snap`", resolved.name);
        return Ok(());
    }

    println!("✦ {}", resolved.name);
    println!();

    // Show timeline in reverse chronological order
    let all_items: Vec<SnapshotItem> = epochs
        .iter()
        .map(|e| SnapshotItem {
            display: if e.is_origin {
                "Origin".to_string()
            } else {
                format!("Epoch #{}", e.epoch_num)
            },
            timestamp: e.timestamp,
            message: e.message.clone(),
            is_locked: e.is_locked,
            kind: "epoch",
        })
        .chain(phantoms.iter().map(|p| SnapshotItem {
            display: format!("Phantom #{}", p.phantom_num),
            timestamp: p.timestamp,
            message: p.message.clone(),
            is_locked: false,
            kind: "phantom",
        }))
        .collect();

    // Sort by timestamp descending
    let mut sorted = all_items;
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    if oneline {
        for item in &sorted {
            let lock_flag = if item.is_locked { " 🔒" } else { "" };
            let kind_marker = match item.kind {
                "epoch" => "●",
                "phantom" => "○",
                _ => "•",
            };
            let time_str = item.timestamp.format("%Y-%m-%d %H:%M").to_string();
            let msg = item
                .message
                .as_deref()
                .map(|m| format!(" \"{}\"", m))
                .unwrap_or_default();
            println!("{kind_marker} {:<20} {}{}{}", item.display, time_str, msg, lock_flag);
        }
    } else {
        for item in &sorted {
            let lock_flag = if item.is_locked { "  🔒 Locked" } else { "" };
            let kind_marker = match item.kind {
                "epoch" => "●",
                "phantom" => "○",
                _ => "•",
            };
            let time_str = item.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            println!("{kind_marker} {:<20}  {}{}", item.display, time_str, lock_flag);
            if let Some(msg) = &item.message {
                println!("  └─ {}", msg);
            }
        }
    }

    Ok(())
}

struct SnapshotItem {
    display: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    message: Option<String>,
    is_locked: bool,
    kind: &'static str,
}
