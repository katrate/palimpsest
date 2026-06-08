use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
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

    // ── Outer vertical split: header / body / footer ──────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),    // Header
            Constraint::Min(1),       // Body
            Constraint::Length(1),    // Footer
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_body(app, frame, chunks[1]);
    render_footer(frame, chunks[2], app);

    // ── Help overlay (on top of everything) ───────────────────
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

// ── Footer bar ─────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hints = vec![
        (" q ", "Quit"),
        (" ? ", "Help"),
        (" j/k ", "Nav"),
        (" r ", "Reload"),
    ];

    let spans: Vec<Span> = hints
        .into_iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {} ", key), Theme::header()),
                Span::styled(format!(" {} ", desc), Theme::dim()),
            ]
        })
        .collect();

    let epoch_count = app.epochs.len();
    let info = Span::styled(
        format!("  {} epochs | {} phantoms ", epoch_count, app.phantoms.len()),
        Theme::dim(),
    );

    let text = Line::from({
        let mut v = spans;
        v.push(info);
        v
    });

    frame.render_widget(Paragraph::new(text).left_aligned(), area);
}

// ── Body: left timeline + right detail ─────────────────────────

fn render_body(app: &mut App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),  // Timeline
            Constraint::Ratio(2, 3),  // Detail + diff
        ])
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
        .title_style(Theme::accent());

    let inner = block.inner(area);

    // Build items
    let items = app.timeline_items();
    let list_items: Vec<ListItem> = items.iter().enumerate().map(|(i, item)| {
        let is_selected = i == app.selected_idx;
        let (label, style) = match item {
            TimelineItem::Epoch(epoch) => {
                let name = epoch.display_name();
                let msg = epoch.message.as_deref().unwrap_or("");
                let locked = if epoch.is_locked { " 🔒" } else { "" };
                let date = epoch.timestamp.format("%m/%d %H:%M");
                if is_selected {
                    (format!(" ▶ {}  {}  {}{}", name, date, msg, locked), Theme::selected())
                } else if epoch.is_origin {
                    (format!("   {}  {}  {}{}", name, date, msg, locked), Theme::accent())
                } else {
                    (format!("   {}  {}  {}{}", name, date, msg, locked), Theme::base())
                }
            }
            TimelineItem::Phantom(phantom) => {
                let name = phantom.display_name();
                let msg = phantom.message.as_deref().unwrap_or("auto-backup");
                let ttl = phantom.remaining_ttl();
                let hours = ttl.num_hours();
                let mins = ttl.num_minutes() % 60;
                let date = phantom.timestamp.format("%m/%d %H:%M");
                if is_selected {
                    (format!(" ▶ {}  {}  {} ({}h{}m)", name, date, msg, hours, mins), Theme::selected())
                } else {
                    (format!("   {}  {}  {} ({}h{}m)", name, date, msg, hours, mins), Theme::dim())
                }
            }
        };

        ListItem::new(Line::from(
            Span::styled(label, style.add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }))
        ))
    }).collect();

    frame.render_widget(block, area);

    if list_items.is_empty() {
        let empty = Paragraph::new("  No snapshots yet.\n  Run `palin snap`!")
            .style(Theme::dim())
            .wrap(Wrap { trim: true });
        frame.render_widget(empty, inner);
        return;
    }

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
            .title_style(Theme::accent());
        let empty = Paragraph::new("  No data to display.")
            .style(Theme::dim())
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let item = items[app.selected_idx];

    // Build the header block for the detail panel
    let (title, subtitle, locked_tag) = match item {
        TimelineItem::Epoch(epoch) => {
            let name = epoch.display_name();
            let date = epoch.timestamp.format("%Y-%m-%d %H:%M:%S");
            let msg = epoch.message.as_deref().unwrap_or("(no message)");
            let lock = if epoch.is_locked { " 🔒 LOCKED" } else { "" };
            (name, format!("{}  —  {}", date, msg), lock.to_string())
        }
        TimelineItem::Phantom(phantom) => {
            let name = phantom.display_name();
            let date = phantom.timestamp.format("%Y-%m-%d %H:%M:%S");
            let ttl = phantom.remaining_ttl();
            let hours = ttl.num_hours();
            let mins = ttl.num_minutes() % 60;
            let msg = phantom.message.as_deref().unwrap_or("auto-backup");
            (name, format!("{}  —  {}  (expires in {}h{}m)", date, msg, hours, mins), String::new())
        }
    };

    let mut title_text = format!("  {}  ", title);
    if !locked_tag.is_empty() {
        title_text.push_str(&locked_tag);
    }

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(format!(" {} ", title_text))
        .title_style(Theme::accent_bold());

    // Split the detail area into info top and file list bottom
    let inner = header_block.inner(area);
    frame.render_widget(header_block, area);

    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Subtitle
            Constraint::Min(1),     // File list
        ])
        .split(inner);

    // Subtitle line
    let sub_line = Paragraph::new(Line::from(Span::styled(subtitle, Theme::dim())));
    frame.render_widget(sub_line, vchunks[0]);

    // File list
    let entries = &app.current_entries;
    if entries.is_empty() {
        let no_files = Paragraph::new("  (no file changes)")
            .style(Theme::dim());
        frame.render_widget(no_files, vchunks[1]);
        return;
    }

    // Viewport offset — only show files that fit the visible area
    let list_height = vchunks[1].height.saturating_sub(1) as usize;
    let max_visible = list_height.max(1);
    let scroll_offset = if app.selected_file_idx >= max_visible {
        app.selected_file_idx.saturating_sub(max_visible).saturating_add(1)
    } else {
        0
    };
    let visible_entries = entries
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(max_visible);

    let file_list_items: Vec<ListItem> = visible_entries.map(|(i, entry)| {
        let status_char = app.status_for(entry);
        let status_style = app.status_color(entry);
        let size = entry.file_size.unwrap_or(0);
        let size_str = if size > 1024 * 1024 {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        } else if size > 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{} B", size)
        };

        let path_str = &entry.file_path;
        // Truncate long paths
        let display_path = if path_str.len() > 50 {
            format!("...{}", &path_str[path_str.len().saturating_sub(47)..])
        } else {
            path_str.clone()
        };

        let is_selected = i == app.selected_file_idx;
        let base_style = if is_selected { Theme::selected() } else { status_style };

        let line = Line::from(vec![
            Span::styled(format!(" {} ", status_char), status_style),
            Span::styled(display_path, base_style),
            Span::styled(format!("  ({})", size_str), Theme::dim()),
        ]);
        ListItem::new(line)
    }).collect();

    let adjusted_selected = app.selected_file_idx.saturating_sub(scroll_offset);
    let mut file_state = ListState::default().with_selected(Some(adjusted_selected));
    let file_list = List::new(file_list_items)
        .highlight_style(Theme::selected())
        .highlight_symbol("");
    frame.render_stateful_widget(file_list, vchunks[1], &mut file_state);
}
