use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::app::events::{AppEvent, GitEvent, ModalEvent};
use crate::app::AppState;
use crate::app::CachedData;
use crate::components::component_v2::ComponentV2;
use crate::components::core::{render_branches, SimpleListPanel};

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

    pub fn hide_branch_commits(&mut self) {
        self.mode = BranchMode::List;
    }

    pub fn handle_escape(&mut self) -> AppEvent {
        match &mut self.mode {
            BranchMode::CommitsSub { panel } => {
                let event = panel.handle_escape();
                if event == AppEvent::None {
                    AppEvent::ExitBranchCommitsSubview
                } else {
                    event
                }
            }
            BranchMode::List => AppEvent::None,
        }
    }

    /// Temporary bridge method for old renderer (will be removed when renderer migrates to ComponentV2)
    pub fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
        data: &CachedData,
    ) {
        match &mut self.mode {
            BranchMode::List => self.list.render(frame, area, is_focused, data),
            BranchMode::CommitsSub { panel } => {
                CommitPanel::render(panel, frame, area, is_focused, data)
            }
        }
    }
}

impl Default for BranchListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentV2 for BranchListPanel {
    fn handle_key_event(&mut self, key: KeyEvent, state: &AppState) -> AppEvent {
        if let BranchMode::CommitsSub { panel } = &mut self.mode {
            return panel.handle_key_event(key, state);
        }

        let selected_branch = || {
            self.selected_index()
                .and_then(|index| state.data_cache.branches.get(index))
        };

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
            KeyCode::Char(' ') => {
                let Some(branch) = selected_branch() else {
                    return AppEvent::None;
                };
                AppEvent::Git(GitEvent::CheckoutBranch {
                    branch_name: branch.name.clone(),
                    force: false,
                })
            }
            KeyCode::Char('n') => {
                let Some(branch) = selected_branch() else {
                    return AppEvent::None;
                };
                AppEvent::Modal(ModalEvent::ShowBranchCreateDialog {
                    from_branch: branch.name.clone(),
                })
            }
            KeyCode::Enter => {
                let Some(branch) = selected_branch() else {
                    return AppEvent::None;
                };
                AppEvent::Git(GitEvent::LoadBranchCommits {
                    branch_name: branch.name.clone(),
                    limit: 100,
                })
            }
            KeyCode::Char('d') => {
                let Some(branch) = selected_branch() else {
                    return AppEvent::None;
                };
                AppEvent::Modal(ModalEvent::ShowBranchDeleteMenu {
                    local_branch: branch.name.clone(),
                    is_head: branch.is_head,
                    upstream: branch.upstream.clone(),
                })
            }
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
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_branch_panel_component_v2() {
        use crate::app::events::AppEvent;
        use crate::components::component_v2::ComponentV2;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = BranchListPanel::new();
        let mut state = mock_state();

        // Add a branch entry so navigation works
        state.data_cache.branches = vec![crate::backend::git_ops::BranchEntry {
            name: "main".to_string(),
            is_head: true,
            upstream: None,
        }];

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);

        // Should return SelectionChanged for 'j' key
        assert_eq!(event, AppEvent::SelectionChanged);
    }

    #[test]
    fn test_branch_list_navigation_wraparound() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        // Start at first branch
        panel.state_mut().select(Some(0));

        // Navigate down twice (should stop at last item, not wrap)
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        panel.handle_key_event(key_j, &state);
        panel.handle_key_event(key_j, &state);

        assert_eq!(panel.selected_index(), Some(1)); // Should be at last item
    }

    #[test]
    fn test_branch_list_enter_loads_branch_commits() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        panel.state_mut().select(Some(1)); // Select feature branch

        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key_enter, &state);

        assert_eq!(
            event,
            AppEvent::Git(GitEvent::LoadBranchCommits {
                branch_name: "feature".to_string(),
                limit: 100,
            })
        );
    }

    #[test]
    fn test_branch_list_space_checkout_selected_branch() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        panel.state_mut().select(Some(1)); // Select feature branch

        let key_space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_space, &state);

        assert_eq!(
            event,
            AppEvent::Git(GitEvent::CheckoutBranch {
                branch_name: "feature".to_string(),
                force: false,
            })
        );
    }

    #[test]
    fn test_branch_list_n_opens_create_dialog() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![crate::backend::git_ops::BranchEntry {
            name: "main".to_string(),
            is_head: true,
            upstream: None,
        }];

        panel.state_mut().select(Some(0)); // Select current branch

        let key_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_n, &state);

        assert_eq!(
            event,
            AppEvent::Modal(ModalEvent::ShowBranchCreateDialog {
                from_branch: "main".to_string(),
            })
        );
    }

    #[test]
    fn test_branch_list_d_opens_delete_menu_with_branch_state() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![crate::backend::git_ops::BranchEntry {
            name: "feature".to_string(),
            is_head: false,
            upstream: Some("refs/remotes/origin/feature".to_string()),
        }];
        panel.state_mut().select(Some(0));

        let key_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_d, &state);

        assert_eq!(
            event,
            AppEvent::Modal(ModalEvent::ShowBranchDeleteMenu {
                local_branch: "feature".to_string(),
                is_head: false,
                upstream: Some("refs/remotes/origin/feature".to_string()),
            })
        );
    }

    #[test]
    fn test_branch_list_empty_state() {
        let mut panel = BranchListPanel::new();
        let state = mock_state();
        // No branches in state

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);

        // Should return None when no branches
        assert_eq!(event, AppEvent::None);
    }

    #[test]
    fn test_branch_commits_subview_delegates_navigation_to_commit_panel() {
        let mut panel = BranchListPanel::new();
        panel.show_branch_commits();

        let mut state = mock_state();
        state.data_cache.commits = vec![
            crate::backend::git_ops::CommitEntry {
                short_id: "abc1234".to_string(),
                id: "abc123".to_string(),
                summary: "Test commit 1".to_string(),
                body: None,
                author: "Author".to_string(),
                timestamp: 1704067200,
            },
            crate::backend::git_ops::CommitEntry {
                short_id: "def4567".to_string(),
                id: "def456".to_string(),
                summary: "Test commit 2".to_string(),
                body: None,
                author: "Author".to_string(),
                timestamp: 1704153600,
            },
        ];
        // Keep branches empty so list-mode handling would return None.

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);

        assert_eq!(event, AppEvent::SelectionChanged);
    }

    #[test]
    fn test_branch_commits_subview_esc_exits_subview_when_not_selecting() {
        let mut panel = BranchListPanel::new();
        panel.show_branch_commits();

        let mut state = mock_state();
        state.data_cache.commits = vec![crate::backend::git_ops::CommitEntry {
            short_id: "abc1234".to_string(),
            id: "abc123".to_string(),
            summary: "Test commit".to_string(),
            body: None,
            author: "Author".to_string(),
            timestamp: 1704067200,
        }];

        let event = panel.handle_escape();

        assert_eq!(event, AppEvent::ExitBranchCommitsSubview);
    }

    fn mock_state() -> crate::app::AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        crate::app::AppState::new(cmd_tx, event_rx)
    }
}
