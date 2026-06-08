use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

use chrono::Local;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::app::{App, TimelineItem};
use super::theme::Theme;

/// Render the entire TUI.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // When picker is open, render nothing except the picker itself
    if app.show_picker {
        render_picker(app, frame, area);
        return;
    }

    let out_height = 8u16;

    // Fill entire area with deep bg
    frame.render_widget(
        Block::default().style(Style::new().bg(Theme::BG_DEEPER)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(out_height),
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
    // Full-width background
    frame.render_widget(
        Block::default().style(Style::new().bg(Theme::SURFACE).fg(Theme::FG)),
        area,
    );

    let gc_style = Theme::btn_warning();

    if app.rename_mode {
        let prompt_text = format!(" Rename: {}_", app.rename_input);
        let gc_text = " [GC] ";
        let info_text = " [Info] ";
        let snap_text = " [ ◆ Snap ] ";
        let total_other = gc_text.len() as u16 + info_text.len() as u16 + snap_text.len() as u16;
        let gc_x = (area.width as usize).saturating_sub(total_other as usize) as u16;
        let info_x = gc_x + gc_text.len() as u16;
        let snap_x = info_x + info_text.len() as u16;
        let gc_area = Rect::new(gc_x, area.y, gc_text.len() as u16, 1);
        let info_area = Rect::new(info_x, area.y, info_text.len() as u16, 1);
        let snap_area = Rect::new(snap_x, area.y, snap_text.len() as u16, 1);
        app.gc_button_area = gc_area;
        app.snap_button_area = snap_area;
        app.info_button_area = info_area;
        app.rename_button_area = Rect::default();

        let spans = vec![
            Span::styled(prompt_text, Theme::fg().bg(Theme::SURFACE).add_modifier(Modifier::BOLD)),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)).left_aligned(), area);

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(gc_text, gc_style))),
            gc_area,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(info_text, Theme::btn_secondary()))),
            info_area,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(snap_text, Theme::btn_primary()))),
            snap_area,
        );
        return;
    }

    let title = format!(" ◆ {} ", app.palin.name);
    let path = format!(" {}", app.palin.path.display());

    // Buttons: GC | Info | Rename | Snap — right-aligned
    let gc_text = " [GC] ";
    let info_text = " [Info] ";
    let rename_text = " [Rename] ";
    let snap_text = " [ ◆ Snap ] ";
    let total_btn_w = gc_text.len() as u16 + info_text.len() as u16 + rename_text.len() as u16 + snap_text.len() as u16;
    let gc_x = (area.width as usize).saturating_sub(total_btn_w as usize) as u16;
    let info_x = gc_x + gc_text.len() as u16;
    let rename_x = info_x + info_text.len() as u16;
    let snap_x = rename_x + rename_text.len() as u16;

    let gc_area = Rect::new(gc_x, area.y, gc_text.len() as u16, 1);
    let info_area = Rect::new(info_x, area.y, info_text.len() as u16, 1);
    let rename_area = Rect::new(rename_x, area.y, rename_text.len() as u16, 1);
    let snap_area = Rect::new(snap_x, area.y, snap_text.len() as u16, 1);

    app.gc_button_area = gc_area;
    app.snap_button_area = snap_area;
    app.rename_button_area = rename_area;
    app.info_button_area = info_area;

    let text = Line::from(vec![
        Span::styled(title, Theme::accent_bold().bg(Theme::SURFACE)),
        Span::styled(path, Theme::dim().bg(Theme::SURFACE)),
    ]);
    frame.render_widget(Paragraph::new(text).left_aligned().style(Style::new().bg(Theme::SURFACE)), area);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(gc_text, gc_style))),
        gc_area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(info_text, Theme::btn_secondary()))),
        info_area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(rename_text, Theme::btn_neutral()))),
        rename_area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(snap_text, Theme::btn_primary()))),
        snap_area,
    );
}

// ── Search bar (persistent, below header) ──────────────────────

