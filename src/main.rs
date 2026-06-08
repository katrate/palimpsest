mod commands;
mod diff;
mod storage;
mod tui;
mod types;

use clap::{Parser, Subcommand, ValueEnum};

/// ✦ Palimpsest — layer your project's history through time
#[derive(Parser)]
#[command(name = "palin", version = env!("CARGO_PKG_VERSION"), about = "A CLI tool for layering your project's history through time")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new palin tracking the current or specified directory
    Init {
        /// Name for the palin
        name: String,
        /// Directory to track (defaults to current directory)
        #[arg(value_hint = clap::ValueHint::DirPath)]
        dir: Option<std::path::PathBuf>,
    },

    /// Take a snapshot of the current state
    Snap {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Descriptive message for the snapshot
        #[arg(short = 'm', long)]
        message: Option<String>,
    },

    /// Show the timeline of all snapshots
    Log {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Compact, one-line-per-epoch view
        #[arg(long)]
        oneline: bool,
        /// Include phantoms in the timeline
        #[arg(long)]
        phantoms: bool,
    },

    /// List all palins on the system
    Ls,

    /// Show files changed since the last snapshot
    Status {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },

    /// Restore a directory to a previous epoch
    Restore {
        /// Epoch identifier (origin, epoch-N, phantom-N, or tag name)
        epoch: String,
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Safe mode — restore to a different directory (no phantom created)
        #[arg(long)]
        to: Option<std::path::PathBuf>,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
        /// Preview changes without actually restoring
        #[arg(long)]
        dry_run: bool,
    },

    /// List active phantoms with remaining TTL
    Phantoms {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },

    /// Show changes between two epochs
    Diff {
        /// First epoch identifier
        epoch1: String,
        /// Second epoch identifier
        epoch2: String,
        /// Optional: specific file to compare
        file: Option<String>,
        /// Palin name (optional, resolves from current directory)
        #[arg(short = 'n', long)]
        name: Option<String>,
    },

    /// Print a file's contents as they were at a specific epoch
    Show {
        /// Epoch identifier
        epoch: String,
        /// File path
        file: String,
        /// Palin name (optional, resolves from current directory)
        #[arg(short = 'n', long)]
        name: Option<String>,
    },

    /// Annotate each line of a file with the epoch it was last changed
    Blame {
        /// File path
        file: String,
        /// Palin name (optional, resolves from current directory)
        #[arg(short = 'n', long)]
        name: Option<String>,
    },

    /// Search file contents across all epochs for a pattern
    Grep {
        /// Regular expression pattern
        pattern: String,
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },

    /// Delete an epoch from a palin
    RmEpoch {
        /// Palin name
        name: String,
        /// Epoch number to delete
        num: i64,
    },

    /// Remove the origin from a palin
    RmOrigin {
        /// Palin name
        name: String,
    },

    /// Delete an entire palin
    RmPalin {
        /// Palin name
        name: String,
        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Garbage collect unreferenced inks
    Gc {
        /// Palin name
        name: String,
    },

    /// Rename a palin
    Rename {
        /// Current name
        old_name: String,
        /// New name
        new_name: String,
    },

    /// Show storage statistics for a palin
    Info {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },

    /// Add a pattern to the exclusion list
    Ignore {
        /// Glob pattern to exclude
        pattern: String,
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },

    /// Tag an epoch
    Tag {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Epoch identifier
        epoch: String,
        /// Tag name
        tag: String,
    },

    /// Delete a tag
    TagDel {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Tag name to delete
        tag: String,
    },

    /// List all tags for a palin
    Tags {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },

    /// Lock an epoch to prevent accidental deletion
    Lock {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Epoch identifier
        epoch: String,
    },

    /// Unlock an epoch
    Unlock {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Epoch identifier
        epoch: String,
    },

    /// Attach or view notes on an epoch
    Note {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Epoch identifier
        epoch: String,
        /// Note text (if not provided, shows existing notes)
        text: Option<String>,
    },

    /// Find files by name across all epochs
    Find {
        /// Filename pattern to search for
        filename: String,
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },

    /// Export an epoch as an archive file
    Export {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
        /// Epoch identifier
        epoch: String,
        /// Export format
        #[arg(long, value_enum, default_value = "zip")]
        format: ExportFormatArg,
        /// Output directory (defaults to current directory)
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },

    /// Launch the TUI (Phase 2)
    View {
        /// Palin name (optional, resolves from current directory)
        name: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum ExportFormatArg {
    Zip,
    Tar,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, dir } => {
            commands::init::execute(&name, dir.as_deref())?;
        }
        Commands::Snap { name, message } => {
            commands::snap::execute(name.as_deref(), message.as_deref())?;
        }
        Commands::Log {
            name,
            oneline,
            phantoms,
        } => {
            commands::log::execute(name.as_deref(), oneline, phantoms)?;
        }
        Commands::Ls => {
            commands::list::execute()?;
        }
        Commands::Status { name } => {
            commands::status::execute(name.as_deref())?;
        }
        Commands::Restore {
            epoch,
            name,
            to,
            yes,
            dry_run,
        } => {
            commands::restore::execute(&epoch, name.as_deref(), to.as_deref(), yes, dry_run)?;
        }
        Commands::Phantoms { name } => {
            commands::phantoms::execute(name.as_deref())?;
        }
        Commands::Diff {
            epoch1,
            epoch2,
            file,
            name,
        } => {
            commands::diff::execute(&epoch1, &epoch2, file.as_deref(), name.as_deref())?;
        }
        Commands::Show {
            epoch,
            file,
            name,
        } => {
            commands::show::execute(&epoch, &file, name.as_deref())?;
        }
        Commands::Blame { file, name } => {
            commands::blame::execute(&file, name.as_deref())?;
        }
        Commands::Grep { pattern, name } => {
            commands::grep::execute(&pattern, name.as_deref())?;
        }
        Commands::RmEpoch { name, num } => {
            let conn = storage::open_db(&name)?;
            remove_epoch(&conn, &name, num)?;
        }
        Commands::RmOrigin { name } => {
            let conn = storage::open_db(&name)?;
            let epoch = storage::get_epoch_by_num(&conn, 0)?
                .ok_or_else(|| anyhow::anyhow!("Origin not found for '{}'", name))?;
            if epoch.is_locked {
                anyhow::bail!("Origin is locked. Use `palin unlock` first.");
            }
            let entries = storage::get_file_entries(&conn, epoch.id, "epoch")?;
            for entry in &entries {
                if let Some(ref hash) = entry.ink_hash {
                    storage::decrement_ink_ref(&conn, hash)?;
                }
            }
            storage::delete_epoch(&conn, epoch.id)?;
            println!("✦ Removed origin from '{}'", name);
        }
        Commands::RmPalin { name, yes } => {
            if !yes {
                print!("  Delete entire palin '{}'? This cannot be undone. [y/N]: ", name);
                use std::io::Write;
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("  Deletion cancelled.");
                    return Ok(());
                }
            }
            storage::unregister_palin(&name)?;
            let palin_dir = crate::types::palimpsest_dir()?.join(&name);
            if palin_dir.exists() {
                std::fs::remove_dir_all(&palin_dir)?;
            }
            println!("✦ Deleted palin '{}'", name);
        }
        Commands::Gc { name } => {
            let conn = storage::open_db(&name)?;
            let unreferenced = storage::get_unreferenced_inks(&conn)?;
            if unreferenced.is_empty() {
                println!("✦ No unreferenced inks to clean up for '{}'.", name);
                return Ok(());
            }
            let total_size: u64 = unreferenced.iter().map(|i| i.size as u64).sum();
            print!(
                "  Delete {} unreferenced ink(s) ({} bytes)? [y/N]: ",
                unreferenced.len(),
                total_size
            );
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("  GC cancelled.");
                return Ok(());
            }
            let mut deleted = 0usize;
            for ink in &unreferenced {
                storage::ink::delete_ink_file(&name, &ink.hash)?;
                storage::delete_ink_from_db(&conn, &ink.hash)?;
                deleted += 1;
            }
            println!(
                "✦ GC complete: deleted {} unreferenced ink(s) ({} bytes reclaimed)",
                deleted, total_size
            );
        }
        Commands::Rename { old_name, new_name } => {
            commands::rename::execute(&old_name, &new_name)?;
        }
        Commands::Info { name } => {
            commands::info::execute(name.as_deref())?;
        }
        Commands::Ignore { pattern, name } => {
            commands::ignore::execute(&pattern, name.as_deref())?;
        }
        Commands::Tag { name, epoch, tag } => {
            commands::tag::execute_add(name.as_deref(), &epoch, &tag)?;
        }
        Commands::TagDel { name, tag } => {
            commands::tag::execute_delete(name.as_deref(), &tag)?;
        }
        Commands::Tags { name } => {
            commands::tag::execute_list(name.as_deref())?;
        }
        Commands::Lock { name, epoch } => {
            commands::lock::execute_lock(name.as_deref(), &epoch)?;
        }
        Commands::Unlock { name, epoch } => {
            commands::lock::execute_unlock(name.as_deref(), &epoch)?;
        }
        Commands::Note {
            name,
            epoch,
            text,
        } => {
            commands::note::execute(name.as_deref(), &epoch, text.as_deref())?;
        }
        Commands::Find { filename, name } => {
            commands::find::execute(&filename, name.as_deref())?;
        }
        Commands::Export {
            name,
            epoch,
            format,
            output,
        } => {
            let fmt = match format {
                ExportFormatArg::Zip => types::ExportFormat::Zip,
                ExportFormatArg::Tar => types::ExportFormat::Tar,
            };
            commands::export::execute(name.as_deref(), &epoch, fmt, output.as_deref())?;
        }
        Commands::View { name } => {
            let resolved = commands::snap::resolve_palin(name.as_deref())?;
            let mut app = tui::app::App::new(resolved)?;
            app.run()?;
        }
    }

    Ok(())
}

fn remove_epoch(conn: &rusqlite::Connection, name: &str, epoch_num: i64) -> anyhow::Result<()> {
    let epoch = storage::get_epoch_by_num(conn, epoch_num)?
        .ok_or_else(|| anyhow::anyhow!("Epoch {} not found for '{}'", epoch_num, name))?;

    if epoch.is_locked {
        anyhow::bail!("Epoch {} is locked. Use `palin unlock` first.", epoch_num);
    }

    let entries = storage::get_file_entries(conn, epoch.id, "epoch")?;
    for entry in &entries {
        if let Some(ref hash) = entry.ink_hash {
            storage::decrement_ink_ref(conn, hash)?;
        }
    }

    storage::delete_epoch(conn, epoch.id)?;
    println!("✦ Deleted {} from '{}'", epoch.display_name(), name);
    Ok(())
}
