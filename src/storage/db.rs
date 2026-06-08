use crate::types::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

/// Open (or create) the SQLite database for a palin
pub fn open_db(name: &str) -> anyhow::Result<Connection> {
    let dir = crate::types::palimpsest_dir()?.join(name);
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("db.sqlite");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    initialize_schema(&conn)?;
    Ok(conn)
}

/// Initialize the database schema
fn initialize_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS epochs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            epoch_num   INTEGER NOT NULL,
            timestamp   TEXT NOT NULL,
            message     TEXT,
            is_origin   BOOLEAN NOT NULL DEFAULT 0,
            is_deleted  BOOLEAN NOT NULL DEFAULT 0,
            is_locked   BOOLEAN NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS phantoms (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            phantom_num INTEGER NOT NULL,
            timestamp   TEXT NOT NULL,
            expires_at  TEXT NOT NULL,
            message     TEXT,
            is_deleted  BOOLEAN NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS file_entries (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id    INTEGER NOT NULL,
            snapshot_type  TEXT NOT NULL,
            file_path      TEXT NOT NULL,
            ink_hash       TEXT,
            file_size      INTEGER,
            modified_at    TEXT,
            status         TEXT NOT NULL,
            is_binary      BOOLEAN NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS inks (
            hash        TEXT PRIMARY KEY,
            size        INTEGER NOT NULL,
            compressed  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            ref_count   INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS tags (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            epoch_id  INTEGER NOT NULL REFERENCES epochs(id),
            name      TEXT NOT NULL UNIQUE,
            created   TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS notes (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            epoch_id  INTEGER NOT NULL REFERENCES epochs(id),
            text      TEXT NOT NULL,
            created   TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Indexes for performance
        CREATE INDEX IF NOT EXISTS idx_file_entries_snapshot ON file_entries(snapshot_id, snapshot_type);
        CREATE INDEX IF NOT EXISTS idx_file_entries_path ON file_entries(file_path);
        CREATE INDEX IF NOT EXISTS idx_epochs_epoch_num ON epochs(epoch_num);
        CREATE INDEX IF NOT EXISTS idx_phantoms_phantom_num ON phantoms(phantom_num);
        CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
        CREATE INDEX IF NOT EXISTS idx_notes_epoch ON notes(epoch_id);
        ",
    )?;
    Ok(())
}

// ─── Epochs ──────────────────────────────────────────────────────────────

/// Create a new epoch
pub fn create_epoch(
    conn: &Connection,
    epoch_num: i64,
    message: Option<&str>,
    is_origin: bool,
) -> anyhow::Result<i64> {
    let timestamp = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO epochs (epoch_num, timestamp, message, is_origin) VALUES (?1, ?2, ?3, ?4)",
        params![epoch_num, timestamp, message, is_origin],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Get the latest epoch number (returns -1 if no epochs exist)
pub fn get_latest_epoch_num(conn: &Connection) -> anyhow::Result<i64> {
    let val: i64 = conn.query_row(
        "SELECT COALESCE(MAX(epoch_num), -1) FROM epochs WHERE is_deleted = 0",
        [],
        |row| row.get(0),
    )?;
    Ok(val)
}

/// Get the latest epoch (full row)
pub fn get_latest_epoch(conn: &Connection) -> anyhow::Result<Option<Epoch>> {
    let mut stmt = conn.prepare(
        "SELECT id, epoch_num, timestamp, message, is_origin, is_deleted, is_locked
         FROM epochs WHERE is_deleted = 0 ORDER BY epoch_num DESC LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;
    match rows.next()? {
        Some(row) => {
            let timestamp_str: String = row.get(2)?;
            Ok(Some(Epoch {
                id: row.get(0)?,
                epoch_num: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))?
                    .with_timezone(&Utc),
                message: row.get(3)?,
                is_origin: row.get(4)?,
                is_deleted: row.get(5)?,
                is_locked: row.get(6)?,
            }))
        }
        None => Ok(None),
    }
}

/// Get an epoch by epoch number
pub fn get_epoch_by_num(conn: &Connection, epoch_num: i64) -> anyhow::Result<Option<Epoch>> {
    let mut stmt = conn.prepare(
        "SELECT id, epoch_num, timestamp, message, is_origin, is_deleted, is_locked
         FROM epochs WHERE epoch_num = ?1 AND is_deleted = 0",
    )?;
    let mut rows = stmt.query(params![epoch_num])?;
    match rows.next()? {
        Some(row) => {
            let timestamp_str: String = row.get(2)?;
            Ok(Some(Epoch {
                id: row.get(0)?,
                epoch_num: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))?
                    .with_timezone(&Utc),
                message: row.get(3)?,
                is_origin: row.get(4)?,
                is_deleted: row.get(5)?,
                is_locked: row.get(6)?,
            }))
        }
        None => Ok(None),
    }
}

/// Get an epoch by its string identifier ("origin", "epoch-3", "3")
pub fn resolve_epoch(conn: &Connection, ident: &str) -> anyhow::Result<Epoch> {
    let epoch_num = if ident.eq_ignore_ascii_case("origin") {
        0
    } else if let Some(num) = ident.strip_prefix("epoch-").or_else(|| ident.strip_prefix("epoch_")) {
        num.parse::<i64>()?
    } else if let Ok(num) = ident.parse::<i64>() {
        num
    } else {
        anyhow::bail!("Invalid epoch identifier: '{}'. Use 'origin', 'epoch-N', or a number.", ident);
    };

    get_epoch_by_num(conn, epoch_num)?.ok_or_else(|| {
        anyhow::anyhow!("Epoch '{}' not found", ident)
    })
}

/// List all non-deleted epochs
pub fn list_epochs(conn: &Connection) -> anyhow::Result<Vec<Epoch>> {
    let mut stmt = conn.prepare(
        "SELECT id, epoch_num, timestamp, message, is_origin, is_deleted, is_locked
         FROM epochs WHERE is_deleted = 0 ORDER BY epoch_num ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let timestamp_str: String = row.get(2)?;
        Ok(Epoch {
            id: row.get(0)?,
            epoch_num: row.get(1)?,
            timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))
                .unwrap()
                .with_timezone(&Utc),
            message: row.get(3)?,
            is_origin: row.get(4)?,
            is_deleted: row.get(5)?,
            is_locked: row.get(6)?,
        })
    })?;
    let mut epochs = Vec::new();
    for row in rows {
        epochs.push(row?);
    }
    Ok(epochs)
}

