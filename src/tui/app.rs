use crate::storage;
use crate::types::{Epoch, FileEntry, FileStatus, Phantom, ResolvedPalin};
use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::time::Duration;

/// Which panel is currently focused for keyboard navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The timeline list (left panel)
    Timeline,
    /// The file tree (right panel)
    Files,
}

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

    /// Tree nodes built from current_entries (hierarchical)
    pub tree_nodes: Vec<FileTreeNode>,

    /// Flat list of visible tree rows (files + expanded folder children)
    pub visible_rows: Vec<VisibleRow>,

    /// Set of full directory paths that are expanded
    pub expanded_dirs: HashSet<String>,

    /// Whether the help overlay is visible
    pub show_help: bool,

    /// Whether the app is still running
    pub running: bool,

    /// Terminal size cache for layout
    pub term_area: Rect,

    /// Bounding rect of the Snap button (updated each frame)
    pub snap_button_area: Rect,

    /// Which panel has keyboard focus
    pub focus: Focus,

    // ── Pending operations (run after next draw) ─────────────

    /// If true, run snap on the next event-loop iteration
    pub pending_snap: bool,

    // ── Output ────────────────────────────────────────────────

    /// Recent command output lines (newest first)
    pub command_output: Vec<String>,
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
            tree_nodes: Vec::new(),
            visible_rows: Vec::new(),
            expanded_dirs: HashSet::new(),
            show_help: false,
            running: true,
            term_area: Rect::default(),
            snap_button_area: Rect::default(),
            focus: Focus::Timeline,
            pending_snap: false,
            command_output: Vec::new(),
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
            self.tree_nodes.clear();
            self.visible_rows.clear();
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
        self.expanded_dirs.clear();
        self.build_tree();
        self.flatten_tree();
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

                if self.pending_snap {
                    self.pending_snap = false;
                    let name = self.palin.name.clone();
                    self.run_cmd(&["snap", &name]);
                    let _ = self.reload();
                }

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

    // ── Main input handler (keyboard + mouse) ─────────────────────

    fn handle_main_input(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(100))? {
            return Ok(());
        }
        let ev = event::read()?;
        match ev {
            Event::Key(KeyEvent { code, kind, modifiers, .. }) if kind == KeyEventKind::Press => {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.running = false;
                    }
                    KeyCode::Char('?') => {
                        self.show_help = true;
                    }
                    // ── Up/Down: navigate within focused panel ──
                    KeyCode::Char('j') | KeyCode::Down => {
                        match self.focus {
                            Focus::Timeline => self.scroll_down(),
                            Focus::Files => {
                                if self.selected_file_idx + 1 < self.visible_rows.len() {
                                    self.selected_file_idx += 1;
                                }
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        match self.focus {
                            Focus::Timeline => self.scroll_up(),
                            Focus::Files => {
                                if self.selected_file_idx > 0 {
                                    self.selected_file_idx -= 1;
                                }
                            }
                        }
                    }
                    // ── Page Up / Down ──
                    KeyCode::Char('J') | KeyCode::PageDown => {
                        let step = (self.term_area.height as usize).saturating_sub(6).max(5);
                        let new_idx = self.selected_idx.saturating_add(step)
                            .min(self.timeline_len().saturating_sub(1));
                        if new_idx != self.selected_idx {
                            self.selected_idx = new_idx;
                            self.load_entries()?;
                        }
                    }
                    KeyCode::Char('K') | KeyCode::PageUp => {
                        let step = (self.term_area.height as usize).saturating_sub(6).max(5);
                        let new_idx = self.selected_idx.saturating_sub(step);
                        if new_idx != self.selected_idx {
                            self.selected_idx = new_idx;
                            self.load_entries()?;
                        }
                    }
                    // ── Go to first/last ──
                    KeyCode::Char('g') if modifiers == KeyModifiers::NONE => {
                        self.selected_idx = 0;
                        self.load_entries()?;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        self.selected_idx = self.timeline_len().saturating_sub(1);
                        self.load_entries()?;
                    }
                    // ── Left/Right: switch focus between panels ──
                    KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab => {
                        self.focus = Focus::Timeline;
                    }
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                        self.focus = Focus::Files;
                    }
                    // ── Enter/Space: toggle folder expand (Files only) ──
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if self.focus == Focus::Files {
                            self.toggle_current_folder();
                        }
                    }
                    KeyCode::Char('r') => {
                        self.reload()?;
                    }
                    KeyCode::Char('s') => {
                        self.run_snap();
                    }
                    _ => {}
                }
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                ..
            }) => {
                let btn = self.snap_button_area;
                if column >= btn.x
                    && column < btn.x + btn.width
                    && row >= btn.y
                    && row < btn.y + btn.height
                {
                    self.run_snap();
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ── Snap command ─────────────────────────────────────────────

    pub fn run_snap(&mut self) {
        self.add_output("● Scanning files...");
        self.pending_snap = true;
    }

    // ── Generic command runner (spawns subprocess) ────────────────

    fn run_cmd(&mut self, args: &[&str]) {
        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                self.add_output(&format!("✖ Error: {}", e));
                return;
            }
        };

        let result = std::process::Command::new(&exe_path).args(args).output();

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

    #[allow(dead_code)]
    pub fn status_for(&self, entry: &FileEntry) -> &'static str {
        match entry.status {
            FileStatus::Added => "+",
            FileStatus::Modified => "~",
            FileStatus::Deleted => "-",
            FileStatus::Unchanged => " ",
        }
    }

    #[allow(dead_code)]
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

