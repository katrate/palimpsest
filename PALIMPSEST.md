# ✦ Palimpsest

*A CLI + TUI tool for layering your project's history through time.*

> Like a palimpsest (a manuscript where old writing remains visible beneath new writing), Palimpsest layers snapshots of your files so you can peel back time and see how everything looked at any point.

---

## Quick Install

Install Palimpsest with a single command — no npm, no Rust, no tokens needed.

### macOS / Linux

```bash
curl -sSL https://raw.githubusercontent.com/katrate/palimpsest/main/scripts/install.sh | sh
```

### Windows (PowerShell 5.1+)

```powershell
powershell -c "irm https://raw.githubusercontent.com/katrate/palimpsest/main/scripts/install.ps1 | iex"
```

> The script downloads the pre-built binary from the latest GitHub Release
> and places it in `/usr/local/bin` (or `~/.local/bin` as fallback on
> macOS/Linux, or `%USERPROFILE%\.palin\bin` and adds it to your PATH on
> Windows).

### Verify

```bash
palin --help
```

### Build from source (requires Rust)

```bash
cargo build --release
./target/release/palin --help
```

---

## Publishing

New releases are automatic — just push a tag:

```bash
git tag v0.2.0
git push origin v0.2.0
```

GitHub Actions will build binaries for Windows/macOS/Linux (x64 + ARM64) and create a GitHub Release with downloads.

---

## Table of Contents

