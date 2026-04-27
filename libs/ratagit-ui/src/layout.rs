use ratagit_core::{AppContext, PanelFocus};

use crate::frame::TerminalSize;
use crate::panels::left_panel_content_len;

const LEFT_PANEL_COUNT: usize = 4;
const LEFT_PANEL_WEIGHTS: [usize; LEFT_PANEL_COUNT] = [36, 22, 26, 16];
const NON_STASH_WEIGHTS: [usize; 3] = [36, 22, 26];
const FILES_INDEX: usize = 0;
const BRANCHES_INDEX: usize = 1;
const COMMITS_INDEX: usize = 2;
const STASH_INDEX: usize = 3;
const DETAILS_SCROLL_NUMERATOR: usize = 2;
const DETAILS_SCROLL_DENOMINATOR: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeftPanelHeights {
    pub(crate) files: usize,
    pub(crate) branches: usize,
    pub(crate) commits: usize,
    pub(crate) stash: usize,
}

impl LeftPanelHeights {
    fn zero() -> Self {
        Self {
            files: 0,
            branches: 0,
            commits: 0,
            stash: 0,
        }
    }

    fn from_array(values: [usize; LEFT_PANEL_COUNT]) -> Self {
        Self {
            files: values[FILES_INDEX],
            branches: values[BRANCHES_INDEX],
            commits: values[COMMITS_INDEX],
            stash: values[STASH_INDEX],
        }
    }

    #[cfg(test)]
    fn as_array(self) -> [usize; LEFT_PANEL_COUNT] {
        [self.files, self.branches, self.commits, self.stash]
    }
}

pub(crate) fn compute_left_panel_heights(
    state: &AppContext,
    total_height: usize,
    panel_chrome_height: usize,
) -> LeftPanelHeights {
    if total_height == 0 {
        return LeftPanelHeights::zero();
    }

    let chrome_budget = panel_chrome_height.saturating_mul(LEFT_PANEL_COUNT);
    if total_height < chrome_budget {
        let fallback = split_by_weights(total_height, &LEFT_PANEL_WEIGHTS);
        return LeftPanelHeights::from_array(fallback);
    }

    let available_content = total_height.saturating_sub(chrome_budget);
    let mut content = split_by_weights(available_content, &LEFT_PANEL_WEIGHTS);
    let minimum_readable = if available_content >= LEFT_PANEL_COUNT {
        1
    } else {
        0
    };
    let stash_collapsed_lines = if available_content > 0 { 1 } else { 0 };

    if state.ui.focus != PanelFocus::Stash {
        collapse_stash_when_unfocused(
            &mut content,
            stash_collapsed_lines,
            minimum_readable,
            available_content,
        );
    }

    if let Some(focus_index) = left_panel_index(state.ui.focus)
        && focus_index != STASH_INDEX
    {
        let content_lengths = [
            left_panel_content_len(state, PanelFocus::Files),
            left_panel_content_len(state, PanelFocus::Branches),
            left_panel_content_len(state, PanelFocus::Commits),
            left_panel_content_len(state, PanelFocus::Stash),
        ];
        let overflow = content_lengths[focus_index].saturating_sub(content[focus_index]);
        if overflow > 0 {
            let donor_order = donor_order_for_focus(focus_index);
            let donor_mins = [
                minimum_readable,
                minimum_readable,
                minimum_readable,
                stash_collapsed_lines,
            ];
            borrow_lines_evenly(
                &mut content,
                focus_index,
                overflow,
                &donor_order,
                &donor_mins,
            );
        }
    }

    let mut heights = [0usize; LEFT_PANEL_COUNT];
    for (index, lines) in content.iter().enumerate() {
        heights[index] = lines.saturating_add(panel_chrome_height);
    }
    LeftPanelHeights::from_array(heights)
}

