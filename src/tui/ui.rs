use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

use super::app::{App, TimelineItem};
use super::theme::Theme;

/// Render the entire TUI.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // Output bar at bottom — always visible, dynamic height
    let output_lines = app.command_output.len().min(5) as u16;
    let out_height = output_lines.max(1); // at least 1 line so it's always visible

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),             // Header bar (with Snap button)
            Constraint::Length(1),             // Search bar
            Constraint::Min(1),                // Body
            Constraint::Length(1),             // Status bar
            Constraint::Length(out_height),    // Output bar (always visible)
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_search_bar(frame, chunks[1], app);
    render_body(app, frame, chunks[2]);
    render_status_bar(frame, chunks[3], app);
    render_output_bar(app, frame, chunks[4]);

    if app.show_help {
        super::help::render_help(frame, area);
    }
}

// ── Header bar ─────────────────────────────────────────────────

fn render_header(frame: &mut Frame, area: Rect, app: &mut App) {
    let title = format!(" ◆ Palimpsest — {} ", app.palin.name);
    let path = format!(" {}", app.palin.path.display());

    // Snap button — right-aligned
    let btn_text = " [ ◆ Snap ] ";
    let btn_x = (area.width as usize).saturating_sub(btn_text.len()) as u16;
    let btn_area = Rect::new(btn_x, area.y, btn_text.len() as u16, 1);

    // Store for mouse-click detection
    app.snap_button_area = btn_area;

    let text = Line::from(vec![
        Span::styled(title, Theme::header()),
        Span::styled(path, Theme::dim()),
    ]);
    frame.render_widget(Paragraph::new(text).left_aligned(), area);

    // Render the Snap button on top
    let btn_style = Style::new()
        .bg(Theme::SURFACE)
        .fg(Theme::GREEN)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(btn_text, btn_style))),
        btn_area,
    );
}

// ── Search bar (persistent, below header) ──────────────────────

fn render_search_bar(frame: &mut Frame, area: Rect, app: &mut App) {
    // Background
    let bg_style = Style::new().bg(Theme::SURFACE);
    frame.render_widget(Block::default().style(bg_style), area);

    let is_focused = app.focus == super::app::Focus::Search;

    // Search input area
    let input_x = area.x;
    let input_width = area.width;
    let input_area = Rect::new(input_x, area.y, input_width, 1);
    app.search_bar_area = input_area;

    // Build the search query display
    let cursor_visible = is_focused;
    let query_display = if app.search_query.is_empty() {
        if is_focused {
            "  Type to search...".to_string()
        } else {
            "  Press / to search".to_string()
        }
    } else {
        format!("  {}", app.search_query)
    };

    let query_style = if is_focused {
        Theme::fg().bg(Theme::SURFACE)
    } else {
        Theme::dim().bg(Theme::SURFACE)
    };

    let qd = query_display.clone();
    let mut spans = vec![Span::styled(qd, query_style)];

    // Cursor indicator (green block)
    if cursor_visible {
        spans.push(Span::styled(
            "█",
            Style::new().bg(Theme::SURFACE).fg(Theme::GREEN).add_modifier(Modifier::DIM),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::new().bg(Theme::SURFACE)),
        input_area,
    );
}

// ── Status bar ─────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    // Indicate which panel is focused
    let focus_hint = match app.focus {
        super::app::Focus::Timeline => " Timeline ",
        super::app::Focus::Files => " Files ",
        super::app::Focus::Search => " Search ",
    };

    let hints = [
        (" q ", "Quit"),
        (" ? ", "Help"),
        ("↑↓", "Nav"),
        ("←→", "Focus"),
        (" r ", "Reload"),
        (" s ", "Snap"),
    ];

    // Focus badge at the end
    let focus_badge = Span::styled(focus_hint, Theme::header());

    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {} ", key), Theme::header()),
                Span::styled(format!(" {}  ", desc), Theme::dim()),
            ]
        })
        .collect();

    let info = Span::styled(
        format!(
            "  {} ep | {} ph | {} files",
            app.epochs.len(),
            app.phantoms.len(),
            app.current_entries.len()
        ),
        Theme::dim(),
    );

    let text = Line::from({
        let mut v: Vec<Span> = spans.into_iter().collect();
        v.push(info);
        v.push(focus_badge);
        v
    });

    frame.render_widget(Paragraph::new(text).left_aligned(), area);
}

