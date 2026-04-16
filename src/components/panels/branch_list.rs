use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{layout::Rect, Frame};
use ratatui::buffer::Buffer;

use crate::app::CachedData;
use crate::app::AppState;
use crate::app::events::{AppEvent, GitEvent};
use crate::components::core::{render_branches, SimpleListPanel};
use crate::components::{Component, Intent};
use crate::components::component_v2::ComponentV2;

use super::CommitPanel;

enum BranchMode {
    List,
    CommitsSub { panel: Box<CommitPanel> },
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
        self.mode = BranchMode::CommitsSub {
            panel: Box::new(CommitPanel::new()),
        };
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
                                let next = state
                                    .selected()
                                    .unwrap_or(0)
                                    .saturating_add(1)
                                    .min(len.saturating_sub(1));
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
            BranchMode::CommitsSub { panel } => {
                Component::render(panel.as_mut(), frame, area, is_focused, data)
            }
        }
    }
}

impl ComponentV2 for BranchListPanel {
    fn handle_key_event(&mut self, key: KeyEvent, state: &AppState) -> AppEvent {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !state.data_cache.branches.is_empty() {
                    let current = self.list.state_mut().selected().unwrap_or(0);
                    let next = (current + 1).min(state.data_cache.branches.len() - 1);
                    self.list.state_mut().select(Some(next));
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !state.data_cache.branches.is_empty() {
                    let current = self.list.state_mut().selected().unwrap_or(0);
                    let prev = current.saturating_sub(1);
                    self.list.state_mut().select(Some(prev));
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Enter => AppEvent::ActivatePanel,
            KeyCode::Char('d') => AppEvent::Git(GitEvent::DiscardSelected),
            _ => AppEvent::None,
        }
    }

    fn render(&self, _area: Rect, _buf: &mut Buffer, _state: &AppState) {
        // Render implementation will be added when ComponentV2 is fully integrated
        // For now, this is a stub to satisfy the trait
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::git_ops::CommitEntry;
    use crate::components::core::MultiSelectableList;

    #[test]
    fn test_branch_panel_component_v2() {
        use crate::components::component_v2::ComponentV2;
        use crate::app::events::AppEvent;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = BranchListPanel::new();
        let mut state = mock_state();

        // Add a branch entry so navigation works
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            }
        ];

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);

        // Should return SelectionChanged for 'j' key
        assert_eq!(event, AppEvent::SelectionChanged);
    }

    fn mock_state() -> crate::app::AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        crate::app::AppState::new(cmd_tx, event_rx)
    }

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

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::components::test_utils::*;

    #[test]
    fn test_branch_list_empty_state() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = BranchListPanel::new();
        let data = create_test_cached_data_with_branches(vec![]);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let line = get_buffer_line(buffer, 1);
        assert!(
            line.contains("No items"),
            "Expected 'No items' for empty branches, got: {}",
            line
        );
    }

    #[test]
    fn test_branch_list_renders_branches() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = BranchListPanel::new();
        let branches = vec![
            test_branch_entry("main", true, None),
            test_branch_entry("feature/test", false, Some("origin/feature/test")),
        ];
        let data = create_test_cached_data_with_branches(branches);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Check that branch names appear
        let mut all_content = String::new();
        for row in 0..10 {
            let line = get_buffer_line(buffer, row);
            all_content.push_str(&line);
            all_content.push('\n');
        }

        assert!(
            all_content.contains("main") || all_content.contains("feature"),
            "Expected branch names in buffer, got:\n{}",
            all_content
        );
    }

    #[test]
    fn test_current_branch_indicator() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = BranchListPanel::new();
        let branches = vec![
            test_branch_entry("main", true, None),
            test_branch_entry("develop", false, None),
        ];
        let data = create_test_cached_data_with_branches(branches);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Check for HEAD indicator (*) on the same line as "main"
        let mut found_head_indicator = false;
        for row in 0..10 {
            let line = get_buffer_line(buffer, row);
            if line.contains("main") && line.contains("*") {
                found_head_indicator = true;
                break;
            }
        }

        assert!(
            found_head_indicator,
            "Expected '*' marker on the same line as 'main' branch"
        );
    }

    #[test]
    fn test_branch_with_upstream() {
        let mut terminal = create_test_terminal(60, 10);
        let mut panel = BranchListPanel::new();
        let branches = vec![test_branch_entry(
            "feature/new",
            false,
            Some("origin/feature/new"),
        )];
        let data = create_test_cached_data_with_branches(branches);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        let mut all_content = String::new();
        for row in 0..10 {
            let line = get_buffer_line(buffer, row);
            all_content.push_str(&line);
            all_content.push('\n');
        }

        assert!(
            all_content.contains("feature") || all_content.contains("origin"),
            "Expected branch with upstream info, got:\n{}",
            all_content
        );
    }
}
