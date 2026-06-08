use ratatui::style::{Color, Modifier, Style};

/// Professional dark theme — inspired by modern design systems (shadcn/ui, Tailwind).
///
/// Palette:
/// ── Backgrounds ────────────────────────────────────────
///   `#0A0F1E`  BG_DEEPER   — deepest background level
///   `#0F172A`  BG          — primary background (navy)
///   `#1E293B`  SURFACE     — elevated surface / panel
///   `#273548`  SURFACE2    — hovered/selected row
///   `#334155`  SURFACE_DIM — dimmed surface skeleton
/// ── Foregrounds ────────────────────────────────────────
///   `#F1F5F9`  FG          — primary text (near white)
///   `#CBD5E1`  FG_SOFT     — softer body text
///   `#94A3B8`  DIM         — muted/secondary text
///   `#64748B`  DIM_DEEPER  — placeholder / disabled
///   `#475569`  BORDER      — borders, separators
/// ── Accents ────────────────────────────────────────────
///   `#06B6D4`  CYAN        — primary accent, selection
///   `#22D3EE`  CYAN_BRIGHT — bright highlight
///   `#8B5CF6`  VIOLET      — secondary accent, tags
///   `#A78BFA`  VIOLET_SOFT — soft violet glow
/// ── Semantic ───────────────────────────────────────────
///   `#22C55E`  GREEN       — added / success
///   `#4ADE80`  GREEN_SOFT  — soft success
///   `#F59E0B`  YELLOW      — modified / warning
///   `#FBBF24`  YELLOW_SOFT — soft warning
///   `#EF4444`  RED         — deleted / error
///   `#F87171`  RED_SOFT    — soft error
///   `#F97316`  ORANGE      — GC / actions
pub struct Theme;

impl Theme {
    // ── Backgrounds ──────────────────────────────────────────
    /// Deep navy — main background
    pub const BG: Color = Color::Rgb(0x0F, 0x17, 0x2A);
    /// Even deeper — footer, behind panels
    pub const BG_DEEPER: Color = Color::Rgb(0x0A, 0x0F, 0x1E);
    /// Slate — elevated surfaces, panels
    pub const SURFACE: Color = Color::Rgb(0x1E, 0x29, 0x3B);
    /// Brighter slate — hovered/selected row backgrounds
    pub const SURFACE2: Color = Color::Rgb(0x27, 0x35, 0x48);
    /// Muted slate — dimmed elements
    pub const SURFACE_DIM: Color = Color::Rgb(0x33, 0x41, 0x4D);

    // ── Foregrounds ──────────────────────────────────────────
    /// Primary text — nearly white
    pub const FG: Color = Color::Rgb(0xF1, 0xF5, 0xF9);
    /// Softer body text
    pub const FG_SOFT: Color = Color::Rgb(0xCB, 0xD5, 0xE1);
    /// Dimmed text — secondary info
    pub const DIM: Color = Color::Rgb(0x94, 0xA3, 0xB8);
    /// Deeper dim — placeholder, disabled
    pub const DIM_DEEPER: Color = Color::Rgb(0x64, 0x74, 0x8B);
    /// Borders, separators
    pub const BORDER: Color = Color::Rgb(0x47, 0x55, 0x69);
    /// Brighter border — focus
    #[allow(dead_code)]
    pub const BORDER_FOCUS: Color = Color::Rgb(0x5B, 0x6B, 0x7F);

    // ── Accents ─────────────────────────────────────────────
    /// Cyan — primary accent, selection, interactive
    pub const CYAN: Color = Color::Rgb(0x06, 0xB6, 0xD4);
    /// Bright cyan — highlight
    pub const CYAN_BRIGHT: Color = Color::Rgb(0x22, 0xD3, 0xEE);
    /// Violet — tags, special markers
    pub const VIOLET: Color = Color::Rgb(0x8B, 0x5C, 0xF6);
    /// Soft violet — softer accent
    #[allow(dead_code)]
    pub const VIOLET_SOFT: Color = Color::Rgb(0xA7, 0x8B, 0xFA);

