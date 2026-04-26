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
}

pub(crate) const ICON_BATCH_SELECTED: &str = "✓";
pub(crate) const ICON_BRANCH: &str = "";
pub(crate) const ICON_COMMIT: &str = "";
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

pub(crate) fn inactive_panel_style() -> Style {
    Style::default().fg(Color::DarkGray)
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
    }
}

pub(crate) fn panel_label(panel: PanelFocus) -> &'static str {
    match panel {
        PanelFocus::Files => "[1] 󰈙 Files",
        PanelFocus::Branches => "[2]  Branches",
        PanelFocus::Commits => "[3]  Commits",
        PanelFocus::Stash => "[4]  Stash",
        PanelFocus::Details => "[5]  Details",
        PanelFocus::Log => "[6] 󰌱 Log",
    }
}
