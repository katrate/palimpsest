use ratatui::style::{Color, Modifier, Style};

/// Professional dark theme inspired by ui-ux-pro-max-skill design system.
///
/// Palette:
/// - Deep navy background (`#0F172A`) — reduces eye strain, modern
/// - Slate surfaces (`#1E293B`) — panels, selected rows
/// - Cyan primary (`#06B6D4`) — accent, selection highlights
/// - Emerald success (`#22C55E`) — added files, positive states
/// - Amber warning (`#F59E0B`) — modified files
/// - Rose destructive (`#EF4444`) — deleted files, errors
/// - Violet accent (`#8B5CF6`) — tags, special badges
pub struct Theme;

impl Theme {
    // ── Backgrounds ──────────────────────────────────────────
    /// Deep navy — main background (WCAG AAA with white text)
    pub const BG: Color = Color::Rgb(0x0F, 0x17, 0x2A);
    /// Slate — secondary surfaces, panels
    pub const SURFACE: Color = Color::Rgb(0x1E, 0x29, 0x3B);
    /// Muted slate — dimmed elements (reserved)
    #[allow(dead_code)]
    pub const SURFACE_DIM: Color = Color::Rgb(0x33, 0x41, 0x4D);

    // ── Foreground ──────────────────────────────────────────
    /// Primary text — nearly white
    pub const FG: Color = Color::Rgb(0xF1, 0xF5, 0xF9);
    /// Dimmed text — subtle
    pub const DIM: Color = Color::Rgb(0x94, 0xA3, 0xB8);
    /// Very dim — borders, separators
    pub const BORDER: Color = Color::Rgb(0x47, 0x55, 0x69);

    // ── Accents ─────────────────────────────────────────────
    /// Cyan — primary accent, selection, interactive
    pub const CYAN: Color = Color::Rgb(0x06, 0xB6, 0xD4);
    /// Violet — tags, special markers
    pub const VIOLET: Color = Color::Rgb(0x8B, 0x5C, 0xF6);
    /// Emerald — success, added files
    pub const GREEN: Color = Color::Rgb(0x22, 0xC5, 0x5E);
    /// Amber — warnings, modified files
    pub const YELLOW: Color = Color::Rgb(0xF5, 0x9E, 0x0B);
    /// Rose — destructive, deleted files, errors
    pub const RED: Color = Color::Rgb(0xEF, 0x44, 0x44);

    // ── Compound Styles ─────────────────────────────────────

    /// Base text on dark bg
    pub fn base() -> Style {
        Style::new().bg(Self::BG).fg(Self::FG)
    }

    /// Dimmed / secondary text
    pub fn dim() -> Style {
        Style::new().bg(Self::BG).fg(Self::DIM)
    }

    /// Cyan accent text
    pub fn accent() -> Style {
        Style::new().bg(Self::BG).fg(Self::CYAN)
    }

    /// Bold accent
    pub fn accent_bold() -> Style {
        Self::accent().add_modifier(Modifier::BOLD)
    }

    /// Default foreground text (no bg override)
    pub fn fg() -> Style {
        Style::new().fg(Self::FG)
    }

    /// Header bar — cyan bg, dark text, bold
    pub fn header() -> Style {
        Style::new()
            .bg(Self::CYAN)
            .fg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Selected item in lists — slate bg, white fg, bold
    pub fn selected() -> Style {
        Style::new()
            .bg(Self::SURFACE)
            .fg(Self::FG)
            .add_modifier(Modifier::BOLD)
    }

    /// Origin badge — green foreground, not bg (too heavy)
    pub fn origin_badge() -> Style {
        Style::new().bg(Self::BG).fg(Self::GREEN).add_modifier(Modifier::BOLD)
    }

    /// Panel border style
    pub fn border() -> Style {
        Style::new().fg(Self::BORDER)
    }

    /// Active / focused border
    pub fn border_active() -> Style {
        Style::new().fg(Self::CYAN)
    }

    /// File status — added
    pub fn added() -> Style {
        Style::new().fg(Self::GREEN)
    }

    /// File status — modified
    pub fn modified() -> Style {
        Style::new().fg(Self::YELLOW)
    }

    /// File status — deleted
    pub fn deleted() -> Style {
        Style::new().fg(Self::RED)
    }

    /// Tag badge style
    #[allow(dead_code)]
    pub fn tag_badge() -> Style {
        Style::new().bg(Self::VIOLET).fg(Self::FG)
    }

    /// Lock badge
    #[allow(dead_code)]
    pub fn lock_badge() -> Style {
        Style::new().bg(Self::RED).fg(Self::FG).add_modifier(Modifier::BOLD)
    }



    /// Command prompt "> " style
    pub fn prompt() -> Style {
        Style::new().fg(Self::CYAN).add_modifier(Modifier::BOLD)
    }

    /// Separator line
    pub fn separator() -> Style {
        Style::new().fg(Self::BORDER).add_modifier(Modifier::DIM)
    }


}