    // ── Semantic ────────────────────────────────────────────
    /// Emerald — success, added files
    pub const GREEN: Color = Color::Rgb(0x22, 0xC5, 0x5E);
    /// Soft green — subtle success
    pub const GREEN_SOFT: Color = Color::Rgb(0x4A, 0xDE, 0x80);
    /// Amber — warnings, modified files
    pub const YELLOW: Color = Color::Rgb(0xF5, 0x9E, 0x0B);
    /// Soft amber — subtle warning
    pub const YELLOW_SOFT: Color = Color::Rgb(0xFB, 0xBF, 0x24);
    /// Rose — destructive, deleted files, errors
    pub const RED: Color = Color::Rgb(0xEF, 0x44, 0x44);
    /// Soft rose — subtle error
    pub const RED_SOFT: Color = Color::Rgb(0xF8, 0x71, 0x71);
    /// Orange — GC, action buttons
    pub const ORANGE: Color = Color::Rgb(0xF9, 0x73, 0x16);

    // ── Compound Styles — Text / Base ───────────────────────

    /// Default text on dark bg
    pub fn base() -> Style {
        Style::new().bg(Self::BG).fg(Self::FG)
    }

    /// Soft body text
    #[allow(dead_code)]
    pub fn body() -> Style {
        Style::new().bg(Self::BG).fg(Self::FG_SOFT)
    }

    /// Dimmed / secondary text
    pub fn dim() -> Style {
        Style::new().bg(Self::BG).fg(Self::DIM)
    }

    /// Deeper dim for placeholders
    pub fn dim_deeper() -> Style {
        Style::new().bg(Self::BG).fg(Self::DIM_DEEPER)
    }

    /// Cyan accent
    pub fn accent() -> Style {
        Style::new().bg(Self::BG).fg(Self::CYAN)
    }

    /// Bold accent
    pub fn accent_bold() -> Style {
        Self::accent().add_modifier(Modifier::BOLD)
    }

    /// Default foreground (no bg override)
    pub fn fg() -> Style {
        Style::new().fg(Self::FG)
    }

    /// Soft foreground (no bg override)
    pub fn fg_soft() -> Style {
        Style::new().fg(Self::FG_SOFT)
    }

    // ── Compound Styles — Surfaces / Panels ─────────────────

    /// Panel surface background
    pub fn panel() -> Style {
        Style::new().bg(Self::SURFACE).fg(Self::FG)
    }

    /// Panel surface with dim text
    pub fn panel_dim() -> Style {
        Style::new().bg(Self::SURFACE).fg(Self::DIM)
    }

    /// Elevated / hovered row background
    pub fn elevated() -> Style {
        Style::new().bg(Self::SURFACE2).fg(Self::FG)
    }

    /// Selected item in lists
    pub fn selected() -> Style {
        Style::new()
            .bg(Self::SURFACE2)
            .fg(Self::CYAN_BRIGHT)
            .add_modifier(Modifier::BOLD)
    }

    // ── Header / Status Bar ─────────────────────────────────

    /// Header bar — cyan bg, dark text, bold
    pub fn header() -> Style {
        Style::new()
            .bg(Self::CYAN)
            .fg(Self::BG_DEEPER)
            .add_modifier(Modifier::BOLD)
    }

    /// Status bar — deep bg, dim text
    pub fn status_bar() -> Style {
        Style::new()
            .bg(Self::BG_DEEPER)
            .fg(Self::DIM)
    }

    /// Keybinding badge in status bar
    pub fn keybind() -> Style {
        Style::new()
            .bg(Self::SURFACE2)
            .fg(Self::CYAN_BRIGHT)
            .add_modifier(Modifier::BOLD)
    }

    /// Keybinding description
    pub fn keydesc() -> Style {
        Style::new()
            .bg(Self::BG_DEEPER)
            .fg(Self::FG_SOFT)
    }

