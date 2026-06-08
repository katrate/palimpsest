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

    // Always reserve space for the command bar at the bottom
    let has_output = !app.command_output.is_empty();
    let output_lines = if app.command_mode || has_output {
        app.command_output.len().min(5) as u16
    } else {
        0
    };
    let cmd_bar_height = (output_lines + 2).max(2); // output + separator + input line

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                  // Header bar
            Constraint::Min(1),                     // Body
            Constraint::Length(1),                  // Status bar
            Constraint::Length(cmd_bar_height),     // Command bar (always visible)
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_body(app, frame, chunks[1]);
    render_status_bar(frame, chunks[2], app);
    render_command_bar(app, frame, chunks[3]);

    if app.show_help {
        super::help::render_help(frame, area);
    }
}

// ── Header bar ─────────────────────────────────────────────────

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(" ◆ Palimpsest — {} ", app.palin.name);
    let path = format!(" {}", app.palin.path.display());

    let text = Line::from(vec![
        Span::styled(title, Theme::header()),
        Span::styled(path, Theme::dim()),
    ]);
    frame.render_widget(Paragraph::new(text).left_aligned(), area);
}

// ── Status bar ─────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let hints = [
        (" q ", "Quit"),
        (" ? ", "Help"),
        (" j/k ", "Nav"),
        (" r ", "Reload"),
    ];

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

// ── Timeline panel (left) ──────────────────────────────────────

fn render_timeline(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(" Timeline ")
        .title_style(Theme::accent())
        .style(Theme::base());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items = app.timeline_items();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("  No snapshots yet.\n  Press `:` then `snap`!")
                .style(Theme::dim())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let list_items: Vec<ListItem> = items.iter().enumerate().map(|(i, item)| {
        let is_selected = i == app.selected_idx;
        let prefix = if is_selected { " ▶" } else { "  " };

        let (label, style) = match item {
            TimelineItem::Epoch(epoch) => {
                let name = epoch.display_name();
                let msg = epoch.message.as_deref().unwrap_or("");
                let locked = if epoch.is_locked { " 🔒" } else { "" };
                let date = epoch.timestamp.format("%m/%d %H:%M");

                let labels = if epoch.is_origin {
                    format!("{} {}  {}  {}{}", prefix, name, date, msg, locked)
                } else {
                    format!("{} {}  {}  {}{}", prefix, name, date, msg, locked)
                };

                if is_selected {
                    (labels, Theme::selected())
                } else if epoch.is_origin {
                    (labels, Theme::origin_badge())
                } else if epoch.is_locked {
                    (labels, Theme::accent())
                } else {
                    (labels, Theme::base())
                }
            }
            TimelineItem::Phantom(phantom) => {
                let name = phantom.display_name();
                let ttl = phantom.remaining_ttl();
                let hours = ttl.num_hours().max(0);
                let mins = ttl.num_minutes().max(0) % 60;
                let date = phantom.timestamp.format("%m/%d %H:%M");
                let label = format!(
                    "{} ○ {}  {}  ({}h{}m)",
                    prefix, name, date, hours, mins
                );

                if is_selected {
                    (label, Theme::selected())
                } else {
                    (label, Theme::dim())
                }
            }
        };

        let bold = if is_selected { Modifier::BOLD } else { Modifier::empty() };
        ListItem::new(Line::from(Span::styled(label, style.add_modifier(bold))))
    }).collect();

    let mut state = ListState::default().with_selected(Some(app.selected_idx));
    let list = List::new(list_items)
        .highlight_style(Theme::selected())
        .highlight_symbol("")
        .repeat_highlight_symbol(true);
    frame.render_stateful_widget(list, inner, &mut state);
}

// ── Detail panel (right) ───────────────────────────────────────

fn render_detail(app: &App, frame: &mut Frame, area: Rect) {
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

    // File tree (hierarchical with expand/collapse)
    let rows = &app.visible_rows;
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("  (no files in this snapshot)").style(Theme::dim()),
            vchunks[1],
        );
        return;
    }

    let list_height = vchunks[1].height.saturating_sub(1) as usize;
    let max_visible = list_height.max(1);
    let scroll_offset = if app.selected_file_idx >= max_visible {
        app.selected_file_idx.saturating_sub(max_visible).saturating_add(1)
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
        let folder_marker = if row.is_dir {
            if row.is_expanded { "▾ " } else { "▸ " }
        } else {
            let status_char = app.status_for_entry(row);
            let status_style = app.status_color_for(row);
            return ListItem::new(Line::from(vec![
                Span::styled(format!("{}  {} ", indent, status_char), status_style),
                Span::styled(&row.name, if i + scroll_offset == app.selected_file_idx { Theme::selected() } else { status_style }),
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
            ]));
        };

        let is_selected = i + scroll_offset == app.selected_file_idx;
        let dir_style = if is_selected { Theme::selected() } else { Theme::accent() };

        ListItem::new(Line::from(vec![
            Span::styled(format!("{}{}", indent, folder_marker), dir_style),
            Span::styled(&row.name, dir_style),
            Span::styled("  (dir)", Theme::dim()),
        ]))
    }).collect();

    let adj = app.selected_file_idx.saturating_sub(scroll_offset);
    let mut fs = ListState::default().with_selected(Some(adj));
    let list = List::new(file_items)
        .highlight_style(Theme::selected())
        .highlight_symbol("");
    frame.render_stateful_widget(list, vchunks[1], &mut fs);
}

