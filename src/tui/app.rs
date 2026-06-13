use crate::commands;
use crate::storage;
use crate::types::{Epoch, FileEntry, FileStatus, Phantom, ResolvedPalin};
use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

enum SnapMessage {
    Done(Vec<String>, anyhow::Result<()>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Timeline,
    Files,
    Search,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Restore,
    Delete,
}

#[derive(Debug, Clone)]
pub struct PendingAction {
    pub kind: ActionKind,
    pub item_name: String,
    pub item_idx: usize,
}

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
    pub rename_button_area: Rect,
    pub info_button_area: Rect,
    pub gc_button_area: Rect,
    pub pending_gc: bool,
    pub focus: Focus,
    pub timeline_list_area: Rect,
    pub timeline_scroll: usize,
    pub confirm_yes_area: Rect,
    pub confirm_no_area: Rect,
    pub pending_action: Option<PendingAction>,
    pub pending_execute_action: Option<PendingAction>,
    pub snap_in_progress: bool,
    snap_start_frame: u64,
    patience_shown: bool,
    pub frame_count: u64,
    snap_rx: Option<Receiver<SnapMessage>>,

    pub command_output: Vec<String>,
    pub search_query: String,
    pub search_cursor: usize,
    pub search_bar_area: Rect,
    pub output_scroll: usize,
    pub output_clear_area: Rect,
    pub output_bar_area: Rect,
    pub compare_mode: bool,
    pub compare_file_path: String,
    pub compare_source_idx: usize,
    pub compare_result_lines: Vec<DiffLine>,
    pub compare_error: Option<String>,
    pub compare_pending_epoch: bool,
    pub diff_scroll: usize,
    pub detail_list_area: Rect,
    pub timeline_btn_start_x: u16,
    pub show_picker: bool,
    pub all_palins: Vec<(String, String)>,
    pub picker_selected: usize,
    pub picker_confirm_delete: Option<String>,
    pub picker_confirm_yes_area: Rect,
    pub picker_confirm_no_area: Rect,
    pub preview_content: Vec<String>,
    pub preview_scroll: usize,
    pub show_preview: bool,
    pub rename_mode: bool,
    pub rename_input: String,
    pub rename_cursor: usize,
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
            rename_button_area: Rect::default(),
            info_button_area: Rect::default(),
            gc_button_area: Rect::default(),
            pending_gc: false,
            focus: Focus::Timeline,
            timeline_list_area: Rect::default(),
            timeline_scroll: 0,
            confirm_yes_area: Rect::default(),
            confirm_no_area: Rect::default(),
            pending_action: None,
            pending_execute_action: None,
            snap_in_progress: false,
            snap_start_frame: 0,
            patience_shown: false,
            frame_count: 0,
            snap_rx: None,
            command_output: Vec::new(),
            search_query: String::new(),
            search_cursor: 0,
            search_bar_area: Rect::default(),
            output_scroll: 0,
            output_clear_area: Rect::default(),
            output_bar_area: Rect::default(),

