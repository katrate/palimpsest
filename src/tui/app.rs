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
                    // Move focus to file list (or scroll down in file list)
                    if self.current_entries.len() > 1 {
                        self.selected_file_idx = self
                            .selected_file_idx
                            .saturating_add(1)
                            .min(self.current_entries.len().saturating_sub(1));
                    }
                }
                KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => {
                    // Move focus back
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
