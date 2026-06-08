use ratatui::style::{Color, Modifier, Style};

/// Theme: black background, white foreground, minimal bright accents.
#[allow(dead_code)]
pub struct Theme;

impl Theme {
    /// Base background for all panels
    pub const BG: Color = Color::Black;

    /// Primary text colour
    pub const FG: Color = Color::White;

    /// Dimmed / less important text
    pub const DIM: Color = Color::DarkGray;

    /// Bright accent — used for headings, logos, selection indicators
    pub const ACCENT: Color = Color::Cyan;

    /// File status – added
    pub const ADDED: Color = Color::Green;

    /// File status – modified
    pub const MODIFIED: Color = Color::Yellow;

    /// File status – deleted
    pub const DELETED: Color = Color::Red;

    /// Locked icon / tag badge
    pub const LOCK: Color = Color::Magenta;

    /// Highlighted row in selected list
    pub const SELECTED_BG: Color = Color::DarkGray;

    /// Border colour
    pub const BORDER: Color = Color::Gray;

    // ── Compound styles ──────────────────────────────────────────

    pub fn base() -> Style {
        Style::new().bg(Self::BG).fg(Self::FG)
    }

    pub fn dim() -> Style {
        Style::new().bg(Self::BG).fg(Self::DIM)
    }

    pub fn accent() -> Style {
        Style::new().bg(Self::BG).fg(Self::ACCENT)
    }

    pub fn accent_bold() -> Style {
        Self::accent().add_modifier(Modifier::BOLD)
    }

    /// Default foreground text
    pub fn fg() -> Style {
        Style::new().fg(Self::FG)
    }

    pub fn header() -> Style {
        Style::new()
            .bg(Self::ACCENT)
            .fg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected() -> Style {
        Style::new()
            .bg(Self::SELECTED_BG)
            .fg(Self::FG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border() -> Style {
        Style::new().fg(Self::BORDER)
    }

    pub fn added() -> Style {
        Style::new().fg(Self::ADDED)
    }

    pub fn modified() -> Style {
        Style::new().fg(Self::MODIFIED)
    }

    pub fn deleted() -> Style {
        Style::new().fg(Self::DELETED)
    }

    pub fn tag() -> Style {
        Style::new().bg(Self::LOCK).fg(Self::BG)
    }
}
