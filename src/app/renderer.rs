use std::collections::HashSet;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};

use crate::components::Component;

use super::ui_state::Panel;
use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum UiSlot {
    Files,
    Branches,
    Commits,
    Stash,
    MainView,
    Log,
}

const LEFT_FILES_INDEX: usize = 0;
const LEFT_BRANCHES_INDEX: usize = 1;
const LEFT_COMMITS_INDEX: usize = 2;
const LEFT_STASH_INDEX: usize = 3;
const LEFT_DYNAMIC_MIN_HEIGHT: u16 = 18;
const STASH_COLLAPSED_HEIGHT: u16 = 3;
const FOCUSED_PANEL_MIN_HEIGHT: u16 = 7;

impl App {
    pub(super) fn render(&mut self, frame: &mut Frame) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
            .split(frame.area());

        let left_heights =
            compute_left_panel_heights(columns[0].height, self.state.ui_state.active_panel);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints(left_heights.map(Constraint::Length))
            .split(columns[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(12), Constraint::Length(8)])
            .split(columns[1]);

        let mut rendered_slots = HashSet::new();

        prepare_slot(frame, left[0], UiSlot::Files, &mut rendered_slots);
        self.state.components.file_list_panel.render(
            frame,
            left[0],
            self.state.ui_state.active_panel == Panel::Files,
            &self.state.data_cache,
        );

        prepare_slot(frame, left[1], UiSlot::Branches, &mut rendered_slots);
        self.state.components.branch_list_panel.render(
            frame,
            left[1],
            self.state.ui_state.active_panel == Panel::Branches,
            &self.state.data_cache,
        );

        prepare_slot(frame, left[2], UiSlot::Commits, &mut rendered_slots);
        self.state.components.commit_panel.render(
            frame,
            left[2],
            self.state.ui_state.active_panel == Panel::Commits,
            &self.state.data_cache,
        );

        prepare_slot(frame, left[3], UiSlot::Stash, &mut rendered_slots);
        self.state.components.stash_list_panel.render(
            frame,
            left[3],
            self.state.ui_state.active_panel == Panel::Stash,
            &self.state.data_cache,
        );

        prepare_slot(frame, right[0], UiSlot::MainView, &mut rendered_slots);
        self.state.components.main_view_panel.render(
            frame,
            right[0],
            self.state.ui_state.active_panel == Panel::MainView,
            &self.state.data_cache,
        );

        prepare_slot(frame, right[1], UiSlot::Log, &mut rendered_slots);
        self.state.components.log_panel.render(
            frame,
            right[1],
            self.state.ui_state.active_panel == Panel::Log,
            &self.state.data_cache,
        );

        // Render modal if active
        if let Some(ref modal) = self.state.active_modal {
            modal.render(frame, frame.area());
        }
    }
}

fn prepare_slot(frame: &mut Frame, area: Rect, slot: UiSlot, rendered_slots: &mut HashSet<UiSlot>) {
    debug_assert!(
        rendered_slots.insert(slot),
        "UI slot rendered more than once in the same frame: {:?}",
        slot
    );
    frame.render_widget(Clear, area);
}

pub(super) fn compute_left_panel_heights(total_height: u16, active_panel: Panel) -> [u16; 4] {
    if total_height < LEFT_DYNAMIC_MIN_HEIGHT {
        return distribute_weighted(total_height, [28, 24, 28, 20]);
    }

    let mut heights = [0_u16; 4];

    match active_panel {
        Panel::Files | Panel::Branches | Panel::Commits => {
            let focused_index = match active_panel {
                Panel::Files => LEFT_FILES_INDEX,
                Panel::Branches => LEFT_BRANCHES_INDEX,
                Panel::Commits => LEFT_COMMITS_INDEX,
                _ => unreachable!(),
            };
            heights[LEFT_STASH_INDEX] = STASH_COLLAPSED_HEIGHT;
            let remaining_for_top_three = total_height.saturating_sub(STASH_COLLAPSED_HEIGHT);
            let focused_height = (remaining_for_top_three / 2)
                .max(FOCUSED_PANEL_MIN_HEIGHT)
                .min(remaining_for_top_three);
            heights[focused_index] = focused_height;
            let remaining = remaining_for_top_three.saturating_sub(focused_height);
            let mut non_focused = [0_usize; 2];
            let mut wi = 0;
            for index in [LEFT_FILES_INDEX, LEFT_BRANCHES_INDEX, LEFT_COMMITS_INDEX] {
                if index != focused_index {
                    non_focused[wi] = index;
                    wi += 1;
                }
            }
            distribute_evenly_into(remaining, &non_focused, &mut heights);
        }
        Panel::Stash => {
            let stash_height = (total_height / 2)
                .max(FOCUSED_PANEL_MIN_HEIGHT)
                .min(total_height);
            heights[LEFT_STASH_INDEX] = stash_height;
            let remaining = total_height.saturating_sub(stash_height);
            distribute_evenly_into(
                remaining,
                &[LEFT_FILES_INDEX, LEFT_BRANCHES_INDEX, LEFT_COMMITS_INDEX],
                &mut heights,
            );
        }
        Panel::MainView | Panel::Log => {
            heights[LEFT_STASH_INDEX] = STASH_COLLAPSED_HEIGHT;
            let remaining = total_height.saturating_sub(STASH_COLLAPSED_HEIGHT);
            distribute_evenly_into(
                remaining,
                &[LEFT_FILES_INDEX, LEFT_BRANCHES_INDEX, LEFT_COMMITS_INDEX],
                &mut heights,
            );
        }
    }

    heights
}

