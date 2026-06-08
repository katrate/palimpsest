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
    Timeline,
    Files,
    Search,
}

/// A line in a diff view
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    Same,
    Added,
    Deleted,
    Replaced,
}

/// Kind of action triggered by a timeline button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Restore,
    Delete,
}

/// A confirmed action waiting to be executed.
#[derive(Debug, Clone)]
pub struct PendingAction {
    pub kind: ActionKind,
    /// Display name e.g. "origin", "epoch-3", "phantom-1"
    pub item_name: String,
    /// Index into the timeline items
    pub item_idx: usize,
}

/// Main application state for the TUI.
pub struct App {
    pub palin: ResolvedPalin,
    pub epochs: Vec<Epoch>,
    pub phantoms: Vec<Phantom>,
    pub selected_idx: usize,
    pub selected_file_idx: usize,
    pub current_entries: Vec<FileEntry>,
    pub tree_nodes: Vec<FileTreeNode>,
    pub visible_rows: Vec<VisibleRow>,
    pub expanded_dirs: HashSet<String>,
    pub show_help: bool,
    pub running: bool,
    pub term_area: Rect,
    pub snap_button_area: Rect,
    pub focus: Focus,

    /// Tracking area for the timeline list (for mouse click detection)
    pub timeline_list_area: Rect,
    /// Scroll offset of the timeline (items before this are scrolled up)
    pub timeline_scroll: usize,
    /// Confirmation yes/no button areas in the output bar
    pub confirm_yes_area: Rect,
    pub confirm_no_area: Rect,
    /// A pending action waiting for user confirmation
    pub pending_action: Option<PendingAction>,
    /// A confirmed action waiting to be executed after the next frame draw
    pub pending_execute_action: Option<PendingAction>,

    pub pending_snap: bool,
    pub command_output: Vec<String>,

    // ── Search bar ──
    pub search_query: String,
    pub search_cursor: usize,
    pub search_bar_area: Rect,

    // ── File compare / diff ──
    pub compare_mode: bool,
    pub compare_file_path: String,
    pub compare_source_idx: usize,
    pub compare_result_lines: Vec<DiffLine>,
    pub compare_error: Option<String>,
    pub compare_pending_epoch: bool,
    pub diff_scroll: usize,

    // ── Detail panel area for click detection ──
    pub detail_list_area: Rect,
}

impl App {
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
            timeline_list_area: Rect::default(),
            timeline_scroll: 0,
            confirm_yes_area: Rect::default(),
            confirm_no_area: Rect::default(),
            pending_action: None,
            pending_execute_action: None,
            pending_snap: false,
            command_output: Vec::new(),
            search_query: String::new(),
            search_cursor: 0,
            search_bar_area: Rect::default(),