fn render_search_bar(frame: &mut Frame, area: Rect, app: &mut App) {
    let bg_style = Style::new().bg(Theme::BG_DEEPER);
    frame.render_widget(Block::default().style(bg_style), area);

    let is_focused = app.focus == super::app::Focus::Search;
    let input_width = area.width;
    let input_area = Rect::new(area.x, area.y, input_width, 1);
    app.search_bar_area = input_area;

    let cursor_visible = is_focused;
    let query_display = if app.search_query.is_empty() {
        if is_focused {
            "  Search files...".to_string()
        } else {
            "  Press / to search files".to_string()
        }
    } else {
        format!("  {}", app.search_query)
    };

    let query_style = if is_focused {
        Theme::fg().bg(Theme::BG_DEEPER)
    } else {
        Theme::dim_deeper().bg(Theme::BG_DEEPER)
    };

    let search_icon = if is_focused {
        Span::styled(" 🔍 ", Theme::accent().bg(Theme::BG_DEEPER))
    } else {
        Span::styled("   ", Style::new().bg(Theme::BG_DEEPER))
    };

    let mut spans = vec![
        search_icon,
        Span::styled(query_display, query_style),
    ];

    if cursor_visible {
        spans.push(Span::styled(
            "█",
            Style::new().bg(Theme::BG_DEEPER).fg(Theme::CYAN_BRIGHT).add_modifier(Modifier::DIM),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::new().bg(Theme::BG_DEEPER)),
        input_area,
    );
}

// ── Status bar ─────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let bg = Style::new().bg(Theme::BG_DEEPER);
    frame.render_widget(Block::default().style(bg), area);

    let focus_hint = match app.focus {
        super::app::Focus::Timeline => " Timeline ",
        super::app::Focus::Files => " Files ",
        super::app::Focus::Search => " Search ",
    };

    let hints = [
        (" q ", "Quit"),
        (" ? ", "Help"),
        (" p ", "Pick"),
        (" i ", "Info"),
        (" d ", "Diff"),
        (" r ", "Rld"),
        (" s ", "Snap"),
    ];

    let focus_badge = Span::styled(focus_hint, Theme::focus_badge());

    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {} ", key), Theme::keybind()),
                Span::styled(format!(" {}  ", desc), Theme::keydesc()),
            ]
        })
        .collect();

    let info = Span::styled(
        format!(
            " {} ep · {} ph · {} files ",
            app.epochs.len(),
            app.phantoms.len(),
            app.current_entries.len()
        ),
        Theme::dim_deeper().bg(Theme::BG_DEEPER),
    );

    let mut v: Vec<Span> = spans.into_iter().collect();
    v.push(info);
    v.push(focus_badge);

    frame.render_widget(
        Paragraph::new(Line::from(v)).left_aligned().style(bg),
        area,
    );
}

// ── Body: timeline left + detail right ─────────────────────────

fn render_body(app: &mut App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
        .split(area);

    // Fill body background
    frame.render_widget(
        Block::default().style(Style::new().bg(Theme::BG)),
        area,
    );

    render_timeline(app, frame, chunks[0]);
    render_detail(app, frame, chunks[1]);
}

// ── Timeline panel (left) — manually rendered for position control ──