pub fn details_scroll_lines_for_terminal_size(size: TerminalSize) -> usize {
    let details_content_height = details_content_lines_for_terminal_size(size);
    if details_content_height == 0 {
        0
    } else {
        (details_content_height * DETAILS_SCROLL_NUMERATOR / DETAILS_SCROLL_DENOMINATOR).max(1)
    }
}

pub fn details_content_lines_for_terminal_size(size: TerminalSize) -> usize {
    let body_height = size.height.max(1).saturating_sub(1);
    let details_panel_height = split_by_weights(body_height, &[70, 30])[0];
    details_panel_height.saturating_sub(2)
}

pub fn focused_left_panel_content_lines_for_terminal_size(
    state: &AppContext,
    size: TerminalSize,
) -> usize {
    let body_height = size.height.max(1).saturating_sub(1);
    let heights = compute_left_panel_heights(state, body_height, 2);
    match state.ui.focus {
        PanelFocus::Files => heights.files,
        PanelFocus::Branches => heights.branches,
        PanelFocus::Commits => heights.commits,
        PanelFocus::Stash => heights.stash,
        PanelFocus::Details | PanelFocus::Log => 0,
    }
    .saturating_sub(2)
}

fn collapse_stash_when_unfocused(
    content: &mut [usize; LEFT_PANEL_COUNT],
    stash_target: usize,
    minimum_readable: usize,
    available_content: usize,
) {
    if content[STASH_INDEX] > stash_target {
        let freed = content[STASH_INDEX] - stash_target;
        content[STASH_INDEX] = stash_target;
        let distributed = split_by_weights(freed, &NON_STASH_WEIGHTS);
        content[FILES_INDEX] = content[FILES_INDEX].saturating_add(distributed[0]);
        content[BRANCHES_INDEX] = content[BRANCHES_INDEX].saturating_add(distributed[1]);
        content[COMMITS_INDEX] = content[COMMITS_INDEX].saturating_add(distributed[2]);
        return;
    }

    if content[STASH_INDEX] < stash_target {
        let needed = stash_target - content[STASH_INDEX];
        let donor_mins = [
            minimum_readable,
            minimum_readable,
            minimum_readable,
            if available_content > 0 { 1 } else { 0 },
        ];
        borrow_lines_evenly(
            content,
            STASH_INDEX,
            needed,
            &[FILES_INDEX, BRANCHES_INDEX, COMMITS_INDEX],
            &donor_mins,
        );
    }
}

