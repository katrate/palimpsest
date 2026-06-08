use crate::storage;
use crate::types::{Epoch, FileEntry, FileStatus, Phantom, ResolvedPalin};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use std::time::Duration;

/// Main application state for the TUI.
pub struct App {
    /// Resolved palin info (name, path, config)
    pub palin: ResolvedPalin,

    /// All epochs loaded from the database
    pub epochs: Vec<Epoch>,

    /// All phantoms loaded from the database
    pub phantoms: Vec<Phantom>,

    /// Index into the combined timeline (epochs + phantoms)
    pub selected_idx: usize,

    /// Index into the file list of the selected item
    pub selected_file_idx: usize,

    /// File entries for the currently selected epoch/phantom item
    pub current_entries: Vec<FileEntry>,

    /// Whether the help overlay is visible
    pub show_help: bool,

    /// Whether the app is still running
    pub running: bool,

    /// Terminal size cache for layout
    pub term_area: Rect,

    // ── Command mode ──────────────────────────────────────────

    /// Whether the user is typing a command
    pub command_mode: bool,

    /// The current command buffer text
    pub command_buffer: String,

    /// Cursor position within the buffer
    pub command_cursor: usize,

    /// Recent command output lines (newest first)
    pub command_output: Vec<String>,

    /// Command history (past executed commands)
    pub command_history: Vec<String>,

    /// Index into command history for navigation (-1 = no history selected)
    pub history_idx: isize,
}

impl App {
    /// Create a new TUI app for the given palin.
    pub fn new(palin: ResolvedPalin) -> Result<Self> {
        let conn = storage::open_db(&palin.name)?;
        let epochs = storage::list_epochs(&conn)?;
        let phantoms = storage::list_phantoms(&conn)?;

        let mut app = Self {
            palin,
            epochs,
            phantoms,
            selected_idx: 0,
            selected_file_idx: 0,
            current_entries: Vec::new(),
            show_help: false,
            running: true,
            term_area: Rect::default(),
            command_mode: false,
            command_buffer: String::new(),
            command_cursor: 0,
            command_output: Vec::new(),
            command_history: Vec::new(),
            history_idx: -1,
        };
        app.load_entries()?;
        Ok(app)
    }

    /// Reload epochs and phantoms from the database.
    pub fn reload(&mut self) -> Result<()> {
        let conn = storage::open_db(&self.palin.name)?;
        self.epochs = storage::list_epochs(&conn)?;
        self.phantoms = storage::list_phantoms(&conn)?;
        self.selected_idx = self.selected_idx.min(self.timeline_len().saturating_sub(1));
        self.load_entries()?;
        Ok(())
    }

    // ── Timeline helpers ──────────────────────────────────────────