fn render_timeline(app: &mut App, frame: &mut Frame, area: Rect) {
    let border_style = if app.focus == super::app::Focus::Timeline {
        Theme::border_active()
    } else {
        Theme::border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(" Timeline ")
        .title_style(Theme::accent_bold())
        .style(Style::new().bg(Theme::BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Compute scroll
    let item_count = app.timeline_len();
    let epoch_count = app.epochs.len();
    let has_both = epoch_count > 0 && app.phantoms.len() > 0;
    let header_rows = if has_both { 2usize } else { 0 };
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
    app.timeline_list_area = inner;
    app.timeline_scroll = scroll;

    let items = app.timeline_items();
    let mut selected_btn_start_x = 0u16;
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("  No snapshots yet.\n  Click [ ◆ Snap ] or press 's'!")
                .style(Theme::dim())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let render_header = |frame: &mut Frame, inner: Rect, vis_row: u16, text: &str| {
        let header_y = inner.y + vis_row;
        let bg_span = Span::styled(
            " ".repeat(inner.width as usize),
            Theme::section_header(),
        );
        frame.render_widget(
            Paragraph::new(Line::from(bg_span)),
            Rect::new(inner.x, header_y, inner.width, 1),
        );
        let sep = Span::styled(" ── ", Theme::separator());
        let label = Span::styled(text, Theme::dim_deeper());
        let sep2 = Span::styled(" ──", Theme::separator());
        // Build full-width filler
        let filler_w = inner.width as usize - text.len() - 8;
        let filler = Span::styled("─".repeat(filler_w.saturating_sub(0)), Theme::separator());
        let header_x = 0;
        frame.render_widget(
            Paragraph::new(Line::from(vec![sep, label, sep2, filler]))
                .style(Style::new().bg(Theme::BG_DEEPER)),
            Rect::new(inner.x + header_x, header_y, inner.width, 1),
        );
    };

    let mut vis_row = 0u16;

    if has_both && scroll == 0 {
        render_header(frame, inner, vis_row, " Snapshots ");
        vis_row += 1;
    }

    for i in scroll..end {
        if has_both && i == epoch_count {
            render_header(frame, inner, vis_row, " Phantoms ");
            vis_row += 1;
        }

        let item = &items[i];
        let is_selected = i == app.selected_idx;
        let prefix = if is_selected { " ▸" } else { "  " };

        let (display_text, base_style) = match item {
            TimelineItem::Epoch(epoch) => {
                let name = epoch.display_name();
                let msg = epoch.message.as_deref().unwrap_or("");
                let locked = if epoch.is_locked { " 🔒" } else { "" };
                let date = epoch.timestamp.with_timezone(&Local).format("%m/%d %H:%M");
                let text = format!("{} {}  {}  {}{}", prefix, name, date, msg, locked);
                let st = if is_selected {
                    Theme::selected()
                } else if epoch.is_origin {
                    Theme::origin_badge()
                } else if epoch.is_locked {
                    Theme::accent()
                } else {
                    Theme::body()
                };
                (text, st)
            }
            TimelineItem::Phantom(phantom) => {
                let name = phantom.display_name();
                let ttl = phantom.remaining_ttl();
                let hours = ttl.num_hours().max(0);
                let mins = ttl.num_minutes().max(0) % 60;
                let date = phantom.timestamp.with_timezone(&Local).format("%m/%d %H:%M");
                let text = format!("{} ◇ {}  {}  ({}h{}m)", prefix, name, date, hours, mins);
                let st = if is_selected { Theme::selected() } else { Theme::dim() };
                (text, st)
            }
        };

        let mut text_style = base_style;
        if is_selected {
            text_style = text_style.add_modifier(Modifier::BOLD);
        }

        let is_epoch = i < epoch_count;
        let lock_indicator = if is_epoch {
            match item {
                TimelineItem::Epoch(e) => {
                    if e.is_locked { " [L]" } else { " [ ]" }
                }
                _ => "    "
            }
        } else {
            "    "
        };
        let buttons = format!("{}{}", lock_indicator, " [↩] [✕]");
        let buttons_len = buttons.chars().count() as u16;
        let max_label = (inner.width.saturating_sub(buttons_len).saturating_sub(1)) as usize;
        let label = if display_text.width() > max_label {
            let mut w = 0usize;
            let mut char_end = 0usize;
            for (ci, c) in display_text.char_indices() {
                let c_w = c.width().unwrap_or(0);
                if w + c_w > max_label {
                    break;
                }
                w += c_w;
                char_end = ci + c.len_utf8();
            }
            format!("{}…", &display_text[..char_end])
        } else {
            display_text
        };
        let label_width = label.width() as u16;
        let pad = inner.width.saturating_sub(label_width + buttons_len);
        let btn_x = inner.x + label_width + pad;
        if i == app.selected_idx {
            selected_btn_start_x = btn_x;
        }

        // Style the buttons section differently
        let btn_style = if is_selected {
            Theme::timeline_action_hover()
        } else {
            Theme::timeline_action()
        };

        let row_y = inner.y + vis_row;
        let row_rect = Rect::new(inner.x, row_y, inner.width, 1);

        let bg_color = if is_selected { Theme::SURFACE2 } else { Theme::BG };

        // Render label with styling
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                &label,
                text_style.bg(bg_color),
            ))),
            row_rect,
        );

        // Render buttons overlay on the right
        let btn_rect = Rect::new(btn_x, row_y, buttons_len, 1);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&buttons, btn_style.bg(bg_color)))),
            btn_rect,
        );

        vis_row += 1;
    }

    app.timeline_btn_start_x = selected_btn_start_x;
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
            .title_style(Theme::accent_bold())
            .style(Theme::base());
        frame.render_widget(
            Paragraph::new("  No data to display.")
                .style(Theme::dim())
                .block(block),
            area,
        );
        return;
    }

    if app.show_preview {
        render_preview(app, frame, area);
        return;
    }

    if !app.compare_result_lines.is_empty() || app.compare_error.is_some() {
        render_diff_view(app, frame, area);
        return;
    }

    let item = items[app.selected_idx];

    let (title, subtitle, badge) = match item {
        TimelineItem::Epoch(epoch) => {
            let name = epoch.display_name();
            let date = epoch.timestamp.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S");
            let msg = epoch.message.as_deref().unwrap_or("(no message)");
            let badge_text = if epoch.is_locked {
                "  🔒 LOCKED  "
            } else if epoch.is_origin {
                "  ◆ ORIGIN  "
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
            let date = phantom.timestamp.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S");
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

    let border_style = if app.focus == super::app::Focus::Files {
        Theme::border_active()
    } else {
        Theme::border()
    };

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(format!(" {} ", title_text))
        .title_style(Theme::accent_bold())
        .style(Theme::base());

    let inner = header_block.inner(area);
    frame.render_widget(header_block, area);

    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(subtitle, Theme::dim()))),
        vchunks[0],
    );

    app.detail_list_area = vchunks[1];

    // File tree
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
            let marker = if row.is_expanded { "▾" } else { "▸" };
            let dot_color = match row.status {
                Some(crate::types::FileStatus::Added) => Theme::GREEN,
                Some(crate::types::FileStatus::Modified) => Theme::YELLOW,
                Some(crate::types::FileStatus::Deleted) => Theme::RED,
                _ => Theme::FG_SOFT,
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
        let name_style = if is_selected { Theme::selected() } else { status_style };

        let name_max = (vchunks[1].width as usize).saturating_sub(18);
        let display_name = if row.name.len() > name_max {
            format!("{}…", &row.name[..name_max.saturating_sub(1)])
        } else {
            row.name.clone()
        };

        let mut spans = vec![
            Span::styled(format!("{}  {} ", indent, status_char), status_style),
            Span::styled(display_name, name_style),
        ];

        if let Some(s) = row.file_size {
            let size_str = if s > 1_048_576 {
                format!(" {:>7.1} MB", s as f64 / 1_048_576.0)
            } else if s > 1024 {
                format!(" {:>7.1} KB", s as f64 / 1024.0)
            } else {
                format!(" {:>7} B", s)
            };
            spans.push(Span::styled(size_str, Theme::dim_deeper()));
        }

        ListItem::new(Line::from(spans))
    }).collect();

    let adj = adj_sel.saturating_sub(scroll_offset);
    let mut fs = ListState::default().with_selected(Some(adj));
    let list = List::new(file_items)
        .highlight_style(Theme::selected())
        .highlight_symbol("");
    frame.render_stateful_widget(list, vchunks[1], &mut fs);
}