/// Soft-delete an epoch
pub fn delete_epoch(conn: &Connection, epoch_id: i64) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE epochs SET is_deleted = 1 WHERE id = ?1 AND is_locked = 0",
        params![epoch_id],
    )?;
    Ok(())
}

/// Lock/unlock an epoch
pub fn set_epoch_lock(conn: &Connection, epoch_id: i64, locked: bool) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE epochs SET is_locked = ?1 WHERE id = ?2",
        params![locked, epoch_id],
    )?;
    Ok(())
}

// ─── Phantoms ────────────────────────────────────────────────────────────

/// Create a new phantom (24h TTL)
pub fn create_phantom(
    conn: &Connection,
    phantom_num: i64,
    message: Option<&str>,
) -> anyhow::Result<i64> {
    let timestamp = Utc::now().to_rfc3339();
    let expires_at = (Utc::now() + chrono::TimeDelta::hours(24)).to_rfc3339();
    conn.execute(
        "INSERT INTO phantoms (phantom_num, timestamp, expires_at, message) VALUES (?1, ?2, ?3, ?4)",
        params![phantom_num, timestamp, expires_at, message],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Get the latest phantom number (returns 0 if no phantoms exist)
pub fn get_latest_phantom_num(conn: &Connection) -> anyhow::Result<i64> {
    let val: i64 = conn.query_row(
        "SELECT COALESCE(MAX(phantom_num), 0) FROM phantoms WHERE is_deleted = 0",
        [],
        |row| row.get(0),
    )?;
    Ok(val)
}

/// Get a phantom by number
pub fn get_phantom_by_num(conn: &Connection, phantom_num: i64) -> anyhow::Result<Option<Phantom>> {
    let mut stmt = conn.prepare(
        "SELECT id, phantom_num, timestamp, expires_at, message, is_deleted
         FROM phantoms WHERE phantom_num = ?1 AND is_deleted = 0",
    )?;
    let mut rows = stmt.query(params![phantom_num])?;
    match rows.next()? {
        Some(row) => {
            let timestamp_str: String = row.get(2)?;
            let expires_str: String = row.get(3)?;
            Ok(Some(Phantom {
                id: row.get(0)?,
                phantom_num: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))?
                    .with_timezone(&Utc),
                expires_at: DateTime::parse_from_rfc3339(&expires_str)
                    .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))?
                    .with_timezone(&Utc),
                message: row.get(4)?,
                is_deleted: row.get(5)?,
            }))
        }
        None => Ok(None),
    }
}