// ── Body: timeline left + detail right ─────────────────────────

fn render_body(app: &mut App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
        .split(area);

    render_timeline(app, frame, chunks[0]);
    render_detail(app, frame, chunks[1]);
}

// ── Timeline panel (left) — manually rendered for position control ──

fn render_timeline(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(" Timeline ")
        .title_style(Theme::accent())
        .style(Theme::base());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Compute scroll BEFORE timeline_items() to avoid borrow conflicts
    let item_count = app.timeline_len();
    let epoch_count = app.epochs.len();
    let has_both = epoch_count > 0 && app.phantoms.len() > 0;
    let header_rows = if has_both { 2usize } else { 0 }; // "Snapshots" + "Phantoms" headers
    let item_visible = (inner.height as usize).max(1).saturating_sub(header_rows).max(1);

    let scroll = if item_count <= item_visible {
        0
    } else if app.selected_idx < item_visible / 2 {
        0
    } else if app.selected_idx >= item_count - item_visible / 2 {
        item_count - item_visible
    } else {
        app.selected_idx.saturating_sub(item_visible / 2)
    };
    let end = (scroll + item_visible).min(item_count);

    // Store for mouse-click detection
    app.timeline_list_area = inner;
    app.timeline_scroll = scroll;

    let items = app.timeline_items();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("  No snapshots yet.\n  Click [ ◆ Snap ] or press 's'!")
                .style(Theme::dim())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    const BTN_W: u16 = 9; // " [↩] [✕]"

    // Helper to render a centered section header
    let render_header = |frame: &mut Frame, inner: Rect, vis_row: u16, text: &str| {
        let header_y = inner.y + vis_row;
        let header_style = Style::new()
            .fg(Theme::FG)
            .bg(Theme::SURFACE)
            .add_modifier(Modifier::DIM);
        // Draw the full-width background first
        let bg_span = Span::styled(
            " ".repeat(inner.width as usize),
            Style::new().bg(Theme::SURFACE),
        );
        frame.render_widget(
            Paragraph::new(Line::from(bg_span)),
            Rect::new(inner.x, header_y, inner.width, 1),
        );
        // Draw centered header text on top
        let header_x = inner.x + (inner.width.saturating_sub(text.len() as u16)) / 2;
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(text, header_style))),
            Rect::new(header_x, header_y, text.len() as u16, 1),
        );
    };

    // Separate visual row counter (accounts for section headers)
    let mut vis_row = 0u16;

    // Render "Snapshots" header at the top when both groups exist and we're at the top
    if has_both && scroll == 0 {
        render_header(frame, inner, vis_row, " ── Snapshots ── ");
        vis_row += 1;
    }

    for i in scroll..end {
        // Render "Phantoms" section header when transitioning from epochs to phantoms
        if has_both && i == epoch_count {
            render_header(frame, inner, vis_row, " ── Phantoms ── ");
            vis_row += 1;
        }

        let item = &items[i];
        let is_selected = i == app.selected_idx;
        let prefix = if is_selected { " >" } else { "  " };

        let (display_text, base_style) = match item {
            TimelineItem::Epoch(epoch) => {
                let name = epoch.display_name();
                let msg = epoch.message.as_deref().unwrap_or("");
                let locked = if epoch.is_locked { " 🔒" } else { "" };
                let date = epoch.timestamp.format("%m/%d %H:%M");
                let text = format!("{} {}  {}  {}{}", prefix, name, date, msg, locked);
                let st = if is_selected {
                    Theme::selected()
                } else if epoch.is_origin {
                    Theme::origin_badge()
                } else if epoch.is_locked {
                    Theme::accent()
                } else {
                    Theme::base()
                };
                (text, st)
            }
            TimelineItem::Phantom(phantom) => {
                let name = phantom.display_name();
                let ttl = phantom.remaining_ttl();
                let hours = ttl.num_hours().max(0);
                let mins = ttl.num_minutes().max(0) % 60;
                let date = phantom.timestamp.format("%m/%d %H:%M");
                let text = format!("{} ○ {}  {}  ({}h{}m)", prefix, name, date, hours, mins);
                let st = if is_selected { Theme::selected() } else { Theme::dim() };
                (text, st)
            }
        };

        let mut text_style = base_style;
        if is_selected {
            text_style = text_style.add_modifier(Modifier::BOLD);
        }

        // Build the row string with action buttons
        let row_text = {
            let max_label = (inner.width.saturating_sub(BTN_W).saturating_sub(1)) as usize;
            let label = if display_text.len() > max_label {
                format!("{}…", &display_text[..max_label.saturating_sub(1)])
            } else {
                display_text
            };
            // Pad to fill remaining space before buttons
            let pad = inner.width.saturating_sub(label.len() as u16 + BTN_W);
            format!("{}  {}  [↩] [✕]", label, " ".repeat(pad.saturating_sub(4) as usize))
        };

        let row_y = inner.y + vis_row;
        let row_rect = Rect::new(inner.x, row_y, inner.width, 1);

        // Render the background (selection highlight manually)
        let bg_color = if is_selected { Theme::SURFACE } else { Theme::BG };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                &row_text,
                text_style.bg(bg_color),
            ))),
            row_rect,
        );

        vis_row += 1;
    }
}

