use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
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

    // Command bar takes bottom space when active
    let (main_area, cmd_area) = if app.command_mode {
        let cmd_h = (app.command_output.len().min(5) + 2) as u16;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(cmd_h.max(3))])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Main vertical split: header / body / footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header bar
            Constraint::Min(1),    // Body
            Constraint::Length(1), // Status bar
        ])
        .split(main_area);

    render_header(frame, chunks[0], app);
    render_body(app, frame, chunks[1]);
    render_status_bar(frame, chunks[2], app);

    if let Some(cmd_area) = cmd_area {
        render_command_bar(app, frame, cmd_area);
    }

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
        (" : ", "Cmd"),
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

    // File list
    let entries = &app.current_entries;
    if entries.is_empty() {
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

    let visible = entries
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(max_visible);

    let file_items: Vec<ListItem> = visible.map(|(i, entry)| {
        let status_char = app.status_for(entry);
        let status_style = app.status_color(entry);

        let size = entry.file_size.unwrap_or(0);
        let size_str = if size > 1_048_576 {
            format!("{:>8.1} MB", size as f64 / 1_048_576.0)
        } else if size > 1024 {
            format!("{:>8.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:>8} B", size)
        };

        let path = &entry.file_path;
        let display_path = if path.len() > 48 {
            format!("...{}", &path[path.len().saturating_sub(45)..])
        } else {
            path.clone()
        };

        let is_selected = i == app.selected_file_idx;
        let base_style = if is_selected { Theme::selected() } else { status_style };

        let line = Line::from(vec![
            Span::styled(format!(" {} ", status_char), status_style),
            Span::styled(display_path, base_style),
            Span::styled(format!(" {}", size_str), Theme::dim()),
        ]);
        ListItem::new(line)
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
    let output_count = app.command_output.len().min(5);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if output_count > 0 {
            vec![Constraint::Length(output_count as u16), Constraint::Length(2)]
        } else {
            vec![Constraint::Min(0), Constraint::Length(2)]
        })
        .split(area);

    // Output lines — newest at bottom, scrolling up
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
            Line::from(Span::styled(format!(" {}", line), style))
        }).collect();

        frame.render_widget(
            Paragraph::new(Text::from(lines)).style(Theme::base()),
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
        Theme::separator(),
    ));

    let cursor_visible = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_millis() / 500) % 2 == 0)
        .unwrap_or(true);

    // Build the input line spans
    let input_spans = if app.command_buffer.is_empty() {
        let cursor = if cursor_visible { "█" } else { " " };
        vec![
            Span::styled(" > ", Theme::prompt()),
            Span::styled(cursor, Theme::fg()),
        ]
    } else {
        let before = &app.command_buffer[..app.command_cursor.min(app.command_buffer.len())];
        let after = &app.command_buffer[app.command_cursor.min(app.command_buffer.len())..];

        let mut spans = vec![
            Span::styled(" > ", Theme::prompt()),
            Span::styled(before.to_string(), Theme::fg()),
        ];

        if cursor_visible && after.is_empty() {
            spans.push(Span::styled(after.to_string(), Theme::fg()));
            spans.push(Span::styled("█", Theme::fg()));
        } else if cursor_visible {
            let ch = &after[..1];
            let rest = &after[1..];
            spans.push(Span::styled(format!("█{}", ch), Theme::fg()));
            spans.push(Span::styled(rest.to_string(), Theme::fg()));
        } else {
            spans.push(Span::styled(after.to_string(), Theme::fg()));
        }
        spans
    };

    // Render: separator on top line, input on bottom line
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(input_area);

    frame.render_widget(
        Paragraph::new(sep).style(Theme::base()),
        input_chunks[0],
    );

    frame.render_widget(
        Paragraph::new(Line::from(input_spans)).style(Theme::base()),
        input_chunks[1],
    );
}