    /// Focus badge
    pub fn focus_badge() -> Style {
        Style::new()
            .bg(Self::VIOLET)
            .fg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    // ── Borders ─────────────────────────────────────────────

    /// Default panel border
    pub fn border() -> Style {
        Style::new().fg(Self::BORDER)
    }

    /// Active / focused border
    pub fn border_active() -> Style {
        Style::new().fg(Self::CYAN)
    }

    // ── Badges ──────────────────────────────────────────────

    /// Origin badge — green foreground, bold
    pub fn origin_badge() -> Style {
        Style::new().bg(Self::BG).fg(Self::GREEN).add_modifier(Modifier::BOLD)
    }

    /// Tag badge
    pub fn tag_badge() -> Style {
        Style::new().bg(Self::VIOLET).fg(Self::FG)
    }

    /// Lock badge — red fg, bold
    pub fn lock_badge() -> Style {
        Style::new().fg(Self::RED).add_modifier(Modifier::BOLD)
    }

    // ── Buttons ─────────────────────────────────────────────

    /// Primary action button (e.g. Snap)
    pub fn btn_primary() -> Style {
        Style::new()
            .bg(Self::GREEN)
            .fg(Self::BG_DEEPER)
            .add_modifier(Modifier::BOLD)
    }

    /// Secondary action button (e.g. Info)
    pub fn btn_secondary() -> Style {
        Style::new()
            .bg(Self::CYAN)
            .fg(Self::BG_DEEPER)
            .add_modifier(Modifier::BOLD)
    }

    /// Warning action button (e.g. GC)
    pub fn btn_warning() -> Style {
        Style::new()
            .bg(Self::ORANGE)
            .fg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Neutral action button (e.g. Rename)
    pub fn btn_neutral() -> Style {
        Style::new()
            .bg(Self::SURFACE2)
            .fg(Self::YELLOW_SOFT)
            .add_modifier(Modifier::BOLD)
    }

    /// Confirm button (Yes)
    pub fn btn_confirm() -> Style {
        Style::new()
            .bg(Self::GREEN)
            .fg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Deny / cancel button (No)
    pub fn btn_deny() -> Style {
        Style::new()
            .bg(Self::RED)
            .fg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    // ── File Status ─────────────────────────────────────────

    /// File added
    pub fn added() -> Style {
        Style::new().fg(Self::GREEN)
    }

    /// File modified
    pub fn modified() -> Style {
        Style::new().fg(Self::YELLOW)
    }

    /// File deleted
    pub fn deleted() -> Style {
        Style::new().fg(Self::RED)
    }

    /// File unchanged
    pub fn unchanged() -> Style {
        Style::new().fg(Self::DIM_DEEPER)
    }

    // ── Diff View ───────────────────────────────────────────

    /// Added line background
    pub fn diff_added() -> Style {
        Style::new()
            .bg(Color::Rgb(0x16, 0x2E, 0x1B))
            .fg(Self::GREEN_SOFT)
    }

    /// Deleted line background
    pub fn diff_deleted() -> Style {
        Style::new()
            .bg(Color::Rgb(0x2E, 0x14, 0x14))
            .fg(Self::RED_SOFT)
    }

    /// Replaced line in diff
    pub fn diff_replaced() -> Style {
        Style::new()
            .bg(Color::Rgb(0x2E, 0x1E, 0x0A))
            .fg(Self::YELLOW_SOFT)
    }

    /// Same / unchanged line
    pub fn diff_same() -> Style {
        Style::new().bg(Self::BG).fg(Self::DIM)
    }

    // ── Misc ────────────────────────────────────────────────

    /// Command prompt "> " style
    pub fn prompt() -> Style {
        Style::new().fg(Self::CYAN).add_modifier(Modifier::BOLD)
    }

    /// Separator line
    pub fn separator() -> Style {
        Style::new().fg(Self::BORDER).add_modifier(Modifier::DIM)
    }

    /// Scroll indicator
    pub fn scroll_hint() -> Style {
        Style::new().bg(Self::BG_DEEPER).fg(Self::DIM_DEEPER)
    }

    /// Section header in lists
    pub fn section_header() -> Style {
        Style::new()
            .bg(Self::BG_DEEPER)
            .fg(Self::DIM_DEEPER)
            .add_modifier(Modifier::DIM)
    }

    /// Timeline action button (restore/delete)
    pub fn timeline_action() -> Style {
        Style::new()
            .bg(Self::SURFACE)
            .fg(Self::DIM)
    }

    /// Timeline action button hover
    pub fn timeline_action_hover() -> Style {
        Style::new()
            .bg(Self::SURFACE)
            .fg(Self::CYAN_BRIGHT)
            .add_modifier(Modifier::BOLD)
    }

    /// Pick a palin entry style
    pub fn picker_current() -> Style {
        Style::new().fg(Self::GREEN_SOFT).add_modifier(Modifier::BOLD)
    }
}
