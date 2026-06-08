# ✦ Palimpsest

**Layer your project's history through time** — a CLI + TUI tool for versioned snapshots with filesystem-level deduplication.

> Like a palimpsest (a manuscript where old writing remains visible beneath new writing), Palimpsest layers snapshots of your files so you can peel back time and see how everything looked at any point.

[![Release](https://img.shields.io/github/v/release/katrate/palimpsest)](https://github.com/katrate/palimpsest/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## Quick Install

```bash
curl -sSL https://raw.githubusercontent.com/katrate/palimpsest/master/scripts/install.sh | sh
```

**Windows (PowerShell):**
```powershell
powershell -c "irm https://raw.githubusercontent.com/katrate/palimpsest/master/scripts/install.ps1 | iex"
```

### Auto-Update

Once installed, update to the latest version with a single command:

```bash
palin -u
```

This fetches the latest release from GitHub, downloads the matching binary for your platform, and replaces the current installation.

### Build from Source

```bash
cargo build --release
./target/release/palin --help
```

---

## Commands

| Command | Description |
|---|---|
| `palin init <name> [dir]` | Create a new palin tracking a directory |
| `palin snap [-m "message"]` | Take a snapshot of the current state |
| `palin log [--oneline] [--phantoms]` | Show timeline of all snapshots |
| `palin diff <epoch1> <epoch2> [file]` | Compare two epochs |
| `palin show <epoch> <file>` | Print a file as it was at an epoch |
| `palin blame <file>` | Annotate lines with last-changing epoch |
| `palin restore <epoch> [-y] [--to dir]` | Restore directory to a previous epoch |
| `palin status` | Show files changed since last snapshot |
| `palin ls` | List all palins |
| `palin info [name]` | Show storage statistics |
| `palin gc <name>` | Garbage collect unreferenced ink blobs |
| `palin lock/unlock <name> <epoch>` | Protect/unprotect an epoch from deletion |
| `palin tag <name> <epoch> <tag>` | Name a checkpoint |
| `palin find <filename>` | Search for files across all epochs |
| `palin export <name> <epoch>` | Export an epoch as zip/tar archive |
| `palin view [name]` | **Launch the TUI browser** |

---

## TUI — `palin view`

Launch a full-screen terminal UI to browse your project's timeline visually.

```
┌──────────────────────────────────────────────────────────────┐
│  ◆ my-website                        [GC] [Info] [Rename]   │
│  🔍 Search files...                                           │
├───────────────────────┬──────────────────────────────────────┤
│  Timeline             │  Epoch #12 — 2026-06-08 14:32:00     │
│                       │  "added error handling"              │
│  ▸ origin     ◆       │                                      │
│    epoch-1            │  Files changed (12):                 │
│    epoch-2            │    ~ src/main.rs      +15.3 KB       │
│    epoch-3   🔒       │    ~ src/lib.rs        +4.2 KB       │
│    epoch-4            │    + Cargo.toml         +1.2 KB       │
│    epoch-5            │    - old_config.yml     -0.8 KB       │
│    epoch-6            │                                      │
│    epoch-7            │  Diff: epoch-6 vs epoch-12           │
│    epoch-8            │    + fn authenticate()               │
│    epoch-9            │    - fn login()                      │
│    epoch-10           │    - fn validate()                   │
│    epoch-11           │                                      │
│  ── Phantoms ──       │                                      │
│    ◇ phantom-1        │                                      │
├───────────────────────┴──────────────────────────────────────┤
│  q  Quit  ?  Help  p  Pick  i  Info  d  Diff  r  Rld        │
│  s  Snap     3 ep · 1 ph · 12 files              Timeline    │
│ ┌─ Messages ──────────────────────── [Clear] ─┐              │
│ │  ◆ Snapshot of 'my-website' created         │              │
│ │  ✓ Restored to epoch-10                     │              │
│ └─────────────────────────────────────────────┘              │
└──────────────────────────────────────────────────────────────┘
```

### Layout

| Section | Description |
|---|---|
| **Header** | Palin name + right-aligned action buttons: `[GC] [Info] [Rename] [◆ Snap]` |
| **Search bar** | Press `/` to search files by name across the current epoch |
| **Timeline** (left) | Chronological list of epochs and phantoms with lock/restore/delete controls |
| **Details** (right) | File tree for the selected epoch with status colors and file sizes |
| **Status bar** | Keyboard shortcuts + epoch/phantom/file counts + focus indicator |
| **Messages** | Scrollable panel showing command history with `[Clear]` button |

### Mouse Controls

| Action | Behavior |
|---|---|
| **Click** an epoch/phantom | Select it and load its file tree |
| **Click** `[L]` / `[ ]` | Toggle epoch lock |
| **Click** `[↩]` | Initiate restore (Shift+click to auto-confirm) |
| **Click** `[✕]` | Initiate delete (Shift+click to auto-confirm) |
| **Click** `[◆ Snap]` | Take a new snapshot |
| **Click** `[Info]` | Show storage statistics |
| **Click** `[GC]` | Garbage collect unreferenced inks |
| **Click** `[Rename]` | Rename the current palin |
| **Scroll wheel** | Scroll diff view, file preview, and message history |
| **Click search bar** | Focus the search input |
| **Click `[Clear]`** | Clear the message history panel |

### Keyboard Shortcuts

| Key | Action |
|---|---|
| **↑ / ↓** | Navigate timeline / file list |
| **← / →** | Switch focus between Timeline and Files panels |
| **Tab / Shift+Tab** | Switch focus between panels |
| **PageUp / PageDown** | Page through timeline |
| **g / G** | Go to first / last timeline item |
| **Enter / Space** | Preview a file (or expand/collapse a directory) |
| **Esc** | Close diff view / preview / quit |
| **d** | Start file compare (on a file) → finish on epoch |
| **s** | Take a snapshot |
| **i** | Show storage info |
| **o** | Scroll message history down |
| **c** | Clear message history |
| **p** | Open palin picker (switch projects) |
| **r** | Reload timeline |
| **?** | Show help overlay |
| **q** | Quit |

### File Compare Flow

1. Navigate to a file in the file panel with **↑/↓**
2. Press **d** — switches focus to Timeline with "Select an epoch..."
3. Navigate to a different epoch with **↑/↓**
4. Press **d** again — compares the file between the two epochs with color-coded diff

Files are compared using the **Myers diff algorithm** via the `similar` crate. Results show:
- **Green (+)** — Added lines
- **Red (-)** — Deleted lines
- **Red (-)** — Modified sections (shown as deletions)

### Palin Picker

Press **p** to open the palin switcher — a full-screen overlay listing all tracked projects:
- **↑/↓** / **click** — Select a palin
- **Enter** / **click** — Switch to it
- **d** — Delete the selected palin (non-current only)
- **Esc** — Close the picker

---

## Professional Theme

The TUI features a professional dark design system inspired by modern frameworks (shadcn/ui, Tailwind):

| Token | Hex | Usage |
|---|---|---|
| `BG_DEEPER` | `#0A0F1E` | Deepest background layer (footer, behind panels) |
| `BG` | `#0F172A` | Primary background (navy) |
| `SURFACE` | `#1E293B` | Elevated surfaces, header, panels |
| `SURFACE2` | `#273548` | Hovered / selected row backgrounds |
| `CYAN` | `#06B6D4` | Primary accent, selection, interactive elements |
| `CYAN_BRIGHT` | `#22D3EE` | Bright highlights, selected text |
| `VIOLET` | `#8B5CF6` | Secondary accent, tags, badges |
| `GREEN` | `#22C55E` | Success, added files, confirm buttons |
| `YELLOW` | `#F59E0B` | Warnings, modified files |
| `RED` | `#EF4444` | Errors, deleted files, deny buttons |
| `ORANGE` | `#F97316` | GC button, warnings |

Diff view uses colorized backgrounds per line type for easier visual scanning.

---

## How It Works

### Storage

```
%USERPROFILE%\.palin\
├── registry.toml              # Maps palin names → tracked directories
├── my-website\                # Your palin
│   ├── config.toml            # Exclusions, settings
│   ├── db.sqlite              # SQLite index of epochs, files, ink refs
│   └── inks\                  # Content-addressed blobs (SHA-256)
└── game-project\              # Another palin
    └── ...
```

- **Inks** — File contents stored by SHA-256 hash, deduplicated across epochs
- **Epochs** — Named snapshots (origin, epoch-1, epoch-2...)
- **Phantoms** — Auto-backups created before restore operations, self-destruct after 24h

### Key Concepts

| Term | Meaning |
|---|---|
| **Origin** | The first snapshot — the foundation layer |
| **Epoch** | Any subsequent snapshot (#1, #2, #3...) |
| **Phantom** | Temporary auto-backup (24h TTL) created before a restore |
| **Ink** | Content-addressed file blob, deduplicated by SHA-256 hash |
| **Lock** | Prevents an epoch from being deleted |

---

## Advanced

### Context-Aware Name Resolution

Commands work from the project directory without specifying the palin name — the registry maps directories to palins automatically.

### Phantom Safety Net

Every `palin restore` creates an automatic phantom backup of the current state first, giving you 24 hours to undo:

```bash
palin restore epoch-3           # Creates phantom-1, restores to epoch-3
palin restore phantom-1         # Undo — back to before the restore
palin phantoms                  # See active phantoms with TTL
```

### Garbage Collection

Inks with zero references accumulate when epochs are deleted. Clean them up:

```bash
palin gc my-website
#  3 unreferenced inks (245 KB). GC? [y/N]: y
#  GC complete: deleted 3 unreferenced inks (245 KB reclaimed)
```

Or click `[GC]` in the TUI.

---

## Tech Stack

| Component | Library |
|---|---|
| CLI Framework | `clap` v4 |
| TUI Framework | `ratatui` |
| Terminal Backend | `crossterm` |
| Database | `rusqlite` (SQLite) |
| Hashing | `sha2` (SHA-256) |
| Diff Engine | `similar` (Myers algorithm) |
| Serialization | `serde` + `toml` |
| Archiving | `zip` + `tar` + `flate2` |
| Unicode Width | `unicode-width` |

---

## License

MIT
