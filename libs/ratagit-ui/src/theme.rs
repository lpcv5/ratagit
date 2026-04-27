use ratagit_core::PanelFocus;
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UiTheme {
    pub(crate) modal: ModalTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModalTheme {
    pub(crate) active: Color,
    pub(crate) text: Color,
    pub(crate) dim: Color,
    pub(crate) background: Color,
    pub(crate) surface: Color,
    pub(crate) border: Color,
    pub(crate) danger: Color,
    pub(crate) warning: Color,
    pub(crate) success: Color,
    pub(crate) scrim: Color,
}

pub(crate) const DEFAULT_THEME: UiTheme = UiTheme {
    modal: ModalTheme {
        active: Color::Rgb(0x7a, 0xa2, 0xf7),
        text: Color::Rgb(0xc0, 0xca, 0xf5),
        dim: Color::Rgb(0x56, 0x5f, 0x89),
        background: Color::Rgb(0x1a, 0x1b, 0x26),
        surface: Color::Rgb(0x24, 0x28, 0x3b),
        border: Color::Rgb(0x3b, 0x42, 0x61),
        danger: Color::Rgb(0xf7, 0x76, 0x8e),
        warning: Color::Rgb(0xe0, 0xaf, 0x68),
        success: Color::Rgb(0x9e, 0xce, 0x6a),
        scrim: Color::Rgb(0x16, 0x1b, 0x2d),
    },
};

pub(crate) fn current_theme() -> &'static UiTheme {
    &DEFAULT_THEME
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowRole {
    Normal,
    Muted,
    BatchSelected,
    FileStaged,
    FileUntracked,
    SearchMatch,
    CurrentBranch,
    Error,
    Notice,
    DiffMeta,
    DiffAdd,
    DiffRemove,
}

pub(crate) const ICON_BATCH_SELECTED: &str = "✓";
pub(crate) const ICON_BRANCH: &str = "";
pub(crate) const ICON_DIRECTORY_CLOSED: &str = "";
pub(crate) const ICON_DIRECTORY_OPEN: &str = "";
pub(crate) const ICON_FILE: &str = "";
pub(crate) const ICON_FILE_STAGED: &str = "";
pub(crate) const ICON_FILE_UNTRACKED: &str = "";
pub(crate) const ICON_SEARCH_MATCH: &str = "";
pub(crate) const ICON_STASH: &str = "";

pub fn focused_panel_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn selected_row_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn batch_selected_row_style() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_info_style() -> Style {
    Style::default()
        .fg(current_theme().modal.active)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_warning_style() -> Style {
    Style::default()
        .fg(current_theme().modal.warning)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_danger_style() -> Style {
    Style::default()
        .fg(current_theme().modal.danger)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_text_style() -> Style {
    Style::default().fg(current_theme().modal.text)
}

pub(crate) fn modal_background_style() -> Style {
    modal_text_style().bg(current_theme().modal.background)
}

pub(crate) fn modal_scrim_style() -> Style {
    Style::default()
        .fg(current_theme().modal.dim)
        .bg(current_theme().modal.scrim)
}

pub(crate) fn modal_border_style() -> Style {
    Style::default().fg(current_theme().modal.border)
}

pub(crate) fn modal_selected_row_style() -> Style {
    Style::default()
        .fg(current_theme().modal.text)
        .bg(current_theme().modal.surface)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_active_input_style() -> Style {
    modal_text_style().bg(current_theme().modal.surface)
}

pub(crate) fn modal_muted_style() -> Style {
    Style::default().fg(current_theme().modal.dim)
}

pub(crate) fn modal_footer_style() -> Style {
    Style::default().fg(current_theme().modal.dim)
}

pub(crate) fn inactive_panel_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(crate) fn loading_spinner_style() -> Style {
    Style::default()
        .fg(current_theme().modal.warning)
        .add_modifier(Modifier::BOLD)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadingSpotlightTone {
    Dim,
    Mid,
    Bright,
}

pub(crate) fn loading_text_style(tone: LoadingSpotlightTone) -> Style {
    match tone {
        LoadingSpotlightTone::Dim => Style::default().fg(current_theme().modal.dim),
        LoadingSpotlightTone::Mid => Style::default().fg(current_theme().modal.active),
        LoadingSpotlightTone::Bright => Style::default()
            .fg(current_theme().modal.warning)
            .add_modifier(Modifier::BOLD),
    }
}

pub(crate) fn title_badge_style(focused: bool) -> Style {
    let background = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    Style::default()
        .fg(Color::Black)
        .bg(background)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn row_style(role: RowRole) -> Style {
    match role {
        RowRole::Normal => Style::default(),
        RowRole::Muted => Style::default().fg(current_theme().modal.dim),
        RowRole::BatchSelected => batch_selected_row_style(),
        RowRole::FileStaged => Style::default().fg(current_theme().modal.success),
        RowRole::FileUntracked => Style::default().fg(current_theme().modal.active),
        RowRole::SearchMatch => Style::default()
            .fg(current_theme().modal.warning)
            .add_modifier(Modifier::BOLD),
        RowRole::CurrentBranch => Style::default()
            .fg(current_theme().modal.success)
            .add_modifier(Modifier::BOLD),
        RowRole::Error => Style::default().fg(current_theme().modal.danger),
        RowRole::Notice => Style::default().fg(current_theme().modal.active),
        RowRole::DiffMeta => Style::default().fg(current_theme().modal.active),
        RowRole::DiffAdd => Style::default().fg(current_theme().modal.success),
        RowRole::DiffRemove => Style::default().fg(current_theme().modal.danger),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelLabel {
    pub(crate) badge: &'static str,
    pub(crate) body: &'static str,
}

pub(crate) fn panel_label(panel: PanelFocus) -> PanelLabel {
    match panel {
        PanelFocus::Files => PanelLabel {
            badge: "1",
            body: "󰈙 Files",
        },
        PanelFocus::Branches => PanelLabel {
            badge: "2",
            body: " Branches",
        },
        PanelFocus::Commits => PanelLabel {
            badge: "3",
            body: " Commits",
        },
        PanelFocus::Stash => PanelLabel {
            badge: "4",
            body: " Stash",
        },
        PanelFocus::Details => PanelLabel {
            badge: "5",
            body: " Details",
        },
        PanelFocus::Log => PanelLabel {
            badge: "6",
            body: "󰌱 Log",
        },
    }
}