    /// Build a combined timeline of all items (epochs then phantoms).
    pub fn timeline_items(&self) -> Vec<TimelineItem<'_>> {
        let mut items: Vec<TimelineItem<'_>> = self
            .epochs
            .iter()
            .map(|e| TimelineItem::Epoch(e))
            .collect();
        for p in &self.phantoms {
            items.push(TimelineItem::Phantom(p));
        }
        items
    }

    /// Total number of timeline items.
    pub fn timeline_len(&self) -> usize {
        self.epochs.len() + self.phantoms.len()
    }

    /// Load file entries for the currently selected timeline item.
    pub fn load_entries(&mut self) -> Result<()> {
        let items = self.timeline_items();
        if items.is_empty() {
            self.current_entries.clear();
            return Ok(());
        }
        let idx = self.selected_idx.min(items.len().saturating_sub(1));
        let conn = storage::open_db(&self.palin.name)?;
        match items[idx] {
            TimelineItem::Epoch(e) => {
                self.current_entries =
                    storage::get_file_entries(&conn, e.id, "epoch")?;
            }
            TimelineItem::Phantom(p) => {
                self.current_entries =
                    storage::get_file_entries(&conn, p.id, "phantom")?;
            }
        }
        self.selected_file_idx = 0;
        Ok(())
    }

    // ── Entry point: run the event loop ───────────────────────────

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = startup()?;

        // Run the event loop inside a closure so shutdown always runs even on error
        let result = (|| -> Result<()> {
            while self.running {
                terminal.draw(|frame| {
                    self.term_area = frame.area();
                    crate::tui::ui::render(self, frame);
                })?;

                if self.show_help {
                    self.handle_help_input()?;
                } else if self.command_mode {
                    self.handle_command_input()?;
                } else {
                    self.handle_main_input()?;
                }
            }
            Ok(())
        })();

        shutdown(&mut terminal)?;
        result
    }

    // ── Main input handler ────────────────────────────────────────

    fn handle_main_input(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(100))? {
            return Ok(());
        }
        let ev = event::read()?;
        if let Event::Key(KeyEvent { code, kind, modifiers, .. }) = ev {
            if kind != KeyEventKind::Press {
                return Ok(());
            }
            match code {
                KeyCode::Char(':') => {
                    self.enter_command_mode();
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.running = false;
                }
                KeyCode::Char('?') => {
                    self.show_help = true;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll_up();
                }
                KeyCode::Char('g') if modifiers == KeyModifiers::NONE => {
                    self.selected_idx = 0;
                    self.load_entries()?;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    self.selected_idx = self.timeline_len().saturating_sub(1);
                    self.load_entries()?;
                }
                KeyCode::Char('J') | KeyCode::PageDown => {
                    let step = (self.term_area.height as usize).saturating_sub(6).max(5);
                    self.selected_idx = self
                        .selected_idx
                        .saturating_add(step)
                        .min(self.timeline_len().saturating_sub(1));
                    self.load_entries()?;
                }
                KeyCode::Char('K') | KeyCode::PageUp => {
                    let step = (self.term_area.height as usize).saturating_sub(6).max(5);
                    self.selected_idx = self.selected_idx.saturating_sub(step);
                    self.load_entries()?;
                }
                KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => {
                    if self.current_entries.len() > 1 {
                        self.selected_file_idx = self
                            .selected_file_idx
                            .saturating_add(1)
                            .min(self.current_entries.len().saturating_sub(1));
                    }
                }
                KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => {
                    if self.selected_file_idx > 0 {
                        self.selected_file_idx = self.selected_file_idx.saturating_sub(1);
                    }
                }
                KeyCode::Char('r') => {
                    self.reload()?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ── Command mode input handler ──────────────────────────────

    fn handle_command_input(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(100))? {
            return Ok(());
        }
        let ev = event::read()?;
        if let Event::Key(KeyEvent { code, kind, .. }) = ev {
            if kind != KeyEventKind::Press {
                return Ok(());
            }
            match code {
                KeyCode::Esc => {
                    self.exit_command_mode();
                }
                KeyCode::Enter => {
                    self.execute_command();
                }
                KeyCode::Backspace => {
                    if self.command_cursor > 0 {
                        self.command_buffer.remove(self.command_cursor - 1);
                        self.command_cursor -= 1;
                    }
                }
                KeyCode::Delete => {
                    if self.command_cursor < self.command_buffer.len() {
                        self.command_buffer.remove(self.command_cursor);
                    }
                }
                KeyCode::Left => {
                    self.command_cursor = self.command_cursor.saturating_sub(1);
                }
                KeyCode::Right => {
                    self.command_cursor = self.command_cursor
                        .min(self.command_buffer.len());
                }
                KeyCode::Up => {
                    self.navigate_history(-1);
                }
                KeyCode::Down => {
                    self.navigate_history(1);
                }
                KeyCode::Home => {
                    self.command_cursor = 0;
                }
                KeyCode::End => {
                    self.command_cursor = self.command_buffer.len();
                }
                KeyCode::Tab => {
                    self.auto_complete();
                }
                KeyCode::Char(c) => {
                    self.command_buffer.insert(self.command_cursor, c);
                    self.command_cursor += 1;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn enter_command_mode(&mut self) {
        self.command_mode = true;
        self.command_buffer.clear();
        self.command_cursor = 0;
        self.history_idx = -1;
    }

    fn exit_command_mode(&mut self) {
        self.command_mode = false;
        self.command_buffer.clear();
        self.command_cursor = 0;
        self.history_idx = -1;
    }

    fn navigate_history(&mut self, direction: isize) {
        if self.command_history.is_empty() {
            return;
        }
        let len = self.command_history.len() as isize;
        let new_idx = self.history_idx + direction;
        if new_idx < -1 || new_idx >= len {
            return;
        }
        self.history_idx = new_idx;
        if new_idx == -1 {
            self.command_buffer.clear();
            self.command_cursor = 0;
        } else {
            let entry = &self.command_history[new_idx as usize];
            self.command_buffer = entry.clone();
            self.command_cursor = entry.len();
        }
    }

    fn auto_complete(&mut self) {
        let known_commands = [
            "snap", "log", "status", "ls", "info", "reload", "help",
            "tag", "tags", "lock", "unlock", "phantoms", "note",
            "restore", "gc", "export", "find", "show", "blame", "grep",
        ];
        let buf = self.command_buffer.trim().to_lowercase();
        if buf.is_empty() {
            return;
        }
        // Simple prefix completion
        for cmd in &known_commands {
            if cmd.starts_with(&buf) && cmd != &buf {
                self.command_buffer = cmd.to_string();
                self.command_cursor = cmd.len();
                break;
            }
        }
    }

    // ── Command execution ────────────────────────────────────────

    fn execute_command(&mut self) {
        let raw = std::mem::take(&mut self.command_buffer);
        self.command_cursor = 0;
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            self.command_mode = false;
            return;
        }

        // Add to history
        self.command_history.push(trimmed.clone());
        if self.command_history.len() > 50 {
            self.command_history.remove(0);
        }
        self.history_idx = -1;

        // Parse: split by spaces, respecting quoted strings
        let args = parse_args(&trimmed);
        if args.is_empty() {
            self.command_mode = false;
            return;
        }

        let cmd = args[0].to_lowercase();
        let rest: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

        match cmd.as_str() {
            "q" | "quit" | "exit" => {
                self.command_mode = false;
                self.running = false;
                return;
            }
            "help" | "?" => {
                self.command_mode = false;
                self.show_help = true;
                return;
            }
            "reload" => {
                self.command_mode = false;
                let _ = self.reload();
                self.add_output("✦ Reloaded timeline");
                return;
            }
            "clear" | "cls" => {
                self.command_output.clear();
                self.command_mode = false;
                return;
            }
            _ => {}
        }

        // Internal commands that don't need subprocess spawning
        if cmd == "ls" {
            self.run_internal("ls", &[]);
            return;
        }

        // Build subprocess args for the palin binary
        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                self.add_output(&format!("✖ Error: {}", e));
                return;
            }
        };

        let mut proc_args: Vec<String> = vec![cmd.clone()];

        // Inject palin name for commands that need it
        let needs_name: bool = matches!(
            cmd.as_str(),
            "snap" | "log" | "status" | "info" | "phantoms"
                | "lock" | "unlock" | "tags" | "ignore" | "gc"
                | "note" | "export"
        );

        if needs_name {
            proc_args.push(self.palin.name.clone());
            proc_args.extend(rest.iter().map(|s| s.to_string()));
        } else if cmd == "tag" || cmd == "tag-del" {
            proc_args.push(self.palin.name.clone());
            proc_args.extend(rest.iter().map(|s| s.to_string()));
        } else if cmd == "restore" {
            // restore <epoch> [name] [-y] [--dry-run]
            // Insert the palin name after the epoch argument
            if !rest.is_empty() {
                proc_args.push(rest[0].to_string()); // epoch
                proc_args.push(self.palin.name.clone());
                for a in &rest[1..] {
                    proc_args.push(a.to_string());
                }
            }
        } else if matches!(cmd.as_str(), "diff" | "show" | "blame") {
            // These use -n for name
            proc_args.push("-n".to_string());
            proc_args.push(self.palin.name.clone());
            proc_args.extend(rest.iter().map(|s| s.to_string()));
        } else if cmd == "find" || cmd == "grep" {
            // find <filename> [name], grep <pattern> [name]
            // Name comes last
            proc_args.extend(rest.iter().map(|s| s.to_string()));
            proc_args.push(self.palin.name.clone());
        } else {
            // Everything else: just append rest as-is
            // (e.g. init, rm-palin, rename — these have required name args)
            proc_args.extend(rest.iter().map(|s| s.to_string()));
        }

        // Run the subprocess
        let result = std::process::Command::new(&exe_path)
            .args(&proc_args)
            .output();

        match result {
            Ok(output) => {
                if !output.stdout.is_empty() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    for line in text.lines() {
                        self.add_output(line);
                    }
                }
                if !output.stderr.is_empty() {
                    let text = String::from_utf8_lossy(&output.stderr);
                    for line in text.lines() {
                        self.add_output(&format!("✖ {}", line));
                    }
                }
                if !output.status.success() {
                    self.add_output(&format!(
                        "✖ Command exited with code {:?}",
                        output.status.code()
                    ));
                }
            }
            Err(e) => {
                self.add_output(&format!("✖ Failed to execute: {}", e));
            }
        }

        // Exit command mode and reload timeline state
        self.command_mode = false;
        let _ = self.reload();
    }

    fn run_internal(&mut self, _cmd: &str, _args: &[&str]) {
        let registry = match storage::read_registry() {
            Ok(r) => r,
            Err(e) => {
                self.add_output(&format!("✖ {}", e));
                return;
            }
        };

        if registry.palins.is_empty() {
            self.add_output("No palins found. Use `palin init <name>` to create one.");
            return;
        }

        self.add_output("✦ Registered palins:");
        let mut palins: Vec<_> = registry.palins.iter().collect();
        palins.sort_by(|a, b| a.0.cmp(b.0));
        for (name, entry) in &palins {
            let p = std::path::Path::new(&entry.path);
            let status = if p.exists() { "" } else { "  ⚠ missing" };
            self.add_output(&format!("  {:<20}  {}{}", name, entry.path, status));
        }
        self.command_mode = false;
    }

    // ── Output helpers ───────────────────────────────────────────

    pub fn add_output(&mut self, line: &str) {
        self.command_output.insert(0, line.to_string());
        if self.command_output.len() > 100 {
            self.command_output.pop();
        }
    }

    // ── Help overlay input ────────────────────────────────────────

    fn handle_help_input(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(100))? {
            return Ok(());
        }
        let ev = event::read()?;
        if let Event::Key(KeyEvent { code, kind, .. }) = ev {
            if kind != KeyEventKind::Press {
                return Ok(());
            }
            if matches!(code, KeyCode::Char('?') | KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                self.show_help = false;
            }
        }
        Ok(())
    }

    // ── Scrolling ─────────────────────────────────────────────────

    fn scroll_down(&mut self) {
        let max = self.timeline_len().saturating_sub(1);
        if self.selected_idx < max {
            self.selected_idx += 1;
            let _ = self.load_entries();
        }
    }

    fn scroll_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
            let _ = self.load_entries();
        }
    }

    // ── Convenience accessors ─────────────────────────────────────

    pub fn status_for(&self, entry: &FileEntry) -> &'static str {
        match entry.status {
            FileStatus::Added => "+",
            FileStatus::Modified => "~",
            FileStatus::Deleted => "-",
            FileStatus::Unchanged => " ",
        }
    }

    pub fn status_color(&self, entry: &FileEntry) -> ratatui::style::Style {
        match entry.status {
            FileStatus::Added => crate::tui::theme::Theme::added(),
            FileStatus::Modified => crate::tui::theme::Theme::modified(),
            FileStatus::Deleted => crate::tui::theme::Theme::deleted(),
            FileStatus::Unchanged => crate::tui::theme::Theme::dim(),
        }
    }
}

/// A single item in the combined timeline (epoch or phantom).
#[derive(Debug, Clone, Copy)]
pub enum TimelineItem<'a> {
    Epoch(&'a Epoch),
    Phantom(&'a Phantom),
}

// ── Argument parsing ───────────────────────────────────────────

/// Split a command string into arguments, respecting double-quoted strings.
fn parse_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

// ─── Terminal initialisation / shutdown ─────────────────────────

type Tui = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;

fn startup() -> Result<Tui> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

fn shutdown(terminal: &mut Tui) -> Result<()> {
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}