// ── File preview ─────────────────────────────────────────────

fn render_preview(app: &mut App, frame: &mut Frame, area: Rect) {
    let file_path = if let Some(row) = app.visible_rows.get(app.selected_file_idx) {
        row.full_path.clone()
    } else {
        String::new()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_active())
        .title(format!(" Preview — {} ", file_path))
        .title_style(Theme::accent_bold())
        .style(Theme::base());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let max_lines = inner.height.saturating_sub(1) as usize;
    let total = app.preview_content.len();
    let scroll = app.preview_scroll.min(total.saturating_sub(max_lines).max(0));

    let scroll_hint = if total > max_lines {
        let pct = if total > 0 { (scroll * 100) / total } else { 0 };
        format!("  [{}% scrolled] ", pct)
    } else {
        String::new()
    };

    if !scroll_hint.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&scroll_hint, Theme::scroll_hint()))),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        let content_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));
        let lines: Vec<Line> = app.preview_content.iter()
            .skip(scroll)
            .take(content_area.height as usize)
            .map(|l| {
                Line::from(Span::styled(l, Theme::fg()))
            })
            .collect();
        frame.render_widget(Paragraph::new(Text::from(lines)), content_area);
    } else {
        let lines: Vec<Line> = app.preview_content.iter()
            .skip(scroll)
            .take(max_lines)
            .map(|l| {
                Line::from(Span::styled(l, Theme::fg()))
            })
            .collect();
        frame.render_widget(Paragraph::new(Text::from(lines)), inner);
    }
}

