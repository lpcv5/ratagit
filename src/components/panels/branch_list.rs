use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{layout::Rect, Frame};

use crate::app::CachedData;
use crate::components::core::{render_branches, SimpleListPanel};
use crate::components::{Component, Intent};

use super::CommitPanel;

enum BranchMode {
    List,
    CommitsSub { panel: CommitPanel },
}

pub struct BranchListPanel {
    list: SimpleListPanel,
    mode: BranchMode,
}

impl BranchListPanel {
    pub fn new() -> Self {
        Self {
            list: SimpleListPanel::new("Branches", render_branches),
            mode: BranchMode::List,
        }
    }

    pub fn state_mut(&mut self) -> &mut ratatui::widgets::ListState {
        self.list.state_mut()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.list.selected_index()
    }

    pub fn show_branch_commits(&mut self) {
        self.mode = BranchMode::CommitsSub { panel: CommitPanel::new() };
    }
}

impl Default for BranchListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for BranchListPanel {
    fn handle_event(&mut self, event: &Event, data: &CachedData) -> Intent {
        match &mut self.mode {
            BranchMode::List => self.list.handle_event(event, data),
            BranchMode::CommitsSub { panel } => {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => {
                                if panel.is_list_multi_select_active() {
                                    panel.clear_list_multi_select();
                                    return Intent::RefreshPanelDetail;
                                }
                                self.mode = BranchMode::List;
                                return Intent::SwitchFocus(crate::app::Panel::Branches);
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                let len = data.commits.len();
                                let state = panel.state_mut();
                                let next = state.selected().unwrap_or(0).saturating_add(1).min(len.saturating_sub(1));
                                state.select(Some(next));
                                panel.refresh_list_multi_range(&data.commits);
                                return Intent::RefreshPanelDetail;
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                let state = panel.state_mut();
                                let next = state.selected().unwrap_or(0).saturating_sub(1);
                                state.select(Some(next));
                                panel.refresh_list_multi_range(&data.commits);
                                return Intent::RefreshPanelDetail;
                            }
                            _ => {}
                        }
                    }
                }
                panel.handle_event(event, data)
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        match &mut self.mode {
            BranchMode::List => self.list.render(frame, area, is_focused, data),
            BranchMode::CommitsSub { panel } => panel.render(frame, area, is_focused, data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::git_ops::CommitEntry;
    use crate::components::core::MultiSelectableList;

    fn key(code: KeyCode) -> Event {
        Event::Key(crossterm::event::KeyEvent::new(
            code,
            crossterm::event::KeyModifiers::NONE,
        ))
    }

    fn commit(id: &str) -> CommitEntry {
        CommitEntry {
            short_id: id.chars().take(8).collect(),
            id: id.to_string(),
            summary: id.to_string(),
            body: None,
            author: "tester <tester@example.com>".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn v_mode_j_extends_branch_sub_commit_range() {
        let mut branch_panel = BranchListPanel::new();
        branch_panel.show_branch_commits();

        let data = CachedData {
            commits: vec![commit("a"), commit("b"), commit("c")],
            ..CachedData::default()
        };

        let intent_v = branch_panel.handle_event(&key(KeyCode::Char('v')), &data);
        assert!(matches!(intent_v, Intent::RefreshPanelDetail));

        let intent_j = branch_panel.handle_event(&key(KeyCode::Char('j')), &data);
        assert!(matches!(intent_j, Intent::RefreshPanelDetail));

        match &branch_panel.mode {
            BranchMode::CommitsSub { panel } => {
                assert_eq!(panel.multi_selected_count(), 2);
            }
            BranchMode::List => panic!("expected commits sub panel mode"),
        }
    }

    #[test]
    fn esc_exits_v_mode_before_leaving_sub_panel() {
        let mut branch_panel = BranchListPanel::new();
        branch_panel.show_branch_commits();

        let data = CachedData {
            commits: vec![commit("a"), commit("b"), commit("c")],
            ..CachedData::default()
        };

        let intent_v = branch_panel.handle_event(&key(KeyCode::Char('v')), &data);
        assert!(matches!(intent_v, Intent::RefreshPanelDetail));

        let intent_esc_first = branch_panel.handle_event(&key(KeyCode::Esc), &data);
        assert!(matches!(intent_esc_first, Intent::RefreshPanelDetail));
        match &branch_panel.mode {
            BranchMode::CommitsSub { panel } => {
                assert!(!panel.is_list_multi_select_active());
                assert_eq!(panel.multi_selected_count(), 0);
            }
            BranchMode::List => panic!("expected commits sub panel mode"),
        }

        let intent_esc_second = branch_panel.handle_event(&key(KeyCode::Esc), &data);
        assert!(matches!(
            intent_esc_second,
            Intent::SwitchFocus(crate::app::Panel::Branches)
        ));
        match &branch_panel.mode {
            BranchMode::List => {}
            BranchMode::CommitsSub { .. } => panic!("expected list mode"),
        }
    }
}
