use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::theme::Theme;

/// Render the help overlay as a centred modal.
pub fn render_help(frame: &mut Frame, area: Rect) {
    // Dimmed overlay
    let overlay = Block::default().style(Style::new().bg(Theme::BG_DEEPER));
    frame.render_widget(Clear, area);
    frame.render_widget(overlay, area);

    // Centered box
    let box_width = 56.min(area.width.saturating_sub(4));
    let box_height = 24.min(area.height.saturating_sub(2)).max(12);
    let x = (area.width.saturating_sub(box_width)) / 2;
    let y = (area.height.saturating_sub(box_height)) / 2;
    let help_area = Rect::new(x, y, box_width, box_height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Theme::border_active())
        .title(" ◆ Help ")
        .title_style(Theme::accent_bold())
        .title_alignment(Alignment::Center)
        .style(Style::new().bg(Theme::BG).fg(Theme::FG));

    let lines = vec![
        Line::from(Span::styled(" Keyboard Shortcuts", Theme::accent_bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / ↓        ", Theme::keybind()),
            Span::styled("Scroll down in timeline", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑        ", Theme::keybind()),
            Span::styled("Scroll up in timeline", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  J / PgDn     ", Theme::keybind()),
            Span::styled("Page down", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  K / PgUp     ", Theme::keybind()),
            Span::styled("Page up", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  g            ", Theme::keybind()),
            Span::styled("Go to first item", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  G / End      ", Theme::keybind()),
            Span::styled("Go to last item", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  Tab / →      ", Theme::keybind()),
            Span::styled("Focus file list", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  ← / Shift+Tab", Theme::keybind()),
            Span::styled("Focus timeline", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  r            ", Theme::keybind()),
            Span::styled("Reload timeline", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  /            ", Theme::keybind()),
            Span::styled("Search files", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  d            ", Theme::keybind()),
            Span::styled("Diff / compare files", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  Enter        ", Theme::keybind()),
            Span::styled("Preview file / expand dir", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  s            ", Theme::keybind()),
            Span::styled("Take snapshot", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  p            ", Theme::keybind()),
            Span::styled("Palin picker", Theme::keydesc()),
        ]),
        Line::from(vec![
            Span::styled("  ? / Esc / q  ", Theme::keybind()),
            Span::styled("Close help / Quit", Theme::keydesc()),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", Theme::dim_deeper())),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, help_area);
}
