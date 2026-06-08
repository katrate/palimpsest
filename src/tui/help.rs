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
    // Semi-transparent overlay via Clear
    let overlay = Block::default().style(Style::new().bg(Theme::BG));
    frame.render_widget(Clear, area);
    frame.render_widget(overlay, area);

    // Centered box
    let box_width = 52.min(area.width.saturating_sub(4));
    let box_height = 22.min(area.height.saturating_sub(2)).max(10);
    let x = (area.width.saturating_sub(box_width)) / 2;
    let y = (area.height.saturating_sub(box_height)) / 2;
    let help_area = Rect::new(x, y, box_width, box_height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Theme::accent())
        .title(" ◆ Help ")
        .title_style(Theme::accent_bold())
        .title_alignment(Alignment::Center)
        .style(Theme::base());

    let lines = vec![
        Line::from(Span::styled(" Keyboard Shortcuts", Theme::accent_bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / ↓        ", Theme::accent()),
            Span::styled("Scroll down in timeline", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑        ", Theme::accent()),
            Span::styled("Scroll up in timeline", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  J / PgDn     ", Theme::accent()),
            Span::styled("Page down", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  K / PgUp     ", Theme::accent()),
            Span::styled("Page up", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  g            ", Theme::accent()),
            Span::styled("Go to first item", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  G / End      ", Theme::accent()),
            Span::styled("Go to last item", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  Tab / →      ", Theme::accent()),
            Span::styled("Next file in detail list", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Tab / ←", Theme::accent()),
            Span::styled("Previous file", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  r            ", Theme::accent()),
            Span::styled("Reload timeline", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  ?            ", Theme::accent()),
            Span::styled("Toggle this help", Theme::fg()),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc      ", Theme::accent()),
            Span::styled("Quit", Theme::fg()),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", Theme::dim())),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, help_area);
}
