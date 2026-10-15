use crate::app::InputMode;
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::components::organisms::{
    draw_branches_panel, draw_commits_panel, draw_files_panel, draw_stash_panel,
};
use crate::ui::panels::{
    render_branch_editor, render_branch_switch_confirm, render_command_log, render_command_palette,
    render_commit_all_confirm, render_commit_editor, render_diff_panel, render_keybindings,
    render_shortcut_bar, render_stash_editor,
};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::Style,
    widgets::Block,
    Frame,
};

/// Collapsed height for stash/log panels when not focused (in lines)
const COLLAPSED_HEIGHT: u16 = 3;
const SHORTCUT_BAR_HEIGHT: u16 = 1;
const PANEL_BORDER_ROWS: u16 = 2;
const LEFT_PANEL_DEFAULT_SPLIT: [u16; 3] = [40, 30, 30];
const LEFT_PANEL_FOCUS_SPLIT: [u16; 3] = [60, 20, 20];

pub fn render_layout(
    frame: &mut Frame,
    snapshot: &AppStateSnapshot<'_>,
    state: &mut crate::ui::RenderState,
) {
    let size = frame.area();
    let theme = UiTheme::default();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.canvas)),
        size,
    );
    let roomy = size.width >= 100 && size.height >= 28;
    let gap = u16::from(roomy);
    let footer_height = if roomy { 2 } else { SHORTCUT_BAR_HEIGHT };
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(footer_height)])
        .split(size);
    let workspace = vertical[0].inner(Margin::new(u16::from(roomy), 0));

    // Horizontal split: Left (30%) | Right (70%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .spacing(u16::from(roomy))
        .split(workspace);

    let left_h = horizontal[0].height;
    let panel_layout = snapshot.side_panel_layout_props();
    let stash_focused = panel_layout.stash_focused;

    // Stash panel: COLLAPSED_HEIGHT when not focused, else share remaining space
    let stash_h = if stash_focused {
        // Give stash ~25% of left height
        (left_h / 4).max(COLLAPSED_HEIGHT).min(left_h)
    } else {
        COLLAPSED_HEIGHT.min(left_h)
    };

    // Files and branches/commits share the remaining height
    let top_h = left_h.saturating_sub(stash_h + gap);
    let top_rect = ratatui::layout::Rect {
        height: top_h,
        ..horizontal[0]
    };

    // Default split: files (40%) + branches (30%) + commits (30%)
    let default_left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[0]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[1]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[2]),
        ])
        .spacing(gap)
        .split(top_rect);

    let focus_index = panel_layout.focus_index;
    let focus_item_count = panel_layout.focus_item_count;

    let should_expand_focused_left_panel = focus_index
        .map(|idx| {
            let visible_rows = usize::from(
                default_left_panels[idx]
                    .height
                    .saturating_sub(PANEL_BORDER_ROWS),
            )
            .max(1);
            focus_item_count > visible_rows
        })
        .unwrap_or(false);

    let left_constraints = if should_expand_focused_left_panel {
        let split = match focus_index {
            Some(0) => [
                LEFT_PANEL_FOCUS_SPLIT[0],
                LEFT_PANEL_FOCUS_SPLIT[1],
                LEFT_PANEL_FOCUS_SPLIT[2],
            ],
            Some(1) => [
                LEFT_PANEL_FOCUS_SPLIT[1],
                LEFT_PANEL_FOCUS_SPLIT[0],
                LEFT_PANEL_FOCUS_SPLIT[2],
            ],
            Some(2) => [
                LEFT_PANEL_FOCUS_SPLIT[1],
                LEFT_PANEL_FOCUS_SPLIT[2],
                LEFT_PANEL_FOCUS_SPLIT[0],
            ],
            _ => LEFT_PANEL_DEFAULT_SPLIT,
        };
        vec![
            Constraint::Percentage(split[0]),
            Constraint::Percentage(split[1]),
            Constraint::Percentage(split[2]),
        ]
    } else {
        vec![
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[0]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[1]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[2]),
        ]
    };

    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints(left_constraints)
        .spacing(gap)
        .split(top_rect);

    let stash_area = ratatui::layout::Rect {
        y: horizontal[0].bottom().saturating_sub(stash_h),
        height: stash_h,
        ..horizontal[0]
    };

    draw_files_panel(
        frame,
        left_panels[0],
        snapshot.files_panel_props(),
        &mut state.files,
    );
    draw_branches_panel(
        frame,
        left_panels[1],
        snapshot.branches_panel_props(),
        &mut state.branches,
    );
    draw_commits_panel(
        frame,
        left_panels[2],
        snapshot.commits_panel_props(),
        &mut state.commits,
    );
    draw_stash_panel(
        frame,
        stash_area,
        snapshot.stash_panel_props(),
        &mut state.stash,
    );

    // Right side: diff + command log
    // Command log collapses to COLLAPSED_HEIGHT when stash is not the concern
    let right_h = horizontal[1].height;
    let log_h = COLLAPSED_HEIGHT.min(right_h);
    let diff_h = right_h.saturating_sub(log_h + gap);

    let diff_area = ratatui::layout::Rect {
        height: diff_h,
        ..horizontal[1]
    };
    let log_area = ratatui::layout::Rect {
        y: horizontal[1].bottom().saturating_sub(log_h),
        height: log_h,
        ..horizontal[1]
    };

    render_diff_panel(
        frame,
        diff_area,
        snapshot.diff_presentation().view(),
        &mut state.diff,
    );
    render_command_log(frame, log_area, snapshot.command_log_props());
    let animation_frame = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis()
        / 100) as usize;
    render_shortcut_bar(
        frame,
        vertical[1],
        snapshot.shortcut_bar_props(animation_frame),
    );
    // The container selects the active overlay and supplies only its display data.
    if snapshot.input_mode != Some(InputMode::Keybindings) {
        state.help = Default::default();
    }
    match snapshot.input_mode {
        Some(InputMode::CreateBranch) => render_branch_editor(
            frame,
            crate::ui::panels::props::BranchEditorProps {
                name: snapshot.input_buffer,
                error: snapshot.input_error,
            },
        ),
        Some(InputMode::CommitEditor) => {
            render_commit_editor(frame, snapshot.commit_editor_props())
        }
        Some(InputMode::StashEditor) => render_stash_editor(frame, snapshot.stash_editor_props()),
        Some(InputMode::BranchSwitchConfirm) => {
            render_branch_switch_confirm(frame, snapshot.branch_switch_confirm_props())
        }
        Some(InputMode::CommitAllConfirm) => {
            render_commit_all_confirm(frame, snapshot.commit_all_confirm_props())
        }
        Some(InputMode::CommandPalette) => {
            render_command_palette(frame, snapshot.command_palette_props())
        }
        Some(InputMode::Keybindings) => {
            render_keybindings(frame, snapshot.keybindings_props(), &mut state.help)
        }
        _ => {}
    }
}