fn distribute_evenly_into(total: u16, target_indices: &[usize], heights: &mut [u16; 4]) {
    if target_indices.is_empty() {
        return;
    }
    let len = target_indices.len() as u16;
    let base = total / len;
    let mut remainder = total % len;
    for index in target_indices {
        let mut value = base;
        if remainder > 0 {
            value += 1;
            remainder -= 1;
        }
        heights[*index] = value;
    }
}

fn distribute_weighted(total: u16, weights: [u16; 4]) -> [u16; 4] {
    let mut heights = [0_u16; 4];
    let sum: u16 = weights.iter().sum();
    let mut consumed = 0_u16;
    for (idx, weight) in weights.into_iter().enumerate() {
        let value = total.saturating_mul(weight) / sum;
        heights[idx] = value;
        consumed = consumed.saturating_add(value);
    }
    let mut remainder = total.saturating_sub(consumed);
    for value in &mut heights {
        if remainder == 0 {
            break;
        }
        *value = value.saturating_add(1);
        remainder -= 1;
    }
    heights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_heights_expand_focused_files_panel_and_collapse_stash() {
        let heights = compute_left_panel_heights(30, Panel::Files);
        assert_eq!(heights[LEFT_STASH_INDEX], STASH_COLLAPSED_HEIGHT);
        assert!(heights[LEFT_FILES_INDEX] > heights[LEFT_BRANCHES_INDEX]);
        assert!(heights[LEFT_FILES_INDEX] > heights[LEFT_COMMITS_INDEX]);
        assert_eq!(heights.iter().sum::<u16>(), 30);
    }

    #[test]
    fn left_heights_expand_stash_when_stash_is_focused() {
        let heights = compute_left_panel_heights(30, Panel::Stash);
        assert_eq!(heights[LEFT_STASH_INDEX], 15);
        assert_eq!(heights[LEFT_FILES_INDEX], 5);
        assert_eq!(heights[LEFT_BRANCHES_INDEX], 5);
        assert_eq!(heights[LEFT_COMMITS_INDEX], 5);
        assert_eq!(heights.iter().sum::<u16>(), 30);
    }

    #[test]
    fn left_heights_keep_stash_collapsed_when_right_side_is_focused() {
        let heights = compute_left_panel_heights(25, Panel::MainView);
        assert_eq!(heights[LEFT_STASH_INDEX], STASH_COLLAPSED_HEIGHT);
        let top_three = [
            heights[LEFT_FILES_INDEX],
            heights[LEFT_BRANCHES_INDEX],
            heights[LEFT_COMMITS_INDEX],
        ];
        let max = *top_three.iter().max().unwrap();
        let min = *top_three.iter().min().unwrap();
        assert!(max - min <= 1);
        assert_eq!(heights.iter().sum::<u16>(), 25);
    }

    #[test]
    fn left_heights_fall_back_to_legacy_weights_on_small_terminal() {
        let heights = compute_left_panel_heights(10, Panel::Files);
        assert_eq!(heights, [3, 3, 2, 2]);
        assert_eq!(heights.iter().sum::<u16>(), 10);
    }

    #[test]
    fn cycle_selection_does_not_wrap_forward() {
        use ratatui::widgets::ListState;
        let mut state = ListState::default();
        state.select(Some(2));
        super::super::intent_executor::cycle_selection(&mut state, 3, 1);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn cycle_selection_does_not_wrap_backward() {
        use ratatui::widgets::ListState;
        let mut state = ListState::default();
        state.select(Some(0));
        super::super::intent_executor::cycle_selection(&mut state, 3, -1);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn left_panel_navigation_stays_on_left_panels_only() {
        use super::super::input_handler::{next_left_panel, previous_left_panel};
        assert_eq!(next_left_panel(Panel::Files), Panel::Branches);
        assert_eq!(next_left_panel(Panel::Stash), Panel::Files);
        assert_eq!(next_left_panel(Panel::MainView), Panel::Files);
        assert_eq!(previous_left_panel(Panel::Files), Panel::Stash);
        assert_eq!(previous_left_panel(Panel::Branches), Panel::Files);
        assert_eq!(previous_left_panel(Panel::Log), Panel::Stash);
    }

    #[test]
    fn dedupe_targets_prefers_parent_directory() {
        use crate::backend::DiffTarget;
        use crate::shared::path_utils::dedupe_targets_parent_first;
        let targets = vec![
            DiffTarget {
                path: "src".to_string(),
                is_dir: true,
            },
            DiffTarget {
                path: "src/main.rs".to_string(),
                is_dir: false,
            },
            DiffTarget {
                path: "README.md".to_string(),
                is_dir: false,
            },
        ];
        let deduped = dedupe_targets_parent_first(&targets);
        assert_eq!(deduped.len(), 2);
        assert!(deduped.iter().any(|t| t.path == "src" && t.is_dir));
        assert!(deduped.iter().any(|t| t.path == "README.md" && !t.is_dir));
    }
}