// ── Command bar ────────────────────────────────────────────────

fn render_command_bar(app: &App, frame: &mut Frame, area: Rect) {
    // Fill the entire command bar area with slate background
    let bg_block = Block::default().style(
        Style::new().bg(Theme::SURFACE),
    );
    frame.render_widget(bg_block, area);

    let output_count = app.command_output.len().min(5);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if output_count > 0 {
            vec![Constraint::Length(output_count as u16), Constraint::Length(2)]
        } else {
            vec![Constraint::Min(0), Constraint::Length(2)]
        })
        .split(area);

    // Output lines — newest at bottom, scrolling up (on slate bg)
    if output_count > 0 {
        let lines: Vec<Line> = app.command_output.iter().take(5).rev().map(|line| {
            let style = if line.starts_with('✖') {
                Theme::deleted()
            } else if line.starts_with('✦') || line.starts_with('●') {
                Theme::accent()
            } else if line.starts_with('>') {
                Theme::accent_bold()
            } else {
                Theme::dim()
            };
            // Override bg to SURFACE so output blends with the bar
            Line::from(Span::styled(format!(" {}", line), style.bg(Theme::SURFACE)))
        }).collect();

        frame.render_widget(
            Paragraph::new(Text::from(lines)).style(Style::new().bg(Theme::SURFACE)),
            chunks[0],
        );
    }

    // Input line — with separator bar
    let input_area = chunks[chunks.len() - 1];

    // Separator line
    let sep = Line::from(Span::styled(
        format!(
            "{}",
            "─".repeat(input_area.width.saturating_sub(1) as usize)
        ),
        Theme::accent()
            .bg(Theme::SURFACE)
            .add_modifier(Modifier::DIM),
    ));

    let cursor_visible = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_millis() / 500) % 2 == 0)
        .unwrap_or(true);

    // Build the input line spans (all on slate bg)
    let input_spans = if !app.command_mode {
        // Show a bright hint when not actively typing a command
        vec![
            Span::styled(" > ", Theme::prompt().bg(Theme::SURFACE)),
            Span::styled(
                "Press : for command",
                Theme::fg()
                    .bg(Theme::SURFACE)
                    .add_modifier(Modifier::DIM),
            ),
        ]
    } else if app.command_buffer.is_empty() {
        let cursor = if cursor_visible { "█" } else { " " };
        vec![
            Span::styled(" > ", Theme::prompt().bg(Theme::SURFACE)),
            Span::styled(cursor, Style::new().fg(Theme::FG).bg(Theme::SURFACE)),
        ]
    } else {
        let before = &app.command_buffer[..app.command_cursor.min(app.command_buffer.len())];
        let after = &app.command_buffer[app.command_cursor.min(app.command_buffer.len())..];

        let mut spans = vec![
            Span::styled(" > ", Theme::prompt().bg(Theme::SURFACE)),
            Span::styled(before.to_string(), Style::new().fg(Theme::FG).bg(Theme::SURFACE)),
        ];

        if cursor_visible && after.is_empty() {
            spans.push(Span::styled(after.to_string(), Style::new().fg(Theme::FG).bg(Theme::SURFACE)));
            spans.push(Span::styled("█", Style::new().fg(Theme::FG).bg(Theme::SURFACE)));
        } else if cursor_visible {
            let ch = &after[..1];
            let rest = &after[1..];
            spans.push(Span::styled(format!("█{}", ch), Style::new().fg(Theme::FG).bg(Theme::SURFACE)));
            spans.push(Span::styled(rest.to_string(), Style::new().fg(Theme::FG).bg(Theme::SURFACE)));
        } else {
            spans.push(Span::styled(after.to_string(), Style::new().fg(Theme::FG).bg(Theme::SURFACE)));
        }
        spans
    };

    // Render: separator on top line, input on bottom line
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(input_area);

    frame.render_widget(
        Paragraph::new(sep).style(Style::new().bg(Theme::SURFACE)),
        input_chunks[0],
    );

    frame.render_widget(
        Paragraph::new(Line::from(input_spans)).style(Style::new().bg(Theme::SURFACE)),
        input_chunks[1],
    );
}