// ── File Tree ──────────────────────────────────────────────────

/// A node in the hierarchical file tree.
#[derive(Debug, Clone)]
pub struct FileTreeNode {
    pub name: String,
    /// Full path from the tracked root
    pub full_path: String,
    pub is_dir: bool,
    pub children: Vec<FileTreeNode>,
    /// File status (only for files, None for directories)
    pub status: Option<FileStatus>,
    pub file_size: Option<i64>,
}

/// A flattened visible row in the rendered tree.
#[derive(Debug, Clone)]
pub struct VisibleRow {
    pub full_path: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
    pub status: Option<FileStatus>,
    pub file_size: Option<i64>,
    pub name: String,
}

impl App {
    /// Build a tree from the flat current_entries list.
    pub fn build_tree(&mut self) {
        let mut entries = self.current_entries.clone();
        entries.sort_by(|a, b| a.file_path.cmp(&b.file_path));

        let mut root = FileTreeNode {
            name: String::new(),
            full_path: String::new(),
            is_dir: true,
            children: Vec::new(),
            status: None,
            file_size: None,
        };

        for entry in &entries {
            let parts: Vec<&str> = entry.file_path.split('/').collect();
            Self::insert_path(&mut root, parts.as_slice(), entry, "");
        }

        fn sort_children(node: &mut FileTreeNode) {
            node.children.sort_by(|a, b| {
                if a.is_dir != b.is_dir {
                    b.is_dir.cmp(&a.is_dir)
                } else {
                    a.name.cmp(&b.name)
                }
            });
            for child in &mut node.children {
                sort_children(child);
            }
        }
        sort_children(&mut root);

        self.tree_nodes = root.children;
    }

    fn insert_path(parent: &mut FileTreeNode, parts: &[&str], entry: &FileEntry, parent_path: &str) {
        if parts.is_empty() {
            return;
        }
        let name = parts[0];
        let current_path = if parent_path.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", parent_path, name)
        };

