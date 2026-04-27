use ratagit_core::PanelFocus;
use ratatui::style::{Color, Modifier, Style};

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
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_warning_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_danger_style() -> Style {
    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
}

pub(crate) fn modal_muted_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(crate) fn modal_footer_style() -> Style {
    Style::default().fg(Color::Blue)
}

pub(crate) fn inactive_panel_style() -> Style {
    Style::default().fg(Color::DarkGray)
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
        RowRole::Muted => Style::default().fg(Color::DarkGray),
        RowRole::BatchSelected => batch_selected_row_style(),
        RowRole::FileStaged => Style::default().fg(Color::Green),
        RowRole::FileUntracked => Style::default().fg(Color::Cyan),
        RowRole::SearchMatch => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        RowRole::CurrentBranch => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        RowRole::Error => Style::default().fg(Color::Red),
        RowRole::Notice => Style::default().fg(Color::Blue),
        RowRole::DiffMeta => Style::default().fg(Color::Cyan),
        RowRole::DiffAdd => Style::default().fg(Color::Green),
        RowRole::DiffRemove => Style::default().fg(Color::Red),
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