// ── Diff view ─────────────────────────────────────────────

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
        .border_style(Theme::border_active())
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

    let scroll_hint = if total > max_lines {
        let pct = if total > 0 {
            (scroll * 100) / total
        } else {
            0
        };
        format!("  [{}% scrolled] ", pct)
    } else {
        String::new()
    };

    let lines: Vec<Line> = app
        .compare_result_lines
        .iter()
        .skip(scroll)
        .take(max_lines)
        .map(|dl| {
            let (marker, style) = match dl.line_type {
                super::app::DiffLineType::Added => ("+", Theme::diff_added()),
                super::app::DiffLineType::Deleted => ("-", Theme::diff_deleted()),
                super::app::DiffLineType::Replaced => ("-", Theme::diff_deleted()),
                super::app::DiffLineType::Same => (" ", Theme::diff_same()),
            };
            Line::from(Span::styled(
                format!(" {} {}", marker, dl.content),
                style,
            ))
        })
        .collect();

    if !scroll_hint.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&scroll_hint, Theme::scroll_hint()))),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        let content_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));
        let content_lines: Vec<Line> = app
            .compare_result_lines
            .iter()
            .skip(scroll)
            .take(content_area.height as usize)
            .map(|dl| {
                let (marker, style) = match dl.line_type {
                    super::app::DiffLineType::Added => ("+", Theme::diff_added()),
                    super::app::DiffLineType::Deleted => ("-", Theme::diff_deleted()),
                    super::app::DiffLineType::Replaced => ("-", Theme::diff_deleted()),
                    super::app::DiffLineType::Same => (" ", Theme::diff_same()),
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

// ── Palin picker overlay ──────────────────────────────────

fn render_picker(app: &mut App, frame: &mut Frame, area: Rect) {
    // Dimmed overlay
    frame.render_widget(
        Block::default().style(Style::new().bg(Theme::BG_DEEPER).fg(Theme::FG)),
        area,
    );

    // Centered popup
    let popup_w = 64u16.min(area.width.saturating_sub(4));
    let popup_h = (app.all_palins.len() as u16 + 6).min(area.height.saturating_sub(4));
    let popup_x = area.x + (area.width - popup_w) / 2;
    let popup_y = area.y + (area.height - popup_h) / 2;
    let popup = Rect::new(popup_x, popup_y, popup_w, popup_h);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::new().fg(Theme::VIOLET))
        .title(" ◆ Palins ")
        .title_style(Theme::accent_bold())
        .style(Style::new().bg(Theme::BG));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Instructions
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Click to switch · [d] to delete · [Esc] to close",
            Theme::dim_deeper(),
        ))),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let list_start_y = inner.y + 1;
    let max_visible = inner.height.saturating_sub(1) as usize;
    let scroll = if app.picker_selected >= max_visible {
        app.picker_selected.saturating_sub(max_visible).saturating_add(1)
    } else {
        0
    };

    let visible_palins = app.all_palins.iter()
        .enumerate()
        .skip(scroll)
        .take(max_visible);

    for (i, (name, path_str)) in visible_palins {
        let is_current = name == &app.palin.name;
        let is_selected = i == app.picker_selected;
        let row_y = list_start_y + (i - scroll) as u16;
        let row_rect = Rect::new(inner.x, row_y, inner.width, 1);

        let bg_color = if is_selected { Theme::SURFACE2 } else { Theme::BG };
        let name_style = if is_current {
            Theme::picker_current()
        } else if is_selected {
            Style::new().fg(Theme::YELLOW_SOFT).add_modifier(Modifier::BOLD)
        } else {
            Theme::fg().bg(Theme::BG)
        };

        let max_path_w = (inner.width as usize).saturating_sub(name.len() + 22);
        let path_display = if path_str.len() > max_path_w {
            format!("…{}", &path_str[path_str.len().saturating_sub(max_path_w.saturating_sub(1))..])
        } else {
            path_str.clone()
        };

        let delete_btn = if is_current { "" } else { " [✕]" };
        let current_tag = if is_current { " ◆" } else { "  " };

        let mut row_text = format!("{}{} {}  {}{}",
            if is_selected { "▸" } else { " " },
            current_tag,
            name,
            path_display,
            delete_btn
        );

        if row_text.len() > inner.width as usize {
            row_text = format!("{}…", &row_text[..inner.width.saturating_sub(2) as usize]);
        }

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&row_text, name_style.bg(bg_color)))),
            row_rect,
        );
    }

    // Delete confirmation overlay
    if let Some(ref name) = app.picker_confirm_delete {
        let confirm_w = 44u16;
        let confirm_h = 3u16;
        let confirm_x = area.x + (area.width - confirm_w) / 2;
        let confirm_y = area.y + area.height / 2;
        let confirm_area = Rect::new(confirm_x, confirm_y, confirm_w, confirm_h);

        let confirm_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(Theme::RED))
            .title(" Delete Palin? ")
            .title_style(Style::new().fg(Theme::RED).add_modifier(Modifier::BOLD))
            .style(Style::new().bg(Theme::SURFACE));

        let confirm_inner = confirm_block.inner(confirm_area);
        frame.render_widget(confirm_block, confirm_area);

        let confirm_line = format!("    Delete '{}'?    ", name);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                &confirm_line,
                Theme::fg().bg(Theme::SURFACE),
            ))),
            Rect::new(confirm_inner.x, confirm_inner.y, confirm_inner.width, 1),
        );

        let btn_y = confirm_inner.y + 1;
        let mid_x = confirm_inner.x + confirm_inner.width / 2;
        let yes_area = Rect::new(mid_x - 8, btn_y, 6, 1);
        let no_area = Rect::new(mid_x + 2, btn_y, 6, 1);
        app.picker_confirm_yes_area = yes_area;
        app.picker_confirm_no_area = no_area;
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(" [Yes] ", Theme::btn_confirm()))),
            yes_area,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(" [No] ", Theme::btn_deny()))),
            no_area,
        );
    }
}