/// List all non-deleted phantoms
pub fn list_phantoms(conn: &Connection) -> anyhow::Result<Vec<Phantom>> {
    let mut stmt = conn.prepare(
        "SELECT id, phantom_num, timestamp, expires_at, message, is_deleted
         FROM phantoms WHERE is_deleted = 0 ORDER BY phantom_num ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let timestamp_str: String = row.get(2)?;
        let expires_str: String = row.get(3)?;
        Ok(Phantom {
            id: row.get(0)?,
            phantom_num: row.get(1)?,
            timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))
                .unwrap()
                .with_timezone(&Utc),
            expires_at: DateTime::parse_from_rfc3339(&expires_str)
                .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))
                .unwrap()
                .with_timezone(&Utc),
            message: row.get(4)?,
            is_deleted: row.get(5)?,
        })
    })?;
    let mut phantoms = Vec::new();
    for row in rows {
        phantoms.push(row?);
    }
    Ok(phantoms)
}

/// Soft-delete a phantom
pub fn delete_phantom(conn: &Connection, phantom_id: i64) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE phantoms SET is_deleted = 1 WHERE id = ?1",
        params![phantom_id],
    )?;
    Ok(())
}

/// Clean up expired phantoms
pub fn cleanup_expired_phantoms(conn: &Connection) -> anyhow::Result<usize> {
    let now = Utc::now().to_rfc3339();
    let count = conn.execute(
        "UPDATE phantoms SET is_deleted = 1 WHERE expires_at < ?1 AND is_deleted = 0",
        params![now],
    )?;
    Ok(count)
}

// ─── File Entries ────────────────────────────────────────────────────────

/// Insert a batch of file entries for a snapshot
pub fn insert_file_entries(
    conn: &Connection,
    snapshot_id: i64,
    snapshot_type: &str,
    entries: &[FileEntry],
) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO file_entries (snapshot_id, snapshot_type, file_path, ink_hash, file_size, modified_at, status, is_binary)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for entry in entries {
        stmt.execute(params![
            snapshot_id,
            snapshot_type,
            entry.file_path,
            entry.ink_hash,
            entry.file_size,
            entry.modified_at,
            entry.status.as_str(),
            entry.is_binary,
        ])?;
    }
    Ok(())
}