            compare_mode: false,
            compare_file_path: String::new(),
            compare_source_idx: 0,
            compare_result_lines: Vec::new(),
            compare_error: None,
            compare_pending_epoch: false,
            diff_scroll: 0,
            detail_list_area: Rect::default(),
        };
        app.load_entries()?;
        Ok(app)
    }

    pub fn reload(&mut self) -> Result<()> {
        let conn = storage::open_db(&self.palin.name)?;
        self.epochs = storage::list_epochs(&conn)?;
        self.phantoms = storage::list_phantoms(&conn)?;
        self.selected_idx = self.selected_idx.min(self.timeline_len().saturating_sub(1));
        self.load_entries()?;
        Ok(())
    }

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

    pub fn timeline_len(&self) -> usize {
        self.epochs.len() + self.phantoms.len()
    }

    pub fn load_entries(&mut self) -> Result<()> {
        // Clear diff view when loading a different epoch
        self.compare_result_lines.clear();
        self.compare_error = None;
        self.compare_mode = false;
        self.diff_scroll = 0;

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

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = startup()?;

        let result = (|| -> Result<()> {
            while self.running {
                terminal.draw(|frame| {
                    self.term_area = frame.area();
                    crate::tui::ui::render(self, frame);
                })?;

                // Run pending snap (after draw so message is visible first)
                if self.pending_snap {
                    self.pending_snap = false;
                    let name = self.palin.name.clone();
                    self.run_cmd(&["snap", &name]);
                    let _ = self.reload();
                }

                // Run pending timeline action (after draw so message is visible first)
                if let Some(action) = self.pending_execute_action.take() {
                    let name = self.palin.name.clone();
                    self.run_action_inner(&action, &name);
                    let _ = self.reload();
                }

                // Run pending compare (after draw so message is visible first)
                if self.compare_pending_epoch {
                    self.compare_pending_epoch = false;
                    self.run_compare();
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

    // ── Main input handler ──────────────────────────────────

    fn handle_main_input(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(100))? {
            return Ok(());
        }
        let ev = event::read()?;
        match ev {
            Event::Key(KeyEvent { code, kind, .. }) if kind == KeyEventKind::Press => {
                // If search bar is focused, handle search input
                if self.focus == Focus::Search {
                    return self.handle_search_input(code);
                }

                // If there's a pending confirmation, handle y/n
                if self.pending_action.is_some() {
                    match code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            self.execute_pending_action();
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            self.cancel_pending_action();
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                match code {
                    KeyCode::Char('q') => {
                        self.running = false;
                    }
                    KeyCode::Esc => {
                        // Clear diff view if active, otherwise quit
                        if !self.compare_result_lines.is_empty() || self.compare_error.is_some() || self.compare_mode {
                            self.compare_result_lines.clear();
                            self.compare_error = None;
                            self.compare_mode = false;
                            self.diff_scroll = 0;
                        } else {
                            self.running = false;
                        }
                    }
                    KeyCode::Char('?') => {
                        self.show_help = true;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        // If viewing a diff result, scroll through it
                        if !self.compare_result_lines.is_empty() {
                            let max = self.compare_result_lines.len().saturating_sub(1);
                            if self.diff_scroll < max {
                                self.diff_scroll += 1;
                            }
                        } else {
                            match self.focus {
                                Focus::Timeline => self.scroll_down(),
                                Focus::Files => {
                                    if self.selected_file_idx + 1 < self.visible_rows.len() {
                                        self.selected_file_idx += 1;
                                    }
                                }
                                Focus::Search => {}
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        // If viewing a diff result, scroll through it
                        if !self.compare_result_lines.is_empty() {
                            if self.diff_scroll > 0 {
                                self.diff_scroll -= 1;
                            }
                        } else {
                            match self.focus {
                                Focus::Timeline => self.scroll_up(),
                                Focus::Files => {
                                    if self.selected_file_idx > 0 {
                                        self.selected_file_idx -= 1;
                                    }
                                }
                                Focus::Search => {}
                            }
                        }
                    }
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
                    KeyCode::Char('g') => {
                        self.selected_idx = 0;
                        self.load_entries()?;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        self.selected_idx = self.timeline_len().saturating_sub(1);
                        self.load_entries()?;
                    }
                    KeyCode::Char('/') => {
                        self.clear_search();
                        self.focus = Focus::Search;
                    }
                    KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab => {
                        self.focus = Focus::Timeline;
                    }
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                        self.focus = Focus::Files;
                    }
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
                kind: MouseEventKind::Down(button),
                column,
                row,
                modifiers,
                ..
            }) => {
                let shift = modifiers.contains(KeyModifiers::SHIFT);

                // 1. Check Snap button
                let btn = self.snap_button_area;
                if column >= btn.x && column < btn.x + btn.width
                    && row >= btn.y && row < btn.y + btn.height
                {
                    self.run_snap();
                    return Ok(());
                }

                // 2. Check confirmation yes/no buttons
                if self.pending_action.is_some() {
                    let y = self.confirm_yes_area;
                    let n = self.confirm_no_area;
                    if column >= y.x && column < y.x + y.width
                        && row >= y.y && row < y.y + y.height
                    {
                        if button == MouseButton::Left {
                            self.execute_pending_action();
                        }
                        return Ok(());
                    }
                    if column >= n.x && column < n.x + n.width
                        && row >= n.y && row < n.y + n.height
                    {
                        if button == MouseButton::Left {
                            self.cancel_pending_action();
                        }
                        return Ok(());
                    }
                }

                // 3. Check search bar area
                let sb = self.search_bar_area;
                if sb.width > 0 && column >= sb.x && column < sb.x + sb.width
                    && row >= sb.y && row < sb.y + sb.height
                {
                    self.focus = Focus::Search;
                    return Ok(());
                }

                // 4. Check detail list area for file click (enter compare mode)
                let dl = self.detail_list_area;
                if dl.width > 0 && column >= dl.x && column < dl.x + dl.width
                    && row >= dl.y && row < dl.y + dl.height
                {
                    // Clear diff view on click
                    self.compare_result_lines.clear();
                    self.compare_error = None;
                    let filtered = self.filtered_visible_rows();
                    let vis_idx = (row - dl.y) as usize;
                    if vis_idx < filtered.len() {
                        let clicked_row = filtered[vis_idx];
                        if !clicked_row.is_dir {
                            // Find the original index in visible_rows
                            if let Some(orig_idx) = self.visible_rows.iter().position(|r| r.full_path == clicked_row.full_path) {
                                self.selected_file_idx = orig_idx;
                                self.enter_compare_mode(orig_idx);
                            }
                            return Ok(());
                        }
                    }
                }

                // 6. Check timeline list area for selection / action buttons
                let tl = self.timeline_list_area;
                if tl.width > 0 && column >= tl.x && column < tl.x + tl.width
                    && row >= tl.y && row < tl.y + tl.height
                {
                    let vis_idx = (row - tl.y) as usize;
                    let raw_idx = vis_idx + self.timeline_scroll;
                    let epoch_count = self.epochs.len();
                    let has_header = epoch_count > 0 && self.phantoms.len() > 0;
                    // Compute item index accounting for section headers
                    let item_idx = if has_header {
                        if self.timeline_scroll == 0 {
                            // Both Snapshots and Phantoms headers visible
                            if raw_idx == 0 {
                                return Ok(()); // Snapshots header
                            }
                            if raw_idx == epoch_count + 1 {
                                return Ok(()); // Phantoms header
                            }
                            if raw_idx <= epoch_count {
                                raw_idx - 1 // epoch item
                            } else {
                                raw_idx - 2 // phantom item (offset by both headers)
                            }
                        } else {
                            // Only Phantoms header visible (scrolled past Snapshots)
                            if raw_idx == epoch_count {
                                return Ok(()); // Phantoms header
                            }
                            if raw_idx < epoch_count {
                                raw_idx // epoch item
                            } else {
                                raw_idx - 1 // phantom item (offset by Phantoms header)
                            }
                        }
                    } else {
                        raw_idx
                    };
                    if item_idx < self.timeline_len() {
                        // Check if click is on the right-side action buttons
                        let btn_region_x = tl.x + tl.width.saturating_sub(9);
                        if column >= btn_region_x && button == MouseButton::Left {
                            let btn_offset = column - btn_region_x;
                            if btn_offset < 4 {
                                self.initiate_action(ActionKind::Restore, item_idx, shift);
                            } else {
                                self.initiate_action(ActionKind::Delete, item_idx, shift);
                            }
                            return Ok(());
                        }
                        // Regular click — select the item
                        // If we were waiting for a compare target, run compare
                        if self.compare_mode {
                            self.compare_source_idx = item_idx;
                            self.compare_pending_epoch = true;
                            self.compare_mode = false;
                            self.add_output(&format!("● Comparing..."));
                            return Ok(());
                        }
                        self.selected_idx = item_idx;
                        self.load_entries()?;
                        return Ok(());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ── Action initiation / confirmation ─────────────────────

    fn initiate_action(&mut self, kind: ActionKind, item_idx: usize, auto_confirm: bool) {
        let items = self.timeline_items();
        let item_name = match items.get(item_idx) {
            Some(TimelineItem::Epoch(e)) => e.display_name(),
            Some(TimelineItem::Phantom(p)) => p.display_name(),
            None => return,
        };

        if auto_confirm {
            // Shift+click — show message now, execute after next frame draw
            self.add_output(&format!("● {} {}...", match kind {
                ActionKind::Restore => "Restoring to",
                ActionKind::Delete => "Deleting",
            }, item_name));
            self.pending_execute_action = Some(PendingAction {
                kind,
                item_name: item_name.clone(),
                item_idx,
            });
        } else {
            // Show confirmation
            self.pending_action = Some(PendingAction {
                kind,
                item_name: item_name.clone(),
                item_idx,
            });
            self.add_output(&format!("  {} {}?  [y/N]", match kind {
                ActionKind::Restore => "Restore to",
                ActionKind::Delete => "Delete",
            }, item_name));
        }
    }

    fn execute_pending_action(&mut self) {
        // Move from pending_action to pending_execute_action so the message
        // renders before the blocking subprocess runs
        if let Some(action) = self.pending_action.take() {
            self.add_output(&format!("● {} {}...", match action.kind {
                ActionKind::Restore => "Restoring to",
                ActionKind::Delete => "Deleting",
            }, action.item_name));
            self.pending_execute_action = Some(action);
        }
    }

    fn cancel_pending_action(&mut self) {
        if let Some(action) = self.pending_action.take() {
            self.add_output(&format!("✖ Cancelled {} of {}", match action.kind {
                ActionKind::Restore => "restore",
                ActionKind::Delete => "deletion",
            }, action.item_name));
        }
    }

    fn run_action_inner(&mut self, action: &PendingAction, name: &str) {
        let items = self.timeline_items();
        match items.get(action.item_idx) {
            Some(TimelineItem::Epoch(epoch)) => {
                match action.kind {
                    ActionKind::Restore => {
                        let epoch_id = epoch.display_name();
                        self.run_cmd(&["restore", &epoch_id, &name, "-y"]);
                    }
                    ActionKind::Delete => {
                        if epoch.is_origin {
                            self.run_cmd(&["rm-origin", &name]);
                        } else {
                            let num = epoch.epoch_num.to_string();
                            self.run_cmd(&["rm-epoch", &name, &num]);
                        }
                    }
                }
            }
            Some(TimelineItem::Phantom(phantom)) => {
                match action.kind {
                    ActionKind::Restore => {
                        let phantom_id = phantom.display_name();
                        self.run_cmd(&["restore", &phantom_id, &name, "-y"]);
                    }
                    ActionKind::Delete => {
                        let num = phantom.phantom_num.to_string();
                        self.run_cmd(&["rm-phantom", &name, &num]);
                    }
                }
            }
            None => {}
        }
    }

    // ── Snap ────────────────────────────────────────────────

    pub fn run_snap(&mut self) {
        self.add_output("● Scanning files...");
        self.pending_snap = true;
    }

    // ── Command runner ────────────────────────────────────

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

    pub fn add_output(&mut self, line: &str) {
        self.command_output.insert(0, line.to_string());
        if self.command_output.len() > 100 {
            self.command_output.pop();
        }
    }

    // ── Search bar ─────────────────────────────────────

    fn handle_search_input(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Esc => {
                self.clear_search();
                self.focus = Focus::Timeline;
            }
            KeyCode::Enter => {
                self.focus = Focus::Files;
            }
            KeyCode::Char(c) => {
                self.search_query.insert(self.search_cursor, c);
                self.search_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    self.search_query.remove(self.search_cursor);
                }
            }
            KeyCode::Delete => {
                if self.search_cursor < self.search_query.len() {
                    self.search_query.remove(self.search_cursor);
                }
            }
            KeyCode::Left => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.search_cursor < self.search_query.len() {
                    self.search_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.search_cursor = 0;
            }
            KeyCode::End => {
                self.search_cursor = self.search_query.len();
            }
            KeyCode::Tab | KeyCode::Down => {
                self.focus = Focus::Files;
            }
            KeyCode::Up => {
                self.focus = Focus::Timeline;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_cursor = 0;
    }

    /// Get the filtered visible rows based on search query
    pub fn filtered_visible_rows(&self) -> Vec<&VisibleRow> {
        if self.search_query.is_empty() {
            return self.visible_rows.iter().collect();
        }

        let query = self.search_query.to_lowercase();
        self.visible_rows.iter().filter(|row| {
            row.full_path.to_lowercase().contains(&query)
                || row.name.to_lowercase().contains(&query)
        }).collect()
    }

    // ── File compare ──────────────────────────────────────

    pub fn enter_compare_mode(&mut self, file_idx: usize) {
        if let Some(row) = self.visible_rows.get(file_idx) {
            if !row.is_dir {
                self.compare_mode = true;
                self.compare_file_path = row.full_path.clone();
                self.compare_result_lines.clear();
                self.compare_error = None;
                self.add_output(&format!(
                    "  Select an epoch to compare '{}' with...",
                    row.full_path
                ));
            }
        }
    }

    fn run_compare(&mut self) {
        let file_path = self.compare_file_path.clone();
        let name = self.palin.name.clone();
        let selected_idx = self.selected_idx;
        let compare_source_idx = self.compare_source_idx;

        // --- Gather all data first (while self is immutably borrowed) ---

        // Build display names data upfront
        let conn = match storage::open_db(&name) {
            Ok(c) => c,
            Err(e) => {
                self.compare_error = Some(format!("DB error: {}", e));
                return;
            }
        };

        // Resolve source and target items from storage directly
        let (source_id, source_type_str, source_display) = self.resolve_item_info(selected_idx);
        let (target_id, target_type_str, target_display) = self.resolve_item_info(compare_source_idx);

        if source_id.is_none() || target_id.is_none() {
            self.compare_error = Some("Source or target not found.".to_string());
            return;
        }
        let source_id = source_id.unwrap();
        let target_id = target_id.unwrap();
        let source_type = source_type_str.unwrap();
        let target_type = target_type_str.unwrap();
        let source_display = source_display.unwrap();
        let target_display = target_display.unwrap();

        let source_entries = match storage::get_file_entries(&conn, source_id, &source_type) {
            Ok(e) => e,
            Err(_) => {
                self.compare_error = Some("Failed to read source entries.".to_string());
                return;
            }
        };
        let target_entries = match storage::get_file_entries(&conn, target_id, &target_type) {
            Ok(e) => e,
            Err(_) => {
                self.compare_error = Some("Failed to read target entries.".to_string());
                return;
            }
        };

        let old_entry = source_entries.iter().find(|e| e.file_path == file_path);
        let new_entry = target_entries.iter().find(|e| e.file_path == file_path);

        // Read content from both sides
        let old_lines = Self::read_file_content(&name, old_entry);
        let new_lines = Self::read_file_content(&name, new_entry);

        // --- Now we can mutate self freely ---
        self.compare_result_lines.clear();
        self.compare_error = None;
        self.diff_scroll = 0;

        // Check if file exists in target
        if new_entry.is_none() {
            self.compare_error = Some("File not available to compare".to_string());
            return;
        }

        if old_entry.is_none() && !new_lines.is_empty() {
            for line in &new_lines {
                self.compare_result_lines.push(DiffLine {
                    line_type: DiffLineType::Added,
                    content: line.clone(),
                });
            }
            self.add_output(&format!(
                "✦ Diff: {} vs {} — {} lines (new file)",
                source_display, target_display, new_lines.len()
            ));
            return;
        }

        if old_entry.is_some() && new_entry.is_none() {
            for line in &old_lines {
                self.compare_result_lines.push(DiffLine {
                    line_type: DiffLineType::Deleted,
                    content: line.clone(),
                });
            }
            self.add_output(&format!(
                "✦ Diff: {} vs {} — {} lines (deleted)",
                source_display, target_display, old_lines.len()
            ));
            return;
        }

        // Use similar crate with stable lifetimes
        use similar::{ChangeTag, DiffOp, TextDiff};
        let old_joined = old_lines.join("\n");
        let new_joined = new_lines.join("\n");
        let diff = TextDiff::from_lines(&old_joined, &new_joined);

        let mut added_count = 0;
        let mut deleted_count = 0;
        let mut modified_count = 0;
        let mut result_lines: Vec<DiffLine> = Vec::new();

        for op in diff.ops() {
            match op {
                DiffOp::Equal { .. } => {
                    for change in diff.iter_changes(op) {
                        let line = change.value().trim_end_matches('\n').to_string();
                        result_lines.push(DiffLine {
                            line_type: DiffLineType::Same,
                            content: line,
                        });
                    }
                }
                DiffOp::Insert { .. } => {
                    for change in diff.iter_changes(op) {
                        if change.tag() == ChangeTag::Insert {
                            let line = change.value().trim_end_matches('\n').to_string();
                            added_count += 1;
                            result_lines.push(DiffLine {
                                line_type: DiffLineType::Added,
                                content: line,
                            });
                        }
                    }
                }
                DiffOp::Delete { .. } => {
                    for change in diff.iter_changes(op) {
                        if change.tag() == ChangeTag::Delete {
                            let line = change.value().trim_end_matches('\n').to_string();
                            deleted_count += 1;
                            result_lines.push(DiffLine {
                                line_type: DiffLineType::Deleted,
                                content: line,
                            });
                        }
                    }
                }
                DiffOp::Replace { .. } => {
                    for change in diff.iter_changes(op) {
                        let line = change.value().trim_end_matches('\n').to_string();
                        match change.tag() {
                            ChangeTag::Delete => {
                                modified_count += 1;
                                result_lines.push(DiffLine {
                                    line_type: DiffLineType::Replaced,
                                    content: line,
                                });
                            }
                            ChangeTag::Insert => {
                                added_count += 1;
                                result_lines.push(DiffLine {
                                    line_type: DiffLineType::Added,
                                    content: line,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Store results (diff is dropped here, old_joined/new_joined are freed)
        self.compare_result_lines = result_lines;

        self.add_output(&format!(
            "✦ Diff: {} vs {} — {} lines (+{} / ~{} / -{})",
            source_display, target_display,
            self.compare_result_lines.len(),
            added_count, modified_count, deleted_count
        ));
    }

    /// Resolve a timeline item to (id, type, display_name) from storage
    fn resolve_item_info(&self, idx: usize) -> (Option<i64>, Option<String>, Option<String>) {
        if idx < self.epochs.len() {
            let e = &self.epochs[idx];
            (Some(e.id), Some("epoch".to_string()), Some(e.display_name()))
        } else {
            let pidx = idx - self.epochs.len();
            if pidx < self.phantoms.len() {
                let p = &self.phantoms[pidx];
                (Some(p.id), Some("phantom".to_string()), Some(p.display_name()))
            } else {
                (None, None, None)
            }
        }
    }

    /// Read file content from ink storage
    fn read_file_content(palin_name: &str, entry: Option<&FileEntry>) -> Vec<String> {
        match entry.and_then(|e| e.ink_hash.as_ref()) {
            Some(hash) => {
                match storage::ink::read_ink(palin_name, hash) {
                    Ok(data) => {
                        let text = String::from_utf8_lossy(&data);
                        text.lines().map(|l| l.to_string()).collect()
                    }
                    Err(_) => vec!["<<error reading>>".to_string()],
                }
            }
            None => vec![],
        }
    }

    // ── Folder status propagation ─────────────────────────

    /// Propagate the most significant child status up to directories.
    /// Priority: Added > Modified > Deleted > Unchanged > None
    pub fn propagate_folder_status(node: &mut FileTreeNode) -> Option<FileStatus> {
        let mut agg: Option<FileStatus> = node.status;
        for child in &mut node.children {
            let child_status = Self::propagate_folder_status(child);
            agg = Self::merge_status(agg, child_status);
        }
        if node.is_dir {
            node.status = agg;
        }
        agg
    }

    fn merge_status(a: Option<FileStatus>, b: Option<FileStatus>) -> Option<FileStatus> {
        match (a, b) {
            (Some(FileStatus::Added), _) | (_, Some(FileStatus::Added)) => Some(FileStatus::Added),
            (Some(FileStatus::Modified), _) | (_, Some(FileStatus::Modified)) => Some(FileStatus::Modified),
            (Some(FileStatus::Deleted), _) | (_, Some(FileStatus::Deleted)) => Some(FileStatus::Deleted),
            (Some(FileStatus::Unchanged), _) => Some(FileStatus::Unchanged),
            (_, Some(FileStatus::Unchanged)) => Some(FileStatus::Unchanged),
            (None, None) => None,
        }
    }

    // ── Help ─────────────────────────────────────────────

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

    // ── Scrolling ───────────────────────────────────────

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

    // ── File tree helpers ───────────────────────────────

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

        // Propagate child statuses up to parent directories
        Self::propagate_folder_status(&mut root);

        self.tree_nodes = root.children;
    }

    fn insert_path(parent: &mut FileTreeNode, parts: &[&str], entry: &FileEntry, parent_path: &str) {
        if parts.is_empty() { return; }
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

    pub fn flatten_tree(&mut self) {
        let mut rows = Vec::new();
        for node in &self.tree_nodes {
            Self::flatten_node(node, 0, &self.expanded_dirs, &mut rows);
        }
        self.visible_rows = rows;
        self.selected_file_idx = self.selected_file_idx.min(self.visible_rows.len().saturating_sub(1));
    }

    fn flatten_node(node: &FileTreeNode, depth: usize, expanded: &HashSet<String>, rows: &mut Vec<VisibleRow>) {
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

    // ── Status accessors ────────────────────────────────

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

/// A single item in the combined timeline (epoch or phantom).
#[derive(Debug, Clone, Copy)]
pub enum TimelineItem<'a> {
    Epoch(&'a Epoch),
    Phantom(&'a Phantom),
}

// ── File Tree ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FileTreeNode {
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
    pub children: Vec<FileTreeNode>,
    pub status: Option<FileStatus>,
    pub file_size: Option<i64>,
}

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

// ─── Terminal init / shutdown ────────────────────────

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
