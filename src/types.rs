use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The base directory for all Palimpsest data.
/// `%USERPROFILE%\\palimpsest\\palin\\` on Windows
pub fn palimpsest_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .home_dir()
        .join("palimpsest")
        .join("palin");
    Ok(base)
}

/// The global registry file path
pub fn registry_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(palimpsest_dir()?.join("registry.toml"))
}

// ─── Registry ────────────────────────────────────────────────────────────

/// The global registry mapping palin names to tracked directories
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    #[serde(flatten)]
    pub palins: HashMap<String, RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub path: String,
    pub created: String,
}

// ─── Palin Config ────────────────────────────────────────────────────────

/// Per-palin configuration (config.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PalinConfig {
    pub name: String,
    pub path: String,
    pub created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshots: Option<SnapshotConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excludes: Option<ExcludeConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binaries: Option<BinaryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapshotConfig {
    #[serde(default)]
    pub auto_interval_minutes: Option<u32>,
    #[serde(default)]
    pub compress_after_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcludeConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BinaryConfig {
    #[serde(default)]
    pub skip_content: bool,
}

// ─── Snapshot Types ──────────────────────────────────────────────────────

/// The type of a snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotType {
    Epoch,
    Phantom,
}

impl SnapshotType {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Epoch => "epoch",
            Self::Phantom => "phantom",
        }
    }
}

/// Status of a file within a snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Unchanged,
}

impl FileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Unchanged => "unchanged",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "added" => Some(Self::Added),
            "modified" => Some(Self::Modified),
            "deleted" => Some(Self::Deleted),
            "unchanged" => Some(Self::Unchanged),
            _ => None,
        }
    }
}

// ─── Epoch ───────────────────────────────────────────────────────────────

/// An epoch (snapshot) entry
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Epoch {
    pub id: i64,
    pub epoch_num: i64,
    pub timestamp: DateTime<Utc>,
    pub message: Option<String>,
    pub is_origin: bool,
    pub is_deleted: bool,
    pub is_locked: bool,
}

impl Epoch {
    pub fn display_name(&self) -> String {
        if self.is_origin {
            "origin".to_string()
        } else {
            format!("epoch-{}", self.epoch_num)
        }
    }
}

// ─── Phantom ─────────────────────────────────────────────────────────────

/// A phantom (auto-backup before restore, 24h TTL)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Phantom {
    pub id: i64,
    pub phantom_num: i64,
    pub timestamp: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub message: Option<String>,
    pub is_deleted: bool,
}

impl Phantom {
    pub fn display_name(&self) -> String {
        format!("phantom-{}", self.phantom_num)
    }

    pub fn remaining_ttl(&self) -> chrono::TimeDelta {
        self.expires_at.signed_duration_since(Utc::now())
    }

    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

// ─── File Entry ──────────────────────────────────────────────────────────

/// A file entry within a snapshot (epoch or phantom)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileEntry {
    pub id: i64,
    pub snapshot_id: i64,
    pub snapshot_type: SnapshotType,
    pub file_path: String,
    pub ink_hash: Option<String>,
    pub file_size: Option<i64>,
    pub modified_at: Option<String>,
    pub status: FileStatus,
    pub is_binary: bool,
}

// ─── Ink ─────────────────────────────────────────────────────────────────

/// Ink metadata (content-addressable storage)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InkInfo {
    pub hash: String,
    pub size: i64,
    pub compressed: bool,
    pub created_at: DateTime<Utc>,
    pub ref_count: i64,
}

// ─── Tag ─────────────────────────────────────────────────────────────────

/// A named tag for an epoch
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Tag {
    pub id: i64,
    pub epoch_id: i64,
    pub name: String,
    pub created: DateTime<Utc>,
}

// ─── Note ────────────────────────────────────────────────────────────────

/// A note attached to an epoch
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Note {
    pub id: i64,
    pub epoch_id: i64,
    pub text: String,
    pub created: DateTime<Utc>,
}

// ─── Diff ────────────────────────────────────────────────────────────────

/// A single diff hunk
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_lines: Vec<String>,
    pub new_start: usize,
    pub new_lines: Vec<String>,
    pub change_type: DiffChangeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffChangeType {
    Equal,
    Insert,
    Delete,
    Replace,
}

/// Result of comparing a file between two epochs
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_path: String,
    pub status: FileStatus,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
}

// ─── Blame ───────────────────────────────────────────────────────────────

/// A single line in a blame annotation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlameLine {
    pub line_number: usize,
    pub content: String,
    pub epoch_num: Option<i64>,
    pub epoch_display: String, // "origin", "epoch-3", etc.
}

// ─── Export ──────────────────────────────────────────────────────────────

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Zip,
    Tar,
}

impl ExportFormat {
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "zip" => Some(Self::Zip),
            "tar" => Some(Self::Tar),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar.gz",
        }
    }
}

// ─── Status Summary ──────────────────────────────────────────────────────

/// Summary of changes for `palin status`
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct StatusSummary {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
    pub unchanged: usize,
}

impl StatusSummary {
    #[allow(dead_code)]
    pub fn total_changed(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

// ─── Info ────────────────────────────────────────────────────────────────

/// Storage statistics for `palin info`
#[derive(Debug, Clone)]
pub struct PalinInfo {
    pub name: String,
    pub path: String,
    pub total_epochs: usize,
    pub total_phantoms: usize,
    pub total_inks: usize,
    pub total_files: usize,
    pub disk_usage_bytes: u64,
    pub dedup_ratio: f64,
}

// ─── Bisect (Phase 4 scaffolding) ────────────────────────────────────────

/// Bisect session state
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BisectState {
    pub palin_name: String,
    pub good_epoch: i64,
    pub bad_epoch: i64,
    pub current_epoch: i64,
    pub remaining_range: (i64, i64),
}

// ─── Name Resolution ─────────────────────────────────────────────────────

/// Result of resolving a palin name or current directory
#[derive(Debug, Clone)]
pub struct ResolvedPalin {
    pub name: String,
    pub path: std::path::PathBuf,
    pub config: PalinConfig,
}
