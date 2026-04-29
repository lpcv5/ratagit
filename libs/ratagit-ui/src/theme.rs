use ratagit_core::PanelFocus;
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UiTheme {
    pub(crate) accent: Color,
    pub(crate) selection_text: Color,
    pub(crate) selection: Color,
    pub(crate) batch_selection_text: Color,
    pub(crate) batch_selection: Color,
    pub(crate) muted: Color,
    pub(crate) danger: Color,
    pub(crate) warning: Color,
    pub(crate) success: Color,
    pub(crate) info: Color,
    pub(crate) modal: ModalTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModalTheme {
    pub(crate) active: Color,
    pub(crate) text: Option<Color>,
    pub(crate) dim: Color,
    pub(crate) background: Option<Color>,
    pub(crate) surface_text: Color,
    pub(crate) surface: Color,
    pub(crate) border: Color,
    pub(crate) danger: Color,
    pub(crate) warning: Color,
    pub(crate) success: Color,
    pub(crate) scrim: Option<Color>,
}

const DEFAULT_THEME_INDEX: usize = 0;

pub(crate) const MAIN_UI_THEME: UiTheme = UiTheme {
    accent: Color::Yellow,
    selection_text: Color::Black,
    selection: Color::Yellow,
    batch_selection_text: Color::White,
    batch_selection: Color::Blue,
    muted: Color::DarkGray,
    danger: Color::Red,
    warning: Color::Yellow,
    success: Color::Green,
    info: Color::Cyan,
    modal: ModalTheme {
        active: Color::Cyan,
        text: None,
        dim: Color::DarkGray,
        background: None,
        surface_text: Color::White,
        surface: Color::Blue,
        border: Color::DarkGray,
        danger: Color::Red,
        warning: Color::Yellow,
        success: Color::Green,
        scrim: None,
    },
};

pub(crate) const TOKYO_NIGHT_THEME: UiTheme = UiTheme {
    accent: Color::Rgb(0xe0, 0xaf, 0x68),
    selection_text: Color::Rgb(0x1a, 0x1b, 0x26),
    selection: Color::Rgb(0xe0, 0xaf, 0x68),
    batch_selection_text: Color::Rgb(0xc0, 0xca, 0xf5),
    batch_selection: Color::Rgb(0x7a, 0xa2, 0xf7),
    muted: Color::Rgb(0x56, 0x5f, 0x89),
    danger: Color::Rgb(0xf7, 0x76, 0x8e),
    warning: Color::Rgb(0xe0, 0xaf, 0x68),
    success: Color::Rgb(0x9e, 0xce, 0x6a),
    info: Color::Rgb(0x7a, 0xa2, 0xf7),
    modal: ModalTheme {
        active: Color::Rgb(0x7a, 0xa2, 0xf7),
        text: Some(Color::Rgb(0xc0, 0xca, 0xf5)),
        dim: Color::Rgb(0x56, 0x5f, 0x89),
        background: Some(Color::Rgb(0x1a, 0x1b, 0x26)),
        surface_text: Color::Rgb(0xc0, 0xca, 0xf5),
        surface: Color::Rgb(0x24, 0x28, 0x3b),
        border: Color::Rgb(0x3b, 0x42, 0x61),
        danger: Color::Rgb(0xf7, 0x76, 0x8e),
        warning: Color::Rgb(0xe0, 0xaf, 0x68),
        success: Color::Rgb(0x9e, 0xce, 0x6a),
        scrim: Some(Color::Rgb(0x16, 0x1b, 0x2d)),
    },
};

pub(crate) const BUILT_IN_THEMES: [UiTheme; 2] = [MAIN_UI_THEME, TOKYO_NIGHT_THEME];

pub(crate) fn current_theme() -> &'static UiTheme {
    &BUILT_IN_THEMES[DEFAULT_THEME_INDEX]
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
pub(crate) const ICON_SEARCH_MATCH: &str = "";
pub(crate) const ICON_STASH: &str = "";

pub fn focused_panel_style() -> Style {
    Style::default()
        .fg(current_theme().accent)
        .add_modifier(Modifier::BOLD)
}

pub fn selected_row_style() -> Style {
    Style::default()
        .fg(current_theme().selection_text)
        .bg(current_theme().selection)
        .add_modifier(Modifier::BOLD)
}

pub fn batch_selected_row_style() -> Style {
    Style::default()
        .fg(current_theme().batch_selection_text)
        .bg(current_theme().batch_selection)
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
    style_with_optional_fg(Style::default(), current_theme().modal.text)
}

pub(crate) fn modal_background_style() -> Style {
    style_with_optional_bg(modal_text_style(), current_theme().modal.background)
}

pub(crate) fn modal_scrim_style() -> Style {
    style_with_optional_bg(
        Style::default().fg(current_theme().modal.dim),
        current_theme().modal.scrim,
    )
}

pub(crate) fn modal_border_style() -> Style {
    Style::default().fg(current_theme().modal.border)
}

pub(crate) fn modal_selected_row_style() -> Style {
    Style::default()
        .fg(current_theme().modal.surface_text)
        .bg(current_theme().modal.surface)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_muted_style() -> Style {
    Style::default().fg(current_theme().modal.dim)
}

pub(crate) fn modal_footer_style() -> Style {
    Style::default().fg(current_theme().modal.dim)
}

pub(crate) fn inactive_panel_style() -> Style {
    Style::default().fg(current_theme().muted)
}

pub(crate) fn loading_spinner_style() -> Style {
    Style::default()
        .fg(current_theme().warning)
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
        LoadingSpotlightTone::Dim => Style::default().fg(current_theme().muted),
        LoadingSpotlightTone::Mid => Style::default().fg(current_theme().info),
        LoadingSpotlightTone::Bright => Style::default()
            .fg(current_theme().warning)
            .add_modifier(Modifier::BOLD),
    }
}

pub(crate) fn title_badge_style(focused: bool) -> Style {
    let background = if focused {
        current_theme().selection
    } else {
        current_theme().muted
    };
    Style::default()
        .fg(current_theme().selection_text)
        .bg(background)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn row_style(role: RowRole) -> Style {
    match role {
        RowRole::Normal => Style::default(),
        RowRole::Muted => Style::default().fg(current_theme().muted),
        RowRole::BatchSelected => batch_selected_row_style(),
        RowRole::FileStaged => Style::default().fg(current_theme().success),
        RowRole::FileUntracked => Style::default().fg(current_theme().info),
        RowRole::SearchMatch => Style::default()
            .fg(current_theme().warning)
            .add_modifier(Modifier::BOLD),
        RowRole::CurrentBranch => Style::default()
            .fg(current_theme().success)
            .add_modifier(Modifier::BOLD),
        RowRole::Error => Style::default().fg(current_theme().danger),
        RowRole::Notice => Style::default().fg(current_theme().info),
        RowRole::DiffMeta => Style::default().fg(current_theme().info),
        RowRole::DiffAdd => Style::default().fg(current_theme().success),
        RowRole::DiffRemove => Style::default().fg(current_theme().danger),
    }
}

fn style_with_optional_fg(style: Style, color: Option<Color>) -> Style {
    if let Some(color) = color {
        style.fg(color)
    } else {
        style
    }
}

fn style_with_optional_bg(style: Style, color: Option<Color>) -> Style {
    if let Some(color) = color {
        style.bg(color)
    } else {
        style
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
