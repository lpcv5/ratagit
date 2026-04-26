use ratagit_core::{AppState, PanelFocus};

use crate::frame::{RenderedFrame, TerminalSize, normalize_lines, pad_and_truncate};
use crate::layout::compute_left_panel_heights;
use crate::panels::{
    PanelLine, panel_title, render_branches_lines, render_commits_lines, render_details_lines,
    render_files_lines, render_log_lines, render_stash_lines, shortcuts_for_state,
};

pub fn render(state: &AppState, size: TerminalSize) -> RenderedFrame {
    let width = size.width.max(1);
    let height = size.height.max(1);
    let mut lines = Vec::with_capacity(height);

    let body_height = height.saturating_sub(1);
    if body_height > 0 {
        lines.extend(render_workspace_rows(state, width, body_height));
    }
    lines.push(pad_and_truncate(shortcuts_for_state(state), width));

    normalize_lines(lines, TerminalSize { width, height })
}

fn render_workspace_rows(state: &AppState, total_width: usize, body_height: usize) -> Vec<String> {
    let separator = " | ";
    let separator_width = separator.len();
    if total_width <= separator_width {
        return vec![" ".repeat(total_width); body_height];
    }

    let (left_width, right_width) = split_columns(total_width - separator_width);
    let left_heights = compute_left_panel_heights(state, body_height, 1);
    let right_heights = split_vertical(body_height, &[70, 30]);

    let left_panels = [
        render_panel(
            panel_title(state, PanelFocus::Files),
            state.focus == PanelFocus::Files,
            left_width,
            left_heights.files,
            render_files_lines(state, left_heights.files.saturating_sub(1)),
        ),
        render_panel(
            panel_title(state, PanelFocus::Branches),
            state.focus == PanelFocus::Branches,
            left_width,
            left_heights.branches,
            render_branches_lines(state, left_heights.branches.saturating_sub(1)),
        ),
        render_panel(
            panel_title(state, PanelFocus::Commits),
            state.focus == PanelFocus::Commits,
            left_width,
            left_heights.commits,
            render_commits_lines(state, left_heights.commits.saturating_sub(1)),
        ),
        render_panel(
            panel_title(state, PanelFocus::Stash),
            state.focus == PanelFocus::Stash,
            left_width,
            left_heights.stash,
            render_stash_lines(state, left_heights.stash.saturating_sub(1)),
        ),
    ]
    .concat();

    let right_panels = [
        render_panel(
            panel_title(state, PanelFocus::Details),
            state.focus == PanelFocus::Details,
            right_width,
            right_heights[0],
            render_details_lines(state, right_heights[0].saturating_sub(1)),
        ),
        render_panel(
            panel_title(state, PanelFocus::Log),
            state.focus == PanelFocus::Log,
            right_width,
            right_heights[1],
            render_log_lines(state, right_heights[1].saturating_sub(1)),
        ),
    ]
    .concat();

    (0..body_height)
        .map(|index| {
            format!(
                "{}{}{}",
                left_panels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(left_width)),
                separator,
                right_panels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(right_width)),
            )
        })
        .collect()
}

fn split_columns(workspace_width: usize) -> (usize, usize) {
    let mut left = workspace_width * 34 / 100;
    if left < 20 {
        left = 20.min(workspace_width);
    }
    if workspace_width.saturating_sub(left) < 20 && workspace_width >= 40 {
        left = workspace_width - 20;
    }
    (left, workspace_width.saturating_sub(left))
}

fn split_vertical(total_height: usize, ratios: &[usize]) -> Vec<usize> {
    if ratios.is_empty() {
        return Vec::new();
    }
    let sum: usize = ratios.iter().sum();
    if sum == 0 {
        return vec![0; ratios.len()];
    }

    let mut values: Vec<usize> = ratios
        .iter()
        .map(|ratio| total_height * ratio / sum)
        .collect();
    let used: usize = values.iter().sum();
    let mut remainder = total_height.saturating_sub(used);
    let mut index = 0usize;
    while remainder > 0 {
        values[index] = values[index].saturating_add(1);
        remainder -= 1;
        index = (index + 1) % values.len();
    }
    values
}

fn render_panel(
    title: &str,
    _focused: bool,
    width: usize,
    height: usize,
    content_lines: Vec<PanelLine>,
) -> Vec<String> {
    if height == 0 {
        return Vec::new();
    }

    let mut lines = Vec::with_capacity(height);
    let header = format!("  {title}");
    lines.push(pad_and_truncate(header, width));

    for line in content_lines.into_iter().take(height.saturating_sub(1)) {
        lines.push(pad_and_truncate(line.text, width));
    }
    while lines.len() < height {
        lines.push(" ".repeat(width));
    }
    lines
}