/// Get file entries for a snapshot (epoch or phantom)
pub fn get_file_entries(
    conn: &Connection,
    snapshot_id: i64,
    snapshot_type: &str,
) -> anyhow::Result<Vec<FileEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, snapshot_id, snapshot_type, file_path, ink_hash, file_size, modified_at, status, is_binary
         FROM file_entries
         WHERE snapshot_id = ?1 AND snapshot_type = ?2
         ORDER BY file_path ASC",
    )?;
    let rows = stmt.query_map(params![snapshot_id, snapshot_type], |row| {
        Ok(FileEntry {
            id: row.get(0)?,
            snapshot_id: row.get(1)?,
            snapshot_type: if row.get::<_, String>(2)? == "epoch" {
                SnapshotType::Epoch
            } else {
                SnapshotType::Phantom
            },
            file_path: row.get(3)?,
            ink_hash: row.get(4)?,
            file_size: row.get(5)?,
            modified_at: row.get(6)?,
            status: FileStatus::from_str(&row.get::<_, String>(7)?).unwrap_or(FileStatus::Unchanged),
            is_binary: row.get(8)?,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

// ─── Inks ────────────────────────────────────────────────────────────────

/// Record an ink reference
pub fn upsert_ink(
    conn: &Connection,
    hash: &str,
    size: i64,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO inks (hash, size, compressed, created_at, ref_count)
         VALUES (?1, ?2, 0, ?3, 1)
         ON CONFLICT(hash) DO UPDATE SET ref_count = ref_count + 1",
        params![hash, size, now],
    )?;
    Ok(())
}

/// Decrement ink ref count (when an epoch is deleted)
pub fn decrement_ink_ref(conn: &Connection, hash: &str) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE inks SET ref_count = MAX(0, ref_count - 1) WHERE hash = ?1",
        params![hash],
    )?;
    Ok(())
}

/// Get unreferenced inks (ref_count = 0)
pub fn get_unreferenced_inks(conn: &Connection) -> anyhow::Result<Vec<InkInfo>> {
    let mut stmt = conn.prepare(
        "SELECT hash, size, compressed, created_at, ref_count
         FROM inks WHERE ref_count <= 0",
    )?;
    let rows = stmt.query_map([], |row| {
        let created_str: String = row.get(3)?;
        Ok(InkInfo {
            hash: row.get(0)?,
            size: row.get(1)?,
            compressed: row.get(2)?,
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))
                .unwrap()
                .with_timezone(&Utc),
            ref_count: row.get(4)?,
        })
    })?;
    let mut inks = Vec::new();
    for row in rows {
        inks.push(row?);
    }
    Ok(inks)
}

/// Delete ink from database
pub fn delete_ink_from_db(conn: &Connection, hash: &str) -> anyhow::Result<()> {
    conn.execute("DELETE FROM inks WHERE hash = ?1", params![hash])?;
    Ok(())
}

/// Get total ink stats
pub fn get_ink_stats(conn: &Connection) -> anyhow::Result<(i64, i64)> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM inks", [], |row| row.get(0))?;
    let total_size: i64 = conn
        .query_row("SELECT COALESCE(SUM(size), 0) FROM inks", [], |row| row.get(0))?;
    Ok((count, total_size))
}

// ─── Tags ────────────────────────────────────────────────────────────────

pub fn create_tag(conn: &Connection, epoch_id: i64, name: &str) -> anyhow::Result<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO tags (epoch_id, name, created) VALUES (?1, ?2, ?3)",
        params![epoch_id, name, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_tag(conn: &Connection, name: &str) -> anyhow::Result<()> {
    conn.execute("DELETE FROM tags WHERE name = ?1", params![name])?;
    Ok(())
}

pub fn list_tags(conn: &Connection) -> anyhow::Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT id, epoch_id, name, created FROM tags ORDER BY name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let created_str: String = row.get(3)?;
        Ok(Tag {
            id: row.get(0)?,
            epoch_id: row.get(1)?,
            name: row.get(2)?,
            created: DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))
                .unwrap()
                .with_timezone(&Utc),
        })
    })?;
    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }
    Ok(tags)
}

// ─── Notes ───────────────────────────────────────────────────────────────

pub fn create_note(conn: &Connection, epoch_id: i64, text: &str) -> anyhow::Result<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO notes (epoch_id, text, created) VALUES (?1, ?2, ?3)",
        params![epoch_id, text, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_notes(conn: &Connection, epoch_id: i64) -> anyhow::Result<Vec<Note>> {
    let mut stmt = conn.prepare(
        "SELECT id, epoch_id, text, created FROM notes WHERE epoch_id = ?1 ORDER BY created ASC",
    )?;
    let rows = stmt.query_map(params![epoch_id], |row| {
        let created_str: String = row.get(3)?;
        Ok(Note {
            id: row.get(0)?,
            epoch_id: row.get(1)?,
            text: row.get(2)?,
            created: DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))
                .unwrap()
                .with_timezone(&Utc),
        })
    })?;
    let mut notes = Vec::new();
    for row in rows {
        notes.push(row?);
    }
    Ok(notes)
}
