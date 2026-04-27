use ratagit_core::{AppContext, PanelFocus};

use crate::frame::{RenderContext, RenderedFrame, TerminalSize, normalize_lines, pad_and_truncate};
use crate::layout::compute_left_panel_heights;
use crate::panel_projection::{PanelProjection, project_panel};
use crate::panels::shortcuts_for_state_with_context;

pub fn render(state: &AppContext, size: TerminalSize) -> RenderedFrame {
    render_with_context(state, size, RenderContext::default())
}

pub fn render_with_context(
    state: &AppContext,
    size: TerminalSize,
    context: RenderContext,
) -> RenderedFrame {
    let width = size.width.max(1);
    let height = size.height.max(1);
    let mut lines = Vec::with_capacity(height);

    let body_height = height.saturating_sub(1);
    if body_height > 0 {
        lines.extend(render_workspace_rows(state, width, body_height));
    }
    lines.push(pad_and_truncate(
        shortcuts_for_state_with_context(state, context),
        width,
    ));

    normalize_lines(lines, TerminalSize { width, height })
}

fn render_workspace_rows(
    state: &AppContext,
    total_width: usize,
    body_height: usize,
) -> Vec<String> {
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
            left_width,
            left_heights.files,
            project_panel(
                state,
                PanelFocus::Files,
                left_heights.files.saturating_sub(1),
            ),
        ),
        render_panel(
            left_width,
            left_heights.branches,
            project_panel(
                state,
                PanelFocus::Branches,
                left_heights.branches.saturating_sub(1),
            ),
        ),
        render_panel(
            left_width,
            left_heights.commits,
            project_panel(
                state,
                PanelFocus::Commits,
                left_heights.commits.saturating_sub(1),
            ),
        ),
        render_panel(
            left_width,
            left_heights.stash,
            project_panel(
                state,
                PanelFocus::Stash,
                left_heights.stash.saturating_sub(1),
            ),
        ),
    ]
    .concat();

    let right_panels = [
        render_panel(
            right_width,
            right_heights[0],
            project_panel(
                state,
                PanelFocus::Details,
                right_heights[0].saturating_sub(1),
            ),
        ),
        render_panel(
            right_width,
            right_heights[1],
            project_panel(state, PanelFocus::Log, right_heights[1].saturating_sub(1)),
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

fn render_panel(width: usize, height: usize, projection: PanelProjection) -> Vec<String> {
    if height == 0 {
        return Vec::new();
    }

    let mut lines = Vec::with_capacity(height);
    let header = format!("  {}", projection.legacy_text_title);
    lines.push(pad_and_truncate(header, width));

    for line in projection.lines.into_iter().take(height.saturating_sub(1)) {
        lines.push(pad_and_truncate(line.text(), width));
    }
    while lines.len() < height {
        lines.push(" ".repeat(width));
    }
    lines
}