        if parts.len() == 1 {
            parent.children.push(FileTreeNode {
                name: name.to_string(),
                full_path: entry.file_path.clone(),
                is_dir: false,
                children: Vec::new(),
                status: Some(entry.status),
                file_size: entry.file_size,
            });
        } else {
            if let Some(existing) = parent.children.iter_mut().find(|c| c.name == name && c.is_dir) {
                Self::insert_path(existing, &parts[1..], entry, &current_path);
            } else {
                let mut dir = FileTreeNode {
                    name: name.to_string(),
                    full_path: current_path.clone(),
                    is_dir: true,
                    children: Vec::new(),
                    status: None,
                    file_size: None,
                };
                Self::insert_path(&mut dir, &parts[1..], entry, &current_path);
                parent.children.push(dir);
            }
        }
    }

    /// Flatten the tree into a list of visible rows based on expanded state.
    pub fn flatten_tree(&mut self) {
        let mut rows = Vec::new();
        for node in &self.tree_nodes {
            Self::flatten_node(node, 0, &self.expanded_dirs, &mut rows);
        }
        self.visible_rows = rows;
        // Clamp selection to valid range
        self.selected_file_idx = self
            .selected_file_idx
            .min(self.visible_rows.len().saturating_sub(1));
    }

    fn flatten_node(
        node: &FileTreeNode,
        depth: usize,
        expanded: &HashSet<String>,
        rows: &mut Vec<VisibleRow>,
    ) {
        let is_expanded = expanded.contains(&node.full_path);
        rows.push(VisibleRow {
            full_path: node.full_path.clone(),
            is_dir: node.is_dir,
            depth,
            is_expanded,
            status: node.status,
            file_size: node.file_size,
            name: node.name.clone(),
        });

        if node.is_dir && is_expanded {
            for child in &node.children {
                Self::flatten_node(child, depth + 1, expanded, rows);
            }
        }
    }

    // ── Expand / Collapse ────────────────────────────────────────

    pub fn toggle_current_folder(&mut self) {
        if let Some(row) = self.visible_rows.get(self.selected_file_idx) {
            if row.is_dir {
                let path = &row.full_path;
                if self.expanded_dirs.contains(path) {
                    self.expanded_dirs.remove(path);
                } else {
                    self.expanded_dirs.insert(path.clone());
                }
                self.flatten_tree();
            }
        }
    }

    pub fn expand_current_folder(&mut self) -> bool {
        if let Some(row) = self.visible_rows.get(self.selected_file_idx) {
            if row.is_dir && !row.is_expanded {
                self.expanded_dirs.insert(row.full_path.clone());
                self.flatten_tree();
                return true;
            }
        }
        false
    }

    pub fn collapse_current_or_up(&mut self) -> bool {
        if let Some(row) = self.visible_rows.get(self.selected_file_idx) {
            if row.is_dir && row.is_expanded {
                self.expanded_dirs.remove(&row.full_path);
                self.flatten_tree();
                return true;
            }
            let path = &row.full_path;
            if let Some(parent) = Self::find_parent_dir(path) {
                if let Some(idx) = self.visible_rows.iter().position(|r| r.full_path == parent) {
                    self.selected_file_idx = idx;
                    return true;
                }
            }
        }
        false
    }

    fn find_parent_dir(path: &str) -> Option<String> {
        if let Some(pos) = path.rfind('/') {
            Some(path[..pos].to_string())
        } else {
            None
        }
    }

    // ── Status accessors ─────────────────────────────────────────

    pub fn status_for_entry(&self, row: &VisibleRow) -> &'static str {
        match row.status {
            Some(FileStatus::Added) => "+",
            Some(FileStatus::Modified) => "~",
            Some(FileStatus::Deleted) => "-",
            Some(FileStatus::Unchanged) => " ",
            None => " ",
        }
    }

    pub fn status_color_for(&self, row: &VisibleRow) -> ratatui::style::Style {
        match row.status {
            Some(FileStatus::Added) => crate::tui::theme::Theme::added(),
            Some(FileStatus::Modified) => crate::tui::theme::Theme::modified(),
            Some(FileStatus::Deleted) => crate::tui::theme::Theme::deleted(),
            Some(FileStatus::Unchanged) => crate::tui::theme::Theme::dim(),
            None => crate::tui::theme::Theme::accent(),
        }
    }
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