fn borrow_lines_evenly(
    content: &mut [usize; LEFT_PANEL_COUNT],
    target_index: usize,
    mut needed: usize,
    donor_order: &[usize],
    donor_mins: &[usize; LEFT_PANEL_COUNT],
) {
    while needed > 0 {
        let mut progress = false;
        for donor in donor_order {
            if needed == 0 {
                break;
            }
            let donor_index = *donor;
            if content[donor_index] > donor_mins[donor_index] {
                content[donor_index] -= 1;
                content[target_index] += 1;
                needed -= 1;
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
}

fn donor_order_for_focus(focus_index: usize) -> [usize; 3] {
    match focus_index {
        FILES_INDEX => [BRANCHES_INDEX, COMMITS_INDEX, STASH_INDEX],
        BRANCHES_INDEX => [COMMITS_INDEX, STASH_INDEX, FILES_INDEX],
        COMMITS_INDEX => [STASH_INDEX, FILES_INDEX, BRANCHES_INDEX],
        _ => [FILES_INDEX, BRANCHES_INDEX, COMMITS_INDEX],
    }
}

fn left_panel_index(panel: PanelFocus) -> Option<usize> {
    match panel {
        PanelFocus::Files => Some(FILES_INDEX),
        PanelFocus::Branches => Some(BRANCHES_INDEX),
        PanelFocus::Commits => Some(COMMITS_INDEX),
        PanelFocus::Stash => Some(STASH_INDEX),
        PanelFocus::Details | PanelFocus::Log => None,
    }
}

fn split_by_weights<const N: usize>(total: usize, weights: &[usize; N]) -> [usize; N] {
    let weight_sum: usize = weights.iter().sum();
    if weight_sum == 0 {
        return [0usize; N];
    }

    let mut values = [0usize; N];
    for (index, weight) in weights.iter().enumerate() {
        values[index] = total.saturating_mul(*weight) / weight_sum;
    }

    let used: usize = values.iter().sum();
    let mut remainder = total.saturating_sub(used);
    let mut index = 0usize;
    while remainder > 0 {
        values[index] = values[index].saturating_add(1);
        remainder -= 1;
        index = (index + 1) % N;
    }

    values
}

#[cfg(test)]
mod tests {
    use ratagit_core::{Action, AppContext, GitResult, PanelFocus, update};
    use ratagit_testkit::{fixture_commit, fixture_many_files};

    use super::*;

    fn state_with_many_files() -> AppContext {
        let mut state = AppContext::default();
        let _commands = update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(fixture_many_files())),
        );
        state
    }

    #[test]
    fn stash_collapses_to_single_content_line_when_unfocused() {
        let state = AppContext::default();
        let heights = compute_left_panel_heights(&state, 40, 2);

        assert_eq!(heights.stash.saturating_sub(2), 1);
    }

    #[test]
    fn stash_focus_restores_default_height() {
        let mut focused_stash = AppContext::default();
        focused_stash.ui.focus = PanelFocus::Stash;
        focused_stash.ui.last_left_focus = PanelFocus::Stash;
        let focused = compute_left_panel_heights(&focused_stash, 40, 2);

        focused_stash.ui.focus = PanelFocus::Files;
        focused_stash.ui.last_left_focus = PanelFocus::Files;
        let collapsed = compute_left_panel_heights(&focused_stash, 40, 2);

        assert!(focused.stash > collapsed.stash);
        assert_eq!(focused.as_array().iter().sum::<usize>(), 40);
    }

    #[test]
    fn focused_left_panel_expands_when_content_overflows() {
        let mut state = state_with_many_files();
        state.ui.focus = PanelFocus::Details;
        let baseline = compute_left_panel_heights(&state, 40, 2);

        state.ui.focus = PanelFocus::Files;
        state.ui.last_left_focus = PanelFocus::Files;
        let expanded = compute_left_panel_heights(&state, 40, 2);

        assert!(expanded.files > baseline.files);
        assert!(expanded.branches <= baseline.branches);
        assert!(expanded.commits <= baseline.commits);
        assert_eq!(expanded.stash, baseline.stash);
    }

    #[test]
    fn commit_files_subpanel_keeps_parent_commits_height() {
        let mut state = AppContext::default();
        state.ui.focus = PanelFocus::Commits;
        state.ui.last_left_focus = PanelFocus::Commits;
        state.repo.commits.items = (0..30)
            .map(|index| fixture_commit(&format!("{index:07x}"), &format!("commit {index}")))
            .collect();
        let parent_height = compute_left_panel_heights(&state, 24, 2).commits;

        state.ui.commits.files.active = true;
        state.work.commit_files.commit_files_loading = false;
        let subpanel_height = compute_left_panel_heights(&state, 24, 2).commits;

        assert_eq!(subpanel_height, parent_height);
    }

    #[test]
    fn round_robin_borrowing_is_deterministic_and_respects_minimums() {
        let mut content = [8usize, 6, 6, 4];
        let donor_mins = [1usize, 1, 1, 1];

        borrow_lines_evenly(&mut content, FILES_INDEX, 12, &[1, 2, 3], &donor_mins);

        assert_eq!(content, [20, 1, 2, 1]);
    }

    #[test]
    fn details_scroll_step_is_two_fifths_of_details_content_height() {
        assert_eq!(
            details_scroll_lines_for_terminal_size(TerminalSize {
                width: 100,
                height: 30,
            }),
            7
        );
        assert_eq!(
            details_scroll_lines_for_terminal_size(TerminalSize {
                width: 80,
                height: 14,
            }),
            3
        );
    }
}