// ── Detail panel (right) ───────────────────────────────────────

fn render_detail(app: &mut App, frame: &mut Frame, area: Rect) {
    let items = app.timeline_items();

    if items.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border())
            .title(" Details ")
            .title_style(Theme::accent())
            .style(Theme::base());
        frame.render_widget(
            Paragraph::new("  No data to display.")
                .style(Theme::dim())
                .block(block),
            area,
        );
        return;
    }

    // ── Diff / Compare view ──────────────────────────────
    if !app.compare_result_lines.is_empty() || app.compare_error.is_some() {
        render_diff_view(app, frame, area);
        return;
    }

    let item = items[app.selected_idx];

    // Build the detail header
    let (title, subtitle, badge) = match item {
        TimelineItem::Epoch(epoch) => {
            let name = epoch.display_name();
            let date = epoch.timestamp.format("%Y-%m-%d %H:%M:%S");
            let msg = epoch.message.as_deref().unwrap_or("(no message)");
            let badge_text = if epoch.is_locked {
                " 🔒 LOCKED "
            } else if epoch.is_origin {
                " ◆ ORIGIN "
            } else {
                ""
            };
            (
                format!(" {} ", name),
                format!("{}  —  {}", date, msg),
                badge_text.to_string(),
            )
        }
        TimelineItem::Phantom(phantom) => {
            let name = phantom.display_name();
            let date = phantom.timestamp.format("%Y-%m-%d %H:%M:%S");
            let ttl = phantom.remaining_ttl();
            let hours = ttl.num_hours().max(0);
            let mins = ttl.num_minutes().max(0) % 60;
            let msg = phantom.message.as_deref().unwrap_or("auto-backup");
            (
                format!(" {} ", name),
                format!("{}  —  {}  (expires in {}h{}m)", date, msg, hours, mins),
                String::new(),
            )
        }
    };

    let mut title_text = title;
    if !badge.is_empty() {
        title_text.push_str(&badge);
    }
    if app.compare_mode {
        title_text.push_str(&format!(" — comparing '{}'", app.compare_file_path));
    }

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(format!(" {} ", title_text))
        .title_style(Theme::accent_bold())
        .style(Theme::base());

    let inner = header_block.inner(area);
    frame.render_widget(header_block, area);

    // Subtitle + file list
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    // Subtitle
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(subtitle, Theme::dim()))),
        vchunks[0],
    );

    // Store the detail list area for click detection
    app.detail_list_area = vchunks[1];

    // File tree (hierarchical with expand/collapse) — filtered by search
    let rows = app.filtered_visible_rows();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("  (no files)").style(Theme::dim()),
            vchunks[1],
        );
        return;
    }

    let list_height = vchunks[1].height.saturating_sub(1) as usize;
    let max_visible = list_height.max(1);

    // Convert from filtered list indices back to original indices
    // to maintain selected_file_idx alignment
    let filtered_count = rows.len();
    let adj_sel = app.selected_file_idx.min(filtered_count.saturating_sub(1));

    let scroll_offset = if adj_sel >= max_visible {
        adj_sel.saturating_sub(max_visible).saturating_add(1)
    } else {
        0
    };

    let visible = rows
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(max_visible);

    let file_items: Vec<ListItem> = visible.map(|(i, row)| {
        let indent = "  ".repeat(row.depth);
        let is_selected = i + scroll_offset == adj_sel;

        if row.is_dir {
            // Show colored status dot for directories with modified children
            let marker = if row.is_expanded { "▾" } else { "▸" };
            let dot_color = match row.status {
                Some(crate::types::FileStatus::Added) => Theme::GREEN,
                Some(crate::types::FileStatus::Modified) => Theme::YELLOW,
                Some(crate::types::FileStatus::Deleted) => Theme::RED,
                _ => Theme::FG,
            };
            let dir_style = if is_selected { Theme::selected() } else { Theme::accent() };
            return ListItem::new(Line::from(vec![
                Span::styled(indent, dir_style),
                Span::styled("●", Style::new().fg(dot_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", marker), dir_style),
                Span::styled(&row.name, dir_style),
                Span::styled("  (dir)", Theme::dim()),
            ]));
        }

        let status_char = app.status_for_entry(row);
        let status_style = app.status_color_for(row);
        ListItem::new(Line::from(vec![
            Span::styled(format!("{}  {} ", indent, status_char), status_style),
            Span::styled(&row.name, if is_selected { Theme::selected() } else { status_style }),
            Span::styled(
                row.file_size.map_or(String::new(), |s| {
                    if s > 1_048_576 {
                        format!(" {:>7.1} MB", s as f64 / 1_048_576.0)
                    } else if s > 1024 {
                        format!(" {:>7.1} KB", s as f64 / 1024.0)
                    } else {
                        format!(" {:>7} B", s)
                    }
                }),
                Theme::dim(),
            ),
        ]))
    }).collect();

    let adj = adj_sel.saturating_sub(scroll_offset);
    let mut fs = ListState::default().with_selected(Some(adj));
    let list = List::new(file_items)
        .highlight_style(Theme::selected())
        .highlight_symbol("");
    frame.render_stateful_widget(list, vchunks[1], &mut fs);
}

// ── Diff view (shown when comparing file between epochs) ───────

fn render_diff_view(app: &App, frame: &mut Frame, area: Rect) {
    let file_path = &app.compare_file_path;

    let title = if let Some(ref err) = app.compare_error {
        format!(" Diff — {} ", err)
    } else {
        format!(" Diff — {} ", file_path)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(title)
        .title_style(Theme::accent_bold())
        .style(Theme::base());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(ref err) = app.compare_error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  ✖ {}", err),
                Theme::deleted(),
            ))),
            inner,
        );
        return;
    }

    if app.compare_result_lines.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  (no differences)",
                Theme::dim(),
            ))),
            inner,
        );
        return;
    }

    let max_lines = inner.height as usize;
    let total = app.compare_result_lines.len();
    let scroll = app.diff_scroll.min(total.saturating_sub(max_lines).max(0));

    // Show scroll indicator if content exceeds visible area
    let scroll_hint = if total > max_lines {
        let pct = if total > 0 {
            (scroll * 100) / total
        } else {
            0
        };
        format!("  [{}% scrolled — ↑/↓ to navigate]", pct)
    } else {
        String::new()
    };

    let lines: Vec<Line> = app
        .compare_result_lines
        .iter()
        .skip(scroll)
        .take(max_lines)
        .map(|dl| {
            let prefix = " ";
            let (marker, style) = match dl.line_type {
                super::app::DiffLineType::Added => {
                    ("+", Theme::added())
                }
                super::app::DiffLineType::Deleted => {
                    ("-", Theme::deleted())
                }
                super::app::DiffLineType::Replaced => {
                    // Replaced lines are removed content — show as deletion
                    ("-", Theme::deleted())
                }
                super::app::DiffLineType::Same => {
                    (" ", Style::default().fg(Theme::FG))
                }
            };
            Line::from(Span::styled(
                format!("{}{} {}", prefix, marker, dl.content),
                style,
            ))
        })
        .collect();

    // First render the scroll hint at the bottom, then the diff content
    // We use a vertical split: one line for hint, rest for content
    if !scroll_hint.is_empty() {
        // Render the hint line at the very top of inner
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&scroll_hint, Theme::dim()))),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        // Shift content down by 1
        let content_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));
        // Render as much content as fits (re-calculate for shifted area)
        let content_lines: Vec<Line> = app
            .compare_result_lines
            .iter()
            .skip(scroll)
            .take(content_area.height as usize)
            .map(|dl| {
                let (marker, style) = match dl.line_type {
                    super::app::DiffLineType::Added => ("+", Theme::added()),
                    super::app::DiffLineType::Deleted => ("-", Theme::deleted()),
                    super::app::DiffLineType::Replaced => {
                        ("-", Theme::deleted())
                    }
                    super::app::DiffLineType::Same => {
                        (" ", Style::default().fg(Theme::FG))
                    }
                };
                Line::from(Span::styled(
                    format!(" {} {}", marker, dl.content),
                    style,
                ))
            })
            .collect();
        frame.render_widget(
            Paragraph::new(Text::from(content_lines)),
            content_area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(Text::from(lines)),
            inner,
        );
    }
}

