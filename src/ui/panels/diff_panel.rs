//! Application adapter for the reusable diff component.
use crate::app::SidePanel;
use crate::git::DiffLine;
use crate::ui::components::diff::{
    DiffPosition, DiffSelection, DiffSelectionMode, DiffStatus, DiffView, DiffViewModel,
    DiffViewState, DiffViewportAnchor,
};
use crate::ui::theme::UiTheme;
use ratatui::{layout::Rect, Frame};

pub struct DiffViewProps<'a> {
    pub hunks: crate::app::HunkView<'a>,
    /// Display metadata can outlive the actionable patch during a file switch.
    pub preview_patch: Option<&'a crate::git::patch::Patch>,
    pub lines: &'a [DiffLine],
    pub scroll: usize,
    pub active_panel: SidePanel,
    pub is_loading: bool,
}

/// Translate domain selection and command availability once at the panel boundary.
pub fn render_diff_panel(
    frame: &mut Frame,
    area: Rect,
    props: DiffViewProps<'_>,
    state: &mut DiffViewState,
) {
    let interaction = props.hunks.interaction;
    let marked_lines = props.hunks.marked_lines();
    let marked_hunks = props.hunks.marked_hunks();
    let (current, total) = if interaction.focused && props.hunks.ready {
        props.hunks.cursor_position()
    } else {
        (0, 0)
    };
    let model = DiffViewModel {
        lines: props.lines,
        patch: props.preview_patch,
        scroll: props.scroll,
        viewport_anchor: interaction
            .cursor_relocation
            .map(|anchor| DiffViewportAnchor {
                revision: anchor.revision,
                previous_row: anchor.previous_row,
                row: anchor.row,
            }),
        status: status(&props),
        empty_message: empty_hint(&props),
        selection: DiffSelection {
            focused: interaction.focused,
            mode: if interaction.line_mode {
                DiffSelectionMode::Line
            } else {
                DiffSelectionMode::Hunk
            },
            hunk_cursor: interaction.cursor,
            line_cursor: interaction.line_cursor,
            marked_hunks: &marked_hunks,
            marked_lines: &marked_lines,
            range_active: if interaction.line_mode {
                interaction.range_anchor.is_some()
            } else {
                interaction.hunk_anchor.is_some()
            },
            position: DiffPosition { current, total },
        },
    };
    let theme = UiTheme::default();
    frame.render_stateful_widget(DiffView::new(model, &theme), area, state);
}

fn status(props: &DiffViewProps<'_>) -> DiffStatus {
    if props.is_loading || !props.hunks.ready {
        DiffStatus::Loading
    } else if props.hunks.interaction.pending_write.is_some() {
        DiffStatus::Applying
    } else if props.hunks.cursor_is_read_only() {
        DiffStatus::ReadOnly
    } else {
        DiffStatus::Idle
    }
}

fn empty_hint(props: &DiffViewProps<'_>) -> &'static str {
    match props.hunks.interaction.side {
        Some(crate::app::DiffSide::Unstaged) => "No unstaged changes",
        Some(crate::app::DiffSide::Staged) => "No staged changes",
        None => match props.active_panel {
            SidePanel::Files => "Select a file to view diff",
            SidePanel::LocalBranches => "Select a branch to view details",
            SidePanel::Commits => "Select a commit/file to view diff",
            SidePanel::Stash => "Select a stash entry/file to view diff",
        },
    }
}
