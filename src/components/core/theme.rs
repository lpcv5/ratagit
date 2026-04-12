use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders},
};

pub const LIST_HIGHLIGHT_SYMBOL: &str = "▸ ";

/// Dracula Theme Color Palette
pub struct Theme {
    // Panel chrome
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub title_focused: Color,
    pub title_unfocused: Color,

    // Selection states
    pub cursor_highlight_fg: Color,
    pub cursor_highlight_bg: Color,
    pub multi_select_fg: Color,
    pub multi_select_bg: Color,

    // Git status colors
    pub git_added: Color,
    pub git_modified: Color,
    pub git_deleted: Color,
    pub git_renamed: Color,
    pub git_untracked: Color,

    // Diff colors
    pub diff_header: Color,
    pub diff_meta: Color,
    pub diff_hunk: Color,
    pub diff_file: Color,
    pub diff_added: Color,
    pub diff_removed: Color,

    // Accents
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub muted_text: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dracula()
    }
}

impl Theme {
    /// Dracula theme colors
    pub fn dracula() -> Self {
        Self {
            // Panel chrome - Dracula Purple and Comment
            border_focused: Color::Rgb(189, 147, 249), // Purple
            border_unfocused: Color::Rgb(98, 114, 164), // Comment
            title_focused: Color::Rgb(248, 248, 242),  // Foreground
            title_unfocused: Color::Rgb(98, 114, 164), // Comment

            // Selection states
            cursor_highlight_fg: Color::Black,
            cursor_highlight_bg: Color::Rgb(189, 147, 249), // Purple
            multi_select_fg: Color::Rgb(248, 248, 242),     // Foreground
            multi_select_bg: Color::Rgb(68, 71, 90),        // Current Line

            // Git status colors
            git_added: Color::Rgb(80, 250, 123),      // Green
            git_modified: Color::Rgb(241, 250, 140),  // Yellow
            git_deleted: Color::Rgb(255, 85, 85),     // Red
            git_renamed: Color::Rgb(139, 233, 253),   // Cyan
            git_untracked: Color::Rgb(189, 147, 249), // Purple

            // Diff colors
            diff_header: Color::Rgb(139, 233, 253), // Cyan
            diff_meta: Color::Rgb(255, 184, 108),   // Orange
            diff_hunk: Color::Rgb(189, 147, 249),   // Purple
            diff_file: Color::Rgb(139, 233, 253),   // Cyan
            diff_added: Color::Rgb(80, 250, 123),   // Green
            diff_removed: Color::Rgb(255, 85, 85),  // Red

            // Accents
            accent_primary: Color::Rgb(255, 121, 198), // Pink
            accent_secondary: Color::Rgb(189, 147, 249), // Purple
            muted_text: Color::Rgb(98, 114, 164),      // Comment
        }
    }
}

// Global theme instance
static THEME: Theme = Theme {
    border_focused: Color::Rgb(189, 147, 249),
    border_unfocused: Color::Rgb(98, 114, 164),
    title_focused: Color::Rgb(248, 248, 242),
    title_unfocused: Color::Rgb(98, 114, 164),
    cursor_highlight_fg: Color::Black,
    cursor_highlight_bg: Color::Rgb(189, 147, 249),
    multi_select_fg: Color::Rgb(248, 248, 242),
    multi_select_bg: Color::Rgb(68, 71, 90),
    git_added: Color::Rgb(80, 250, 123),
    git_modified: Color::Rgb(241, 250, 140),
    git_deleted: Color::Rgb(255, 85, 85),
    git_renamed: Color::Rgb(139, 233, 253),
    git_untracked: Color::Rgb(189, 147, 249),
    diff_header: Color::Rgb(139, 233, 253),
    diff_meta: Color::Rgb(255, 184, 108),
    diff_hunk: Color::Rgb(189, 147, 249),
    diff_file: Color::Rgb(139, 233, 253),
    diff_added: Color::Rgb(80, 250, 123),
    diff_removed: Color::Rgb(255, 85, 85),
    accent_primary: Color::Rgb(255, 121, 198),
    accent_secondary: Color::Rgb(189, 147, 249),
    muted_text: Color::Rgb(98, 114, 164),
};

pub fn theme() -> &'static Theme {
    &THEME
}

pub fn panel_border_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(THEME.border_focused)
    } else {
        Style::default().fg(THEME.border_unfocused)
    }
}

pub fn panel_title_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default()
            .fg(THEME.title_focused)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(THEME.title_unfocused)
    }
}

pub fn selected_row_style() -> Style {
    Style::default()
        .fg(THEME.cursor_highlight_fg)
        .bg(THEME.cursor_highlight_bg)
        .add_modifier(Modifier::BOLD)
}

pub fn multi_select_row_style() -> Style {
    Style::default()
        .fg(THEME.multi_select_fg)
        .bg(THEME.multi_select_bg)
}

pub fn muted_text_style() -> Style {
    Style::default().fg(THEME.muted_text)
}

pub fn accent_primary_color() -> Color {
    THEME.accent_primary
}

pub fn accent_secondary_color() -> Color {
    THEME.accent_secondary
}

pub fn panel_block(title: impl Into<String>, is_focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(panel_border_style(is_focused))
        .title(Line::styled(title.into(), panel_title_style(is_focused)))
}

#[cfg(test)]
mod tests {
    use super::panel_block;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    #[test]
    fn panel_block_renders_rounded_corners() {
        let area = Rect::new(0, 0, 12, 4);
        let mut buffer = Buffer::empty(area);

        panel_block("Panel", true).render(area, &mut buffer);

        assert_eq!(buffer[(0, 0)].symbol(), "╭");
        assert_eq!(buffer[(11, 0)].symbol(), "╮");
        assert_eq!(buffer[(0, 3)].symbol(), "╰");
        assert_eq!(buffer[(11, 3)].symbol(), "╯");
    }
}