// ── Output panel (scrollable, with clear button) ───────────

fn render_output_bar(app: &mut App, frame: &mut Frame, area: Rect) {
    // Store the full output area for mouse click detection
    app.output_bar_area = area;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(" Messages ")
        .title_style(Theme::dim().fg(Theme::DIM))
        .style(Style::new().bg(Theme::BG_DEEPER));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let clear_text = " [Clear] ";
    let clear_style = Theme::btn_neutral();
    let clear_area = Rect::new(inner.x + inner.width.saturating_sub(clear_text.len() as u16), inner.y, clear_text.len() as u16, 1);
    app.output_clear_area = clear_area;

    // If there's a pending action, show a confirmation prompt inside the panel
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
        let yes_x = inner.x + prompt_len;
        let no_x = yes_x + yes_text.len() as u16;

        app.confirm_yes_area = Rect::new(yes_x, inner.y, yes_text.len() as u16, 1);
        app.confirm_no_area = Rect::new(no_x, inner.y, no_text.len() as u16, 1);

        let spans = vec![
            Span::styled(prompt, Theme::fg().bg(Theme::BG_DEEPER)),
            Span::styled(yes_text, Theme::btn_confirm()),
            Span::styled(no_text, Theme::btn_deny()),
        ];

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(Theme::BG_DEEPER)),
            inner,
        );
        return;
    }

    // GC confirmation
    if app.pending_gc {
        let prompt = Span::styled(
            "  Run garbage collection?  ",
            Theme::fg().bg(Theme::BG_DEEPER),
        );
        let yes_text = " [ Yes ] ";
        let no_text = " [ No ] ";

        let prompt_len = "  Run garbage collection?  ".len() as u16;
        let yes_x = inner.x + prompt_len;
        let no_x = yes_x + yes_text.len() as u16;

        app.confirm_yes_area = Rect::new(yes_x, inner.y, yes_text.len() as u16, 1);
        app.confirm_no_area = Rect::new(no_x, inner.y, no_text.len() as u16, 1);

        let spans = vec![
            prompt,
            Span::styled(yes_text, Theme::btn_confirm()),
            Span::styled(no_text, Theme::btn_deny()),
        ];

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(Theme::BG_DEEPER)),
            inner,
        );
        return;
    }

    let total = app.command_output.len();
    if total == 0 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  ◆ Palimpsest — [o] focus output · [c] clear · [s] snap · [p] pick · [?] help",
                Theme::dim_deeper().bg(Theme::BG_DEEPER),
            ))),
            inner,
        );
        return;
    }

    // Scrolling for long output
    let max_lines = inner.height.saturating_sub(1) as usize;
    let scroll = app.output_scroll.min(total.saturating_sub(max_lines).max(0));

    // Scroll indicator
    let scroll_hint = if total > max_lines {
        let pct = if total > 0 { (scroll * 100) / total } else { 0 };
        format!("  {} msgs · [{}%] ", total, pct)
    } else {
        String::new()
    };

    // Render clear button on the right of the top line
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(clear_text, clear_style))),
        clear_area,
    );

    if !scroll_hint.is_empty() {
        // Render scroll hint on top line
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(&scroll_hint, Theme::scroll_hint()))),
            Rect::new(inner.x, inner.y, inner.width.saturating_sub(clear_text.len() as u16), 1),
        );
        let content_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(1));
        let lines: Vec<Line> = render_output_lines(app, scroll, content_area.height as usize);
        frame.render_widget(Paragraph::new(Text::from(lines)), content_area);
    } else {
        let lines: Vec<Line> = render_output_lines(app, scroll, max_lines);
        frame.render_widget(Paragraph::new(Text::from(lines)), inner);
    }
}

fn render_output_lines<'a>(app: &'a App, scroll: usize, max_lines: usize) -> Vec<Line<'a>> {
    app.command_output
        .iter()              // newest first (index 0 = newest)
        .skip(scroll)        // skip `scroll` newest messages
        .take(max_lines)     // take next `max_lines` messages
        .rev()               // reverse: oldest-in-window at top, newest at bottom
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
            Line::from(Span::styled(format!(" {}", line), style.bg(Theme::BG_DEEPER)))
        })
        .collect()
}