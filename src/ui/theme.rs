use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct UiTheme {
    pub canvas: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub accent: Color,
    pub border_inactive: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub shortcut_bg: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    pub selection_bg: Color,
    pub visual_selection_bg: Color,
    pub diff_added_bg: Color,
    pub diff_removed_bg: Color,
    pub diff_staged_added_bg: Color,
    pub diff_staged_removed_bg: Color,
    pub diff_staged_added_selected_bg: Color,
    pub diff_staged_removed_selected_bg: Color,
    pub diff_staged_added_cursor_bg: Color,
    pub diff_staged_removed_cursor_bg: Color,
    pub diff_added_selected_bg: Color,
    pub diff_removed_selected_bg: Color,
    pub diff_added_cursor_bg: Color,
    pub diff_removed_cursor_bg: Color,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            canvas: Color::Rgb(13, 17, 23),
            surface: Color::Rgb(22, 27, 34),
            surface_raised: Color::Rgb(33, 40, 50),
            accent: Color::Rgb(121, 184, 255),
            border_inactive: Color::Rgb(48, 57, 69),
            text_primary: Color::Rgb(218, 225, 234),
            text_muted: Color::Rgb(139, 153, 171),
            shortcut_bg: Color::Rgb(22, 27, 34),

            success: Color::Rgb(126, 211, 161),
            warning: Color::Rgb(230, 190, 115),
            error: Color::Rgb(245, 141, 148),
            info: Color::Rgb(165, 180, 252),

            selection_bg: Color::Rgb(37, 57, 82),
            visual_selection_bg: Color::Rgb(46, 58, 86),
            diff_added_bg: Color::Rgb(24, 46, 36),
            diff_removed_bg: Color::Rgb(51, 30, 37),
            diff_staged_added_bg: Color::Rgb(33, 67, 48),
            diff_staged_removed_bg: Color::Rgb(73, 36, 45),
            diff_staged_added_selected_bg: Color::Rgb(44, 85, 60),
            diff_staged_removed_selected_bg: Color::Rgb(94, 44, 55),
            diff_staged_added_cursor_bg: Color::Rgb(52, 100, 70),
            diff_staged_removed_cursor_bg: Color::Rgb(111, 52, 64),
            diff_added_selected_bg: Color::Rgb(30, 65, 47),
            diff_removed_selected_bg: Color::Rgb(72, 38, 46),
            diff_added_cursor_bg: Color::Rgb(40, 80, 58),
            diff_removed_cursor_bg: Color::Rgb(88, 44, 54),
        }
    }
}

impl UiTheme {
    pub fn active_highlight(&self) -> Style {
        Style::default()
            .bg(self.selection_bg)
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn inactive_highlight(&self) -> Style {
        Style::default().bg(self.surface_raised)
    }

    pub fn highlight_for(&self, is_active: bool) -> Style {
        if is_active {
            self.active_highlight()
        } else {
            self.inactive_highlight()
        }
    }
}