// ── Output bar (with optional confirmation prompt) ───────────

fn render_output_bar(app: &mut App, frame: &mut Frame, area: Rect) {
    // Fill the entire area with slate background
    let bg_block = Block::default().style(Style::new().bg(Theme::SURFACE));
    frame.render_widget(bg_block, area);

    // If there's a pending action, show a confirmation prompt
    if let Some(ref action) = app.pending_action {
        let prompt = format!(
            "  {} {}?  ",
            match action.kind {
                super::app::ActionKind::Restore => "Restore to",
                super::app::ActionKind::Delete => "Delete",
            },
            action.item_name
        );

        let yes_text = " [ Yes ] ";
        let no_text = " [ No ] ";

        let prompt_len = prompt.len() as u16;
        let yes_x = area.x + prompt_len;
        let no_x = yes_x + yes_text.len() as u16;

        // Store button areas for click detection
        app.confirm_yes_area = Rect::new(yes_x, area.y, yes_text.len() as u16, 1);
        app.confirm_no_area = Rect::new(no_x, area.y, no_text.len() as u16, 1);

        let yes_style = Style::new()
            .bg(Theme::GREEN)
            .fg(Theme::BG)
            .add_modifier(Modifier::BOLD);
        let no_style = Style::new()
            .bg(Theme::RED)
            .fg(Theme::FG)
            .add_modifier(Modifier::BOLD);

        let spans = vec![
            Span::styled(prompt, Theme::fg().bg(Theme::SURFACE)),
            Span::styled(yes_text, yes_style),
            Span::styled(no_text, no_style),
        ];

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(Theme::SURFACE)),
            area,
        );
        return;
    }

    let count = app.command_output.len().min(5);
    if count == 0 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  ◆ Palimpsest — click Snap to take a snapshot",
                Theme::dim().bg(Theme::SURFACE),
            ))),
            area,
        );
        return;
    }

    let lines: Vec<Line> = app
        .command_output
        .iter()
        .take(5)
        .rev()
        .map(|line| {
            let style = if line.starts_with('✖') {
                Theme::deleted()
            } else if line.starts_with('✦') || line.starts_with('●') {
                Theme::accent()
            } else if line.starts_with('>') {
                Theme::accent_bold()
            } else {
                Theme::dim()
            };
            Line::from(Span::styled(format!(" {}", line), style.bg(Theme::SURFACE)))
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines)).style(Style::new().bg(Theme::SURFACE)),
        area,
    );
}