            compare_mode: false,
            compare_file_path: String::new(),
            compare_source_idx: 0,
            compare_result_lines: Vec::new(),
            compare_error: None,
            compare_pending_epoch: false,
            diff_scroll: 0,
            detail_list_area: Rect::default(),
            timeline_btn_start_x: 0,
            show_picker: false,
            all_palins: Vec::new(),
            picker_selected: 0,
            picker_confirm_delete: None,
            picker_confirm_yes_area: Rect::default(),
            picker_confirm_no_area: Rect::default(),
            preview_content: Vec::new(),
            preview_scroll: 0,
            show_preview: false,
            rename_mode: false,
            rename_input: String::new(),
            rename_cursor: 0,
        };
        app.load_entries()?;
        app.load_palins();
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
                self.frame_count += 1;

                // Animate snap spinner and show patience message if slow
                if self.snap_in_progress {
                    let frames = ['◐', '◓', '◑', '◒'];
                    let idx = (self.frame_count / 2) as usize % frames.len();
                    if let Some(first) = self.command_output.first_mut() {
                        if first.contains("Scanning files") {
                            *first = format!("{} Scanning files...", frames[idx]);
                        }
                    }
                    if !self.patience_shown && self.frame_count - self.snap_start_frame > 100 {
                        self.patience_shown = true;
                        self.add_output("  It's taking longer than usual, please be patient...");
                    }
                }

                terminal.draw(|frame| {
                    self.term_area = frame.area();
                    crate::tui::ui::render(self, frame);
                })?;

                // Poll background snap thread for output
                let snap_msgs = match self.snap_rx {
                    Some(ref rx) => {
                        let mut msgs = Vec::new();
                        while let Ok(msg) = rx.try_recv() {
                            msgs.push(msg);
                        }
                        Some(msgs)
                    }
                    None => None,
                };
                if let Some(msgs) = snap_msgs {
                    let mut snap_done = false;
                    for msg in msgs {
                        match msg {
                            SnapMessage::Done(lines, result) => {
                                snap_done = true;
                                self.snap_in_progress = false;
                                match result {
                                    Ok(()) => {
                                        for line in lines.iter().skip(1) {
                                            self.add_output(line);
                                        }
                                        if let Err(e) = self.reload() {
                                            self.add_output(&format!("✖ Reload failed: {}", e));
                                        }
                                    }
                                    Err(e) => {
                                        self.add_output(&format!("✖ Snap failed: {}", e));
                                    }
                                }
                            }
                        }
                    }
                    if snap_done {
                        self.snap_rx = None;
                    }
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

                // If there's a pending GC confirmation, handle y/n
                if self.pending_gc {
                    match code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            self.execute_gc();
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            self.pending_gc = false;
                            self.add_output("✖ GC cancelled.");
                        }
                        _ => {}
                    }
                    return Ok(());
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

                // If picker is open, handle picker keys
                if self.show_picker {
                    match code {
                    KeyCode::Down => {
                        if self.picker_selected + 1 < self.all_palins.len() {
                            self.picker_selected += 1;
                        }
                    }
                    KeyCode::Up => {
                        if self.picker_selected > 0 {
                            self.picker_selected -= 1;
                        }
                    }
                        KeyCode::Enter => {
                            let name = self.all_palins.get(self.picker_selected).map(|(n, _)| n.clone());
                            if let Some(ref n) = name {
                                let _ = self.switch_to_palin(n);
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Delete => {
                            if let Some((name, _)) = self.all_palins.get(self.picker_selected) {
                                if name != &self.palin.name {
                                    self.picker_confirm_delete = Some(name.clone());
                                }
                            }
                        }
                        KeyCode::Esc => {
                            self.show_picker = false;
                            self.picker_confirm_delete = None;
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                // If preview is shown, handle preview keys
                if self.show_preview && !self.preview_content.is_empty() {
                    match code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if self.preview_scroll + 1 < self.preview_content.len() {
                            self.preview_scroll += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.preview_scroll > 0 {
                            self.preview_scroll -= 1;
                        }
                    }
                        KeyCode::Esc => {
                            self.show_preview = false;
                            self.preview_content.clear();
                            self.preview_scroll = 0;
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                // If rename mode is active, handle text input
                if self.rename_mode {
                    match code {
                        KeyCode::Char(c) => {
                            self.rename_input.insert(self.rename_cursor, c);
                            self.rename_cursor += 1;
                        }
                        KeyCode::Backspace => {
                            if self.rename_cursor > 0 {
                                self.rename_cursor -= 1;
                                self.rename_input.remove(self.rename_cursor);
                            }
                        }
                        KeyCode::Delete => {
                            if self.rename_cursor < self.rename_input.len() {
                                self.rename_input.remove(self.rename_cursor);
                            }
                        }
                        KeyCode::Enter => {
                            let _ = self.commit_rename();
                        }
                        KeyCode::Esc => {
                            self.rename_mode = false;
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
                    KeyCode::Char('j') | KeyCode::Char('k') => {
                        // j/k only scroll diff results (not navigation — use arrows for that)
                        if !self.compare_result_lines.is_empty() {
                            if code == KeyCode::Char('j') && self.diff_scroll < self.compare_result_lines.len().saturating_sub(1) {
                                self.diff_scroll += 1;
                            } else if code == KeyCode::Char('k') && self.diff_scroll > 0 {
                                self.diff_scroll -= 1;
                            }
                        }
                    }
                    KeyCode::Down => {
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
                    KeyCode::Up => {
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
                            let was_comp = self.compare_mode;
                            self.load_entries()?;
                            self.compare_mode = was_comp;
                        }
                    }
                    KeyCode::Char('K') | KeyCode::PageUp => {
                        let step = (self.term_area.height as usize).saturating_sub(6).max(5);
                        let new_idx = self.selected_idx.saturating_sub(step);
                        if new_idx != self.selected_idx {
                            self.selected_idx = new_idx;
                            let was_comp = self.compare_mode;
                            self.load_entries()?;
                            self.compare_mode = was_comp;
                        }
                    }
                    KeyCode::Char('g') => {
                        self.selected_idx = 0;
                        let was_comp = self.compare_mode;
                        self.load_entries()?;
                        self.compare_mode = was_comp;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        self.selected_idx = self.timeline_len().saturating_sub(1);
                        let was_comp = self.compare_mode;
                        self.load_entries()?;
                        self.compare_mode = was_comp;
                    }
                    KeyCode::Char('/') => {
                        self.clear_search();
                        self.focus = Focus::Search;
                    }
                    KeyCode::Char('p') => {
                        self.open_picker();
                    }
                    KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab => {
                        self.focus = Focus::Timeline;
                    }
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                        self.focus = Focus::Files;
                    }
                    KeyCode::Char('d') => {
                        if self.compare_mode {
                            // In compare mode — press d to run compare against current epoch
                            // Works regardless of focus (file or timeline)
                            self.compare_pending_epoch = true;
                            self.compare_mode = false;
                            self.add_output("● Comparing...");
                        } else if self.focus == Focus::Files {
                            // Start compare: select a file, switch to timeline
                            let row_path = self.visible_rows.get(self.selected_file_idx).map(|r| r.full_path.clone());
                            let is_dir = self.visible_rows.get(self.selected_file_idx).map(|r| r.is_dir).unwrap_or(false);
                            if let Some(ref path) = row_path {
                                if !is_dir {
                                    // Find original index
                                    if let Some(orig_idx) = self.visible_rows.iter().position(|r| r.full_path == *path) {
                                        self.enter_compare_mode(orig_idx);
                                        self.focus = Focus::Timeline;
                                        self.add_output(&format!("  Select an epoch to compare '{}' with...", path));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if self.focus == Focus::Files {
                            // If on a file (not dir), preview it
                            let row_path = self.visible_rows.get(self.selected_file_idx).map(|r| r.full_path.clone());
                            let is_dir = self.visible_rows.get(self.selected_file_idx).map(|r| r.is_dir).unwrap_or(false);
                            if is_dir {
                                self.toggle_current_folder();
                            } else if !self.show_preview {
                                if let Some(ref path) = row_path {
                                    self.preview_file(path);
                                }
                            }
                        }
                    }
                    KeyCode::Char('r') => {
                        self.reload()?;
                    }
                    KeyCode::Char('i') => {
                        self.run_info();
                    }
                    KeyCode::Char('s') => {
                        self.run_snap();
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        if !self.command_output.is_empty() {
                            let max = self.command_output.len().saturating_sub(1);
                            if self.output_scroll < max {
                                self.output_scroll += 1;
                            }
                        }
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.clear_output();
                    }
                    _ => {}
                }
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column,
                row,
                ..
            }) => {
                // Scroll wheel only works for diff and preview content
                if !self.compare_result_lines.is_empty() {
                    let max = self.compare_result_lines.len().saturating_sub(1);
                    if self.diff_scroll < max {
                        self.diff_scroll += 1;
                    }
                } else if self.show_preview && !self.preview_content.is_empty() {
                    if self.preview_scroll + 1 < self.preview_content.len() {
                        self.preview_scroll += 1;
                    }
                } else {
                    let ob = self.output_bar_area;
                    if ob.height > 0 && column >= ob.x && column < ob.x + ob.width
                        && row >= ob.y && row < ob.y + ob.height
                        && !self.command_output.is_empty()
                    {
                        let max = self.command_output.len().saturating_sub(1);
                        if self.output_scroll < max {
                            self.output_scroll += 1;
                        }
                    }
                }
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column,
                row,
                ..
            }) => {
                // Scroll wheel only works for diff, preview, and output content
                if !self.compare_result_lines.is_empty() {
                    if self.diff_scroll > 0 {
                        self.diff_scroll -= 1;
                    }
                } else if self.show_preview && !self.preview_content.is_empty() {
                    if self.preview_scroll > 0 {
                        self.preview_scroll -= 1;
                    }
                } else {
                    let ob = self.output_bar_area;
                    if ob.height > 0
                        && !self.command_output.is_empty()
                    {
                        if self.output_scroll > 0 {
                            self.output_scroll -= 1;
                        }
                    }
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

                // 0. If picker is open, handle picker clicks
                if self.show_picker {
                    // Check delete confirm buttons
                    if self.picker_confirm_delete.is_some() {
                        let ya = self.picker_confirm_yes_area;
                        let na = self.picker_confirm_no_area;
                        if ya.width > 0 && column >= ya.x && column < ya.x + ya.width
                            && row >= ya.y && row < ya.y + ya.height
                        {
                            let _ = self.delete_current_palin();
                            return Ok(());
                        }
                        if na.width > 0 && column >= na.x && column < na.x + na.width
                            && row >= na.y && row < na.y + na.height
                        {
                            self.picker_confirm_delete = None;
                            return Ok(());
                        }
                        return Ok(());
                    }
                    // Check click on palin items
                    let area = self.term_area;
                    let picker_x = area.x + 4;
                    let picker_y = area.y + 3;
                    let list_start_y = picker_y + 2;
                    if row >= list_start_y && column >= picker_x && column <= area.x + area.width - 4 {
                        let idx = (row - list_start_y) as usize;
                        if idx < self.all_palins.len() {
                            if column >= area.x + area.width - 16 && column <= area.x + area.width - 4 {
                                // Click on delete button
                                if let Some((name, _)) = self.all_palins.get(idx) {
                                    if name != &self.palin.name {
                                        self.picker_confirm_delete = Some(name.clone());
                                    }
                                }
                            } else {
                                self.picker_selected = idx;
                                if button == MouseButton::Left {
                                    let name_to_switch = self.all_palins.get(idx).map(|(n, _)| n.clone());
                                    if let Some(ref name) = name_to_switch {
                                        let _ = self.switch_to_palin(name);
                                    }
                                }
                            }
                            return Ok(());
                        }
                    }
                    // Click outside to close
                    self.show_picker = false;
                    return Ok(());
                }

                // Check rename mode clicks
                if self.rename_mode {
                    // Commit on click outside (or handle specifically)
                    self.rename_mode = false;
                    return Ok(());
                }

                // 1. Check Snap button
                let btn = self.snap_button_area;
                if column >= btn.x && column < btn.x + btn.width
                    && row >= btn.y && row < btn.y + btn.height
                {
                    self.run_snap();
                    return Ok(());
                }

                // Check GC button
                let gcbtn = self.gc_button_area;
                if gcbtn.width > 0 && column >= gcbtn.x && column < gcbtn.x + gcbtn.width
                    && row >= gcbtn.y && row < gcbtn.y + gcbtn.height
                {
                    self.run_gc_confirm();
                    return Ok(());
                }

                // Check Info button
                let ibtn = self.info_button_area;
                if ibtn.width > 0 && column >= ibtn.x && column < ibtn.x + ibtn.width
                    && row >= ibtn.y && row < ibtn.y + ibtn.height
                {
                    self.run_info();
                    return Ok(());
                }

                // Check Rename button
                let rbtn = self.rename_button_area;
                if rbtn.width > 0 && column >= rbtn.x && column < rbtn.x + rbtn.width
                    && row >= rbtn.y && row < rbtn.y + rbtn.height
                {
                    self.begin_rename();
                    return Ok(());
                }

                // Check output clear button
                let clr = self.output_clear_area;
                if clr.width > 0 && column >= clr.x && column < clr.x + clr.width
                    && row >= clr.y && row < clr.y + clr.height
                {
                    self.clear_output();
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

                // 4. Check detail list area for file click
                let dl = self.detail_list_area;
                if dl.width > 0 && column >= dl.x && column < dl.x + dl.width
                    && row >= dl.y && row < dl.y + dl.height
                {
                    let vis_idx = (row - dl.y) as usize;
                    let file_info = self.filtered_visible_rows().get(vis_idx).map(|r| {
                        (r.full_path.clone(), r.is_dir)
                    });
                    if let Some((ref file_path, is_dir)) = file_info {
                        if !is_dir {
                            if let Some(orig_idx) = self.visible_rows.iter().position(|r| r.full_path == *file_path) {
                                self.selected_file_idx = orig_idx;
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
                        // Compute btn_start from timeline width (display columns) rather than
                        // using the byte-length-based stored value, to avoid misalignment
                        // from multi-byte characters (○, 🔒, etc.) in labels.
                        let btn_start = tl.x + tl.width.saturating_sub(12); // 12 = buttons display width
                        if btn_start > 0 && column >= btn_start && button == MouseButton::Left {
                            let btn_offset = column - btn_start;
                            // First 4 chars: lock indicator
                            if btn_offset < 4 {
                                let epoch_only = item_idx < self.epochs.len();
                                if epoch_only {
                                    let _ = self.toggle_epoch_lock(item_idx);
                                }
                            } else if btn_offset < 8 {
                                // [↩] region: columns 4-7 (4-7 are the 4 chars of " [↩" before the last ])
                                // Actually [↩ is 3 cols but we're generous with the region
                                self.initiate_action(ActionKind::Restore, item_idx, shift);
                            } else {
                                self.initiate_action(ActionKind::Delete, item_idx, shift);
                            }
                            return Ok(());
                        }
                        // Regular click — select the item
                        // If we were waiting for a compare target, run compare
                        if self.compare_mode {
                            // Don't overwrite compare_source_idx — it's stored from enter_compare_mode
                            self.compare_pending_epoch = true;
                            self.compare_mode = false;
                            self.add_output("● Comparing...");
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

    // ── Info ────────────────────────────────────────────────

    pub fn run_info(&mut self) {
        let name = self.palin.name.clone();
        self.run_cmd(&["info", &name]);
    }

    // ── GC ──────────────────────────────────────────────────

    /// Check for unreferenced inks and show confirmation
    pub fn run_gc_confirm(&mut self) {
        let name = self.palin.name.clone();
        let conn = match storage::open_db(&name) {
            Ok(c) => c,
            Err(e) => {
                self.add_output(&format!("✖ DB error: {}", e));
                return;
            }
        };
        let unreferenced = match storage::get_unreferenced_inks(&conn) {
            Ok(u) => u,
            Err(e) => {
                self.add_output(&format!("✖ Error: {}", e));
                return;
            }
        };
        if unreferenced.is_empty() {
            self.add_output("✦ No unreferenced inks to clean up.");
            return;
        }
        let total_size: u64 = unreferenced.iter().map(|i| i.size as u64).sum();
        self.pending_gc = true;
        self.add_output(&format!(
            "  {} unreferenced ink(s) ({} bytes). GC? [y/N]",
            unreferenced.len(),
            total_size
        ));
    }

    /// Execute the garbage collection (delete unreferenced inks)
    pub fn execute_gc(&mut self) {
        self.pending_gc = false;
        let name = self.palin.name.clone();
        let conn = match storage::open_db(&name) {
            Ok(c) => c,
            Err(_) => {
                self.add_output("✖ Failed to open DB");
                return;
            }
        };
        let unreferenced = match storage::get_unreferenced_inks(&conn) {
            Ok(u) => u,
            Err(_) => {
                self.add_output("✖ Failed to query inks");
                return;
            }
        };
        let mut deleted = 0usize;
        let mut total_size = 0u64;
        for ink in &unreferenced {
            if storage::ink::delete_ink_file(&name, &ink.hash).is_ok() {
                let _ = storage::delete_ink_from_db(&conn, &ink.hash);
                deleted += 1;
                total_size += ink.size as u64;
            }
        }
        self.add_output(&format!(
            "✦ GC complete: deleted {} unreferenced ink(s) ({} bytes reclaimed)",
            deleted, total_size
        ));
    }

    // ── Snap ────────────────────────────────────────────────

    pub fn run_snap(&mut self) {
        if self.snap_in_progress {
            self.add_output("  Snap already in progress...");
            return;
        }
        self.snap_in_progress = true;
        self.snap_start_frame = self.frame_count;
        self.patience_shown = false;
        self.add_output("◐ Scanning files...");
        let name = self.palin.name.clone();
        let (tx, rx) = mpsc::channel();
        self.snap_rx = Some(rx);
        std::thread::spawn(move || {
            let res = commands::snap::execute_inner(Some(&name), None);
            match res {
                Ok(lines) => {
                    let _ = tx.send(SnapMessage::Done(lines, Ok(())));
                }
                Err(e) => {
                    let _ = tx.send(SnapMessage::Done(Vec::new(), Err(e)));
                }
            }
        });
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
        self.output_scroll = 0;
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

    pub fn clear_output(&mut self) {
        self.command_output.clear();
        self.output_scroll = 0;
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
                // Store the currently selected epoch as the compare source
                self.compare_source_idx = self.selected_idx;
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

        // Resolve source (original epoch) and target (compare-to epoch) from storage
        let (source_id, source_type_str, source_display) = self.resolve_item_info(compare_source_idx);
        let (target_id, target_type_str, target_display) = self.resolve_item_info(selected_idx);

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

    // ── Palin picker ────────────────────────────────────

    pub fn load_palins(&mut self) {
        match storage::read_registry() {
            Ok(registry) => {
                let mut list: Vec<(String, String)> = registry.palins
                    .iter()
                    .map(|(n, e)| (n.clone(), e.path.clone()))
                    .collect();
                list.sort_by(|a, b| a.0.cmp(&b.0));
                self.all_palins = list;
            }
            Err(_) => {
                self.all_palins = Vec::new();
            }
        }
    }

    pub fn open_picker(&mut self) {
        self.load_palins();
        self.picker_selected = self.all_palins.iter().position(|(n, _)| n == &self.palin.name).unwrap_or(0);
        self.show_picker = true;
        self.picker_confirm_delete = None;
    }

    pub fn switch_to_palin(&mut self, name: &str) -> Result<()> {
        let config = storage::read_palin_config(name)?;
        let conn = storage::open_db(name)?;
        self.palin = ResolvedPalin {
            name: name.to_string(),
            path: std::path::PathBuf::from(&config.path),
            config,
        };
        self.epochs = storage::list_epochs(&conn)?;
        self.phantoms = storage::list_phantoms(&conn)?;
        self.selected_idx = 0;
        self.load_entries()?;
        self.show_picker = false;
        self.add_output(&format!("✦ Switched to '{}'", name));
        Ok(())
    }

    pub fn delete_current_palin(&mut self) -> Result<()> {
        if let Some(ref name) = self.picker_confirm_delete.clone() {
            storage::unregister_palin(name)?;
            let palin_dir = crate::types::palimpsest_dir()?.join(name);
            if palin_dir.exists() {
                std::fs::remove_dir_all(&palin_dir)?;
            }
            self.add_output(&format!("✦ Deleted palin '{}'", name));
            self.picker_confirm_delete = None;
            self.load_palins();
            // Switch to first available palin or reload current
            if let Some(first) = self.all_palins.first().cloned() {
                self.switch_to_palin(&first.0)?;
            }
        }
        Ok(())
    }

    // ── File preview ────────────────────────────────────

    pub fn preview_file(&mut self, file_path: &str) {
        let conn = match storage::open_db(&self.palin.name) {
            Ok(c) => c,
            Err(_) => { return; }
        };
        let items = self.timeline_items();
        if items.is_empty() { return; }
        let idx = self.selected_idx.min(items.len().saturating_sub(1));
        let (snap_id, snap_type) = match items[idx] {
            TimelineItem::Epoch(e) => (e.id, "epoch"),
            TimelineItem::Phantom(p) => (p.id, "phantom"),
        };
        let entries = match storage::get_file_entries(&conn, snap_id, snap_type) {
            Ok(e) => e,
            Err(_) => { return; }
        };
        if let Some(entry) = entries.iter().find(|e| e.file_path == file_path) {
            if let Some(ref hash) = entry.ink_hash {
                match storage::ink::read_ink(&self.palin.name, hash) {
                    Ok(data) => {
                        let text = String::from_utf8_lossy(&data);
                        self.preview_content = text.lines().map(|l| l.to_string()).collect();
                        self.preview_scroll = 0;
                        self.show_preview = true;
                    }
                    Err(_) => {
                        self.add_output(&format!("✖ Failed to read '{}'", file_path));
                    }
                }
            } else {
                self.preview_content = vec!["(empty file / binary)".to_string()];
                self.preview_scroll = 0;
                self.show_preview = true;
            }
        }
    }

    // ── Rename palin ────────────────────────────────────

    pub fn begin_rename(&mut self) {
        self.rename_mode = true;
        self.rename_input = self.palin.name.clone();
        self.rename_cursor = self.rename_input.len();
    }

    pub fn commit_rename(&mut self) -> Result<()> {
        let new_name = self.rename_input.trim().to_string();
        if new_name.is_empty() || new_name == self.palin.name {
            self.rename_mode = false;
            return Ok(());
        }
        let old_name = self.palin.name.clone();
        // Run the rename command via subprocess
        let exe_path = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => { self.rename_mode = false; return Ok(()); }
        };
        let result = std::process::Command::new(&exe_path)
            .args(&["rename", &old_name, &new_name])
            .output();
        match result {
            Ok(output) => {
                if !output.stdout.is_empty() {
                    self.add_output(&String::from_utf8_lossy(&output.stdout));
                }
                if !output.stderr.is_empty() {
                    self.add_output(&format!("✖ {}", String::from_utf8_lossy(&output.stderr)));
                }
                if output.status.success() {
                    self.palin.name = new_name;
                    self.palin.config.name = self.palin.name.clone();
                    self.add_output("✦ Palin renamed");
                    self.load_palins();
                }
            }
            Err(e) => {
                self.add_output(&format!("✖ Rename failed: {}", e));
            }
        }
        self.rename_mode = false;
        Ok(())
    }

    // ── Lock / Unlock ───────────────────────────────────

    pub fn toggle_epoch_lock(&mut self, item_idx: usize) -> Result<()> {
        let epoch = match self.epochs.get(item_idx) {
            Some(e) => e.clone(),
            None => { return Ok(()); }
        };
        let new_locked = !epoch.is_locked;
        let conn = storage::open_db(&self.palin.name)?;
        storage::set_epoch_lock(&conn, epoch.id, new_locked)?;
        let name = epoch.display_name();
        self.reload()?;
        self.add_output(&format!(
            "✦ {} {}",
            if new_locked { "Locked" } else { "Unlocked" },
            name
        ));
        Ok(())
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
            let was_comparing = self.compare_mode;
            let _ = self.load_entries();
            self.compare_mode = was_comparing;
        }
    }

    fn scroll_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
            let was_comparing = self.compare_mode;
            let _ = self.load_entries();
            self.compare_mode = was_comparing;
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