1. [The Concept](#the-concept)
2. [Naming & Terminology](#naming--terminology)
3. [Storage Architecture](#storage-architecture)
4. [Commands Reference](#commands-reference)
5. [Usage Flow](#usage-flow)
6. [Name Resolution (Context-Aware)](#name-resolution-context-aware)
7. [SQLite Schema](#sqlite-schema)
8. [TUI Design](#tui-design)
9. [Project Structure](#project-structure)
10. [Tech Stack](#tech-stack)
11. [Build Phases](#build-phases)

---

## The Concept

Palimpsest is a **versioned snapshot browser** — think GitHub's file history view, but as a **terminal user interface (TUI)** for your local projects.

You create a **palin** (palimpsest instance) for any directory you want to track. The first snapshot is called the **Origin**. Every subsequent snapshot is an **Epoch**. You can browse the full timeline, compare epochs, view diffs, and manage history — all from a beautiful Ratatui-powered terminal interface.

### Key Design Principles

- **User-defined names**: Each tracked project gets a human-readable name, chosen by you at init time.
- **Centralized storage**: All palin data lives in `%USERPROFILE%\palimpsest\palin\` — not inside your project.
- **Context-aware**: Commands work in the current directory without needing to specify the palin name.
- **Git-inspired, not a git clone**: No staging, no branching, no remotes. Just pure snapshot-and-browse.

---

## Naming & Terminology

| Concept | Palimpsest Term | Meaning |
|---|---|---|
| **Tool name** | **Palimpsest** | Ancient reuse of parchment — old layers visible beneath new |
| **Executable** | `palin` | Short, punchy, easy to type |
| **Tracked project** | **palin** | An instance of palimpsest tracking a directory |
| **First snapshot** | **Origin** | The founding layer — everything starts here |
| **Subsequent snapshots** | **Epoch #1, #2, #3...** | Incremental layers of time on top of the Origin |
| **Take a snapshot** | **`palin snap`** | "Take an epoch snapshot" |
| **View history** | **`palin log`** | Timeline of all epochs |
| **Compare** | **`palin diff`** | See what changed between epochs |
| **Restore directory** | **`palin restore`** | Time travel — revert to a previous epoch |
| **Auto-backup before restore** | **Phantom** | Temporary save that self-destructs after 24h |
| **Stored file content** | **Ink** | Content-addressed by SHA-256 hash, deduplicated across epochs |
| **Launch TUI** | **`palin view`** | "View through time" |

---

## Storage Architecture

### Directory Layout

```
%USERPROFILE%\palimpsest\palin\
├── registry.toml                   # Global registry: palin name → tracked directory
│
├── my-website\                     # Palin: user-defined name
│   ├── config.toml                 # Config: exclusions, auto-snapshot interval, etc.
│   ├── db.sqlite                   # SQLite index: origins, epochs, files, ink refs
│   ├── inks\                       # Content-addressable inks (SHA-256 hashed file contents)
│   │   ├── a1b2c3d4e5f6...
│   │   └── ...
│   └── epochs\                     # Manifest files (JSON)
│       ├── origin.json             # The origin manifest
│       ├── epoch-001.json          # Epoch manifests
│       ├── epoch-002.json
│       └── ...
│
├── game-project\                   # Another palin
│   ├── config.toml
│   ├── db.sqlite
│   ├── inks\
│   ├── epochs\
│   └── phantoms\                   # Auto-created before restores (24h TTL)
│       ├── phantom-001.json
│       └── phantom-002.json
│
└── dotfiles\                       # And another
    └── ...
```

### `registry.toml`

Maps palin names to their tracked directories so `palin snap` (without args) can resolve from the current directory.

```toml
[my-website]
path = "C:\\Projects\\my-website"
created = "2026-06-06T14:32:00"

[game-project]
path = "D:\\Dev\\game"
created = "2026-06-06T15:00:00"

[dotfiles]
path = "C:\\Users\\HP"
created = "2026-06-07T09:15:00"
```

### `config.toml` (per palin)

```toml
name = "my-website"
path = "C:\\Projects\\my-website"
created = "2026-06-06T14:32:00"

[snapshots]
auto_interval_minutes = 30          # For daemon mode
compress_after_days = 7             # Compress old inks with zstd

[excludes]
patterns = [
    "node_modules/**",
    ".git/**",
    "target/**",
    "*.log",
    "*.tmp"
]

[binaries]
skip_content = true                  # Track metadata only for binary files
```

### Ink Store (Content-Addressable Storage)

- Each unique file content is **SHA-256 hashed** → ink stored in `inks/` with the hash as filename
- Identical files across epochs **deduplicate automatically** — the same hash just gets referenced again
- Binary files: metadata (path, size, modified time) is tracked, but content is skipped by default (configurable)
- Old inks (configurable, default 7+ days) are **compressed with zstd** to save space

---

## Commands Reference

### Initialization

| Command | Description |
|---|---|
| `palin init <name>` | Create a new palin tracking the current directory |
| `palin init <name> <dir>` | Create a new palin tracking a specific directory |
| `palin ls` | List all palins on the system |

### Snapshotting

| Command | Description |
|---|---|
| `palin snap` | Take an epoch of the current directory's palin |
| `palin snap <name>` | Take an epoch of a specific palin (from anywhere) |
| `palin snap -m "message"` | Take an epoch with a descriptive message |
| `palin status` | Show files changed since the last epoch |

### History & Diff

| Command | Description |
|---|---|
| `palin log` | Show timeline of all epochs (current directory) |
| `palin log <name>` | Show timeline for a specific palin |
| `palin log --oneline` | Compact view (one line per epoch) |
| `palin log --phantoms` | Include phantoms in the timeline |
| `palin diff <epoch1> <epoch2>` | Compare two epochs in current palin |
| `palin diff <epoch1> <epoch2> <file>` | Compare a specific file between two epochs |
| `palin origin` | Show details about the origin |

### Restore

| Command | Description |
|---|---|
| `palin restore <epoch>` | Restore tracked directory to an epoch (creates phantom first) |
| `palin restore <epoch> -y` | Skip confirmation prompt |
| `palin restore <epoch> --to <dir>` | Safe mode — restore to a different directory (no phantom) |
| `palin restore <epoch> --dry-run` | Preview changes without actually restoring |
| `palin restore phantom-1` | Undo a restore — go back to a phantom state |
| `palin phantoms` | List active phantoms with remaining TTL |

### TUI

| Command | Description |
|---|---|
| `palin view` | Launch the Ratatui TUI for the current directory's palin |
| `palin view <name>` | Launch the TUI for a specific palin |

### File Inspection

| Command | Description |
|---|---|
| `palin show <epoch> <file>` | Print a file's contents exactly as they were at that epoch |
| `palin blame <file>` | Annotate each line of a file with the epoch it was last changed (like `git blame`) |
| `palin grep <pattern>` | Search file contents across all epochs for a pattern |

### Management

| Command | Description |
|---|---|
| `palin rm <name> epoch <num>` | Delete a specific epoch (inks NOT deleted — see `gc`) |
| `palin rm <name> origin` | Remove the origin from the index (inks preserved) |
| `palin rm <name>` | Delete an entire palin (prompts for confirmation) |
| `palin gc <name>` | Garbage collect — purge unreferenced inks (no longer used by any epoch or phantom) |
| `palin rename <old-name> <new-name>` | Rename a palin |
| `palin info [name]` | Show storage stats: epochs, inks, disk usage, dedup ratio |
| `palin lock <name> <epoch>` | Lock an epoch — prevent accidental deletion |
| `palin unlock <name> <epoch>` | Unlock an epoch |
| `palin ignore <pattern>` | Add a pattern to the exclusion list |

### Tagging

| Command | Description |
|---|---|
| `palin tag <name> <epoch> <tag>` | Give a named tag to an epoch (e.g. `palin tag epoch-3 v1.0`) |
| `palin tag <name> <epoch> <tag> -d` | Remove a tag |
| `palin tags [name]` | List all tags for a palin |
| `palin restore <tag>` | Restore to a tagged epoch by tag name |

### Search & Export

| Command | Description |
|---|---|
| `palin find [name] <filename>` | Search across all epochs for a file by name |
| `palin export <name> <epoch> [--format zip]` | Export an epoch as an archive file |
| `palin export <name> <epoch> [--format tar]` | Export as tar archive |

### Notes & Annotation

| Command | Description |
|---|---|
| `palin note <name> <epoch> <text>` | Attach a note/annotation to an epoch |
| `palin note <name> <epoch>` | View notes for an epoch |

### Bisect (Bug Hunting)

| Command | Description |
|---|---|
| `palin bisect start <name> <good-epoch> <bad-epoch>` | Start a binary search through epochs |
| `palin bisect good` | Mark current epoch as good |
| `palin bisect bad` | Mark current epoch as bad |
| `palin bisect reset` | Cancel bisect session |

### Advanced

| Command | Description |
|---|---|
| `palin fork <name> <new-name> [new-path]` | Fork a palin — copy all data to a new palin at a different path |
| `palin merge <source> <target>` | Merge two palins' timelines into one |
| `palin cross-diff <palin1>:<epoch> <palin2>:<epoch>` | Compare epochs across different palins |
| `palin export --web [name]` | Generate a static HTML page of the timeline |

### Daemon (Phase 3)

| Command | Description |
|---|---|
| `palin daemon start <name>` | Start auto-snapshot daemon for a palin |
| `palin daemon stop <name>` | Stop auto-snapshot daemon |
| `palin daemon status` | Show running daemons |

---

## Usage Flow

### Basic Workflow

```bash
# Start tracking a project
cd C:\Projects\my-website
palin init my-website

# Record the initial state
palin snap -m "project bootstrap"

# Make some changes...

# Record the new state
palin snap -m "added authentication"

# View history
palin log

# See what changed
palin diff origin epoch-1

# Browse visually
palin view

# Make more changes, record again
palin snap -m "redesigned dashboard"
palin snap -m "fixed login bug"

# Oops, epoch 2 was a mistake
palin rm my-website epoch 2

# Clean up orphaned inks later
palin gc my-website

# Time travel — restore to epoch 1
palin restore my-website epoch-1
  # → Confirms: "Restore to epoch-1? Current state will be saved as phantom-1"
  # → Yes: creates phantom-1 of current state, restores to epoch-1

# Changed my mind — undo the restore
palin restore my-website phantom-1
  # → Creates phantom-2 of current state
  # → Restores to phantom-1 (the state before the first restore)

# See active phantoms
palin phantoms
  # phantom-1 — expires in 23h 14m
  # phantom-2 — expires in 23h 59m

# 24 hours later — phantoms auto-deleted on next command
palin snap -m "back to work"
  # → phantom-1 and phantom-2 cleaned up automatically

# Inspect a file at a specific epoch
palin show epoch-3 src/main.rs

# Blame — see which epoch last changed each line
palin blame src/config.rs

# Search contents across all epochs
palin grep "TODO"
```

### Multiple Projects

```bash
# Track another project
cd D:\Dev\game
palin init game-project
palin snap -m "initial build"

# Work on my-website, but need to check game-project
cd C:\Projects\my-website
palin log game-project          # Works from anywhere with explicit name
palin view game-project         # Launch TUI for game-project
```

---

## Name Resolution (Context-Aware)

When you run `palin <cmd>` **without** a name argument:

1. Palimpsest reads `%USERPROFILE%\palimpsest\palin\registry.toml`
2. It finds which palin's `path` matches (or is a parent of) your **current working directory**
3. Resolution rules:
   - **0 matches** → Error: *"No palin found for this directory. Did you mean to `palin init <name>` here?"*
   - **1 match** → Use that palin ✓
   - **2+ matches** → Error: *"Multiple palins match this directory. Specify one by name."*

When you **do** specify a name (`palin snap my-website`), it operates directly on that palin regardless of your current directory.

---

## SQLite Schema

Each palin gets its own `db.sqlite` file.

```sql
-- Epochs (snapshots)
CREATE TABLE epochs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch_num   INTEGER NOT NULL,        -- 0 = origin, 1, 2, 3...
    timestamp   TEXT NOT NULL,            -- ISO 8601
    message     TEXT,
    is_origin   BOOLEAN NOT NULL DEFAULT 0,
    is_deleted  BOOLEAN NOT NULL DEFAULT 0
);

-- Phantoms (auto-backups before restore, 24h TTL)
CREATE TABLE phantoms (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    phantom_num INTEGER NOT NULL,         -- 1, 2, 3...
    timestamp   TEXT NOT NULL,            -- ISO 8601 when created
    expires_at  TEXT NOT NULL,            -- ISO 8601 when auto-deleted (timestamp + 24h)
    message     TEXT,                     -- Description (e.g. "before restore to epoch-7")
    is_deleted  BOOLEAN NOT NULL DEFAULT 0
);

-- File entries in each epoch or phantom
CREATE TABLE file_entries (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id    INTEGER NOT NULL,      -- References epoch or phantom
    snapshot_type  TEXT NOT NULL,         -- 'epoch' or 'phantom'
    file_path      TEXT NOT NULL,         -- Relative path from tracked root
    ink_hash       TEXT,                  -- NULL for binary/deleted files
    file_size      INTEGER,
    modified_at    TEXT,                  -- File's last modified timestamp
    status         TEXT NOT NULL,         -- 'added', 'modified', 'deleted', 'unchanged'
    is_binary      BOOLEAN NOT NULL DEFAULT 0
);

-- Ink references (for GC tracking)
CREATE TABLE inks (
    hash        TEXT PRIMARY KEY,          -- SHA-256 hash
    size        INTEGER NOT NULL,
    compressed  BOOLEAN NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    ref_count   INTEGER NOT NULL DEFAULT 0
);

-- Tags (named checkpoints)
CREATE TABLE tags (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch_id  INTEGER NOT NULL REFERENCES epochs(id),
    name      TEXT NOT NULL UNIQUE,
    created   TEXT NOT NULL
);

-- Notes (epoch annotations)
CREATE TABLE notes (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch_id  INTEGER NOT NULL REFERENCES epochs(id),
    text      TEXT NOT NULL,
    created   TEXT NOT NULL
);

-- Metadata
CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

---

## TUI Design

### Layout (Ratatui)

```
┌──────────────────────────────────────────────────────────────┐
│  ◆ Palimpsest — my-website                       Auto: 30m │
├──────────────────────────────────────────────────────────────┤
│  Timeline                │  Epoch #12  —  Today 14:32       │
│                          │  "added error handling"           │
│  ── Epoch #12  ◄ now    │                                   │
│  ── Epoch #11            │  Files changed (12):             │
│  ── Epoch #10            │  ● src/main.rs     ┌────────────┐│
│  ── Epoch #9             │    +15 / -3        │ @@ -42,7 + ││
│  ── Epoch #8             │  ● src/lib.rs      │ - old line  ││
│  ── Epoch #7             │    +42 / -10       │ + new line  ││
│  ══ Origin  ◆            │  ● Cargo.toml      │             ││
│                          │    +1 / -0         └────────────┘│
│  [?] Help  [q] Quit      │                                   │
│  [s] Snap  [/] Search    │  Tab: switch panel               │
└──────────────────────────────────────────────────────────────┘
```

### Views

| View | Description |
|---|---|
| **Dashboard** | Overview: latest epoch, total files, storage used, recent changes |
| **Timeline Explorer** | Left: epoch list grouped by date. Right: file changes for selected epoch |
| **File Tree** | Expandable directory tree showing added/modified/deleted files |
| **Diff Viewer** | Unified or side-by-side diff with line highlighting |
| **Search** | Find files across all epochs and see their history |
| **Blame View** | Line-by-line annotation showing which epoch last changed each line |

### Keybindings

| Key | Action |
|---|---|
| `j` / `k` | Scroll up/down |
| `Enter` | Expand/collapse or select |
| `Tab` | Switch panel |
| `/` | Search |
| `s` | Take a snapshot |
| `d` | Show diff |
| `r` | Restore to selected epoch (with phantom safety) |
| `p` | List active phantoms |
| `q` | Quit |
| `?` | Help overlay |

### Color Coding

- **Green** — Added files
- **Yellow** — Modified files
- **Red** — Deleted files
- **Blue** — Currently selected item
- **Gray** — Unchanged files (dimmed)

---

## Project Structure

```
palin/
├── Cargo.toml
├── src/
│   ├── main.rs                    # CLI entry point (clap)
│   │
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── init.rs                # palin init
│   │   ├── snap.rs                # palin snap
│   │   ├── log.rs                 # palin log
│   │   ├── diff.rs                # palin diff
│   │   ├── show.rs                # palin show (print file at epoch)
│   │   ├── blame.rs               # palin blame (annotate lines)
│   │   ├── grep.rs                # palin grep (search across epochs)
│   │   ├── status.rs              # palin status
│   │   ├── restore.rs             # palin restore (with phantom safety)
│   │   ├── phantoms.rs            # palin phantoms (list + auto-clean)
│   │   ├── rm.rs                  # palin rm
│   │   ├── gc.rs                  # palin gc
│   │   ├── list.rs                # palin ls
│   │   └── view.rs                # palin view (launches TUI)
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── db.rs                  # SQLite schema + queries (epochs + phantoms + blame)
│   │   ├── ink.rs                 # Content-addressable ink store (read/write/compress)
│   │   ├── walker.rs              # Filesystem walker with exclusions
│   │   ├── registry.rs            # registry.toml read/write
│   │   └── restore.rs             # Restore engine: apply snapshot → directory
│   │
│   ├── diff/
│   │   ├── mod.rs
│   │   └── engine.rs              # Line-level diff (Myers algorithm)
│   │
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── app.rs                 # App state + event loop
│   │   ├── ui.rs                  # Main layout + rendering
│   │   ├── dashboard.rs           # Dashboard widget
│   │   ├── timeline.rs            # Timeline explorer widget (includes phantoms)
│   │   ├── filetree.rs            # File tree widget
│   │   ├── diffview.rs            # Diff viewer widget
│   │   ├── blameview.rs           # Blame view widget (line annotations)
│   │   ├── search.rs              # Search widget
│   │   ├── restore_dialog.rs      # Restore confirmation dialog
│   │   └── theme.rs               # Color theme
│   │
│   └── types.rs                   # Shared types (Epoch, Phantom, FileEntry, etc.)
│
├── tests/
│   ├── storage_test.rs
│   ├── walker_test.rs
│   ├── diff_test.rs
│   └── blame_test.rs
│
└── PALIMPSEST.md                  # This file
```

---

## Tech Stack

| Component | Choice | Why |
|---|---|---|
| **Language** | **Rust** | Performance, safety, excellent TUI ecosystem |
| **CLI Framework** | **clap** (v4) | Derive-based argument parsing, industry standard |
| **TUI Framework** | **ratatui** | The standard Rust TUI library (fork of tui-rs) |
| **Terminal Backend** | **crossterm** | Cross-platform terminal handling (Windows included) |
| **Database** | **rusqlite** | Embedded SQLite — fast, no daemon needed |
| **Hashing** | **sha2** (SHA-256) | Content-addressable ink storage |
| **Serialization** | **serde** + **serde_json** + **toml** | For manifests and config files |
| **Diff Engine** | **similar** | Myers diff implementation with line-level comparisons |
| **Grep/Blame Engine** | **regex** + custom | Pattern search and line-tracking across epochs |
| **Compression** | **zstd** | Fast compression for old inks |
| **Archiving** | **zip** crate | Export epochs as zip/tar archives |
| **Error Handling** | **anyhow** | Simple error propagation |
| **Async Runtime** | **tokio** (light) | Only for daemon mode (auto-snap) |

---

## Build Phases

### Phase 1 — Core Engine (CLI only)

**Snapshot & History Core**
- [ ] Rust project scaffold with clap CLI
- [ ] `palin init <name> [dir]` — creates storage dir + SQLite DB + registry entry + config
- [ ] `palin snap [name]` — walks tracked directory, hashes files, stores inks, records origin or epoch
- [ ] `palin log [name]` — queries SQLite, prints timeline (with `--phantoms` flag)
- [ ] `palin ls` — reads registry, lists all palins
- [ ] `palin status [name]` — shows files changed since last epoch
- [ ] Name resolution from current directory via registry

**Restore & Phantom System**
- [ ] `palin restore <epoch>` — restore directory to an epoch (auto-creates phantom, confirmation prompt)
- [ ] `palin restore <epoch> --to <dir>` — safe restore to different directory
- [ ] `palin restore phantom-1` — restore to a phantom (undo a restore)
- [ ] `palin phantoms` — list active phantoms with remaining TTL
- [ ] Phantom auto-cleanup (check expiry on every command)

**Management**
- [ ] `palin rm <name> epoch <num>` / `palin rm <name> origin` — removes epoch from index
- [ ] `palin gc <name>` — garbage collects unreferenced inks (checks epochs + phantoms)
- [ ] `palin rename <old> <new>` — rename palins
- [ ] `palin info [name]` — storage stats with dedup ratio
- [ ] `palin ignore <pattern>` — add exclusion patterns on the fly

**Tagging & Notes**
- [ ] `palin tag <epoch> <tag>` / `palin tags` / `palin restore <tag>` — named checkpoints
- [ ] `palin lock <epoch>` / `palin unlock <epoch>` — prevent accidental deletion
- [ ] `palin note <epoch> <text>` — annotate epochs
- [ ] `palin find <filename>` — search across all epochs for a file

**File Inspection**
- [ ] `palin show <epoch> <file>` — print file contents at an epoch
- [ ] `palin blame <file>` — annotate each line with last-changing epoch
- [ ] `palin grep <pattern>` — search file contents across all epochs

**Export**
- [ ] `palin export <epoch> --format zip | tar` — export epoch as archive

### Phase 2 — Ratatui TUI

- [ ] `palin view [name]` — launches TUI
- [ ] Timeline explorer widget (epoch list + phantom indicators + tag badges)
- [ ] File tree widget (changes in selected epoch)
- [ ] Diff viewer widget (unified diff)
- [ ] Blame view widget (line-by-line epoch annotations)
- [ ] Search widget (find files + epochs + grep results)
- [ ] Restore confirmation dialog with phantom warning
- [ ] Keyboard navigation, panel switching, help overlay
- [ ] Color-coded file status indicators
- [ ] Dark/light theme toggle
- [ ] Animated timeline visualization

### Phase 3 — Daemon & Auto-Snapshots

- [ ] `palin daemon start <name>` — background auto-snapshot every N minutes
- [ ] `palin daemon stop <name>` — stop the daemon
- [ ] Configurable interval + exclusions in `config.toml`
- [ ] Old ink compression with zstd

### Phase 4 — Advanced Features

- [ ] `palin bisect start/good/bad/reset` — binary search through epochs for bugs
- [ ] `palin fork <name> <new-name> [path]` — fork a palin (copy all data to new path)
- [ ] `palin merge <source> <target>` — merge two palins' timelines
- [ ] `palin cross-diff <a>:<epoch> <b>:<epoch>` — compare across palins
- [ ] `palin export --web [name]` — generate static HTML timeline page

### Phase 5 — Polish

- [ ] Windows long path handling (`\\?\` prefix)
- [ ] NTFS junction/reparse point awareness
- [ ] Graceful handling of permission-denied files
- [ ] Performance optimization (parallel hashing with rayon)
- [ ] `palin diff` with pretty terminal output
- [ ] Undo/restore deleted epoch
- [ ] Tab completions (clap completions)
- [ ] Progress bar on first snapshot (large projects)

---

## License

MIT
