use arboard::Clipboard;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;

use crate::app::events::AppEvent;
use crate::app::AppState;
use crate::app::CachedData;
use crate::backend::git_ops::CommitEntry;
use crate::components::component_v2::ComponentV2;
use crate::components::core::{MultiSelectState, MultiSelectableList, TreePanel};

enum CommitMode {
    List,
    #[allow(dead_code)] // Used in mode transitions
    FilesLoading {
        commit_id: String,
        summary: String,
    },
    FilesTree {
        #[allow(dead_code)] // Used in mode transitions
        commit_id: String,
        #[allow(dead_code)] // Used in mode transitions
        summary: String,
        tree: TreePanel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Used in mode_view() method
pub enum CommitModeView {
    List,
    FilesLoading { commit_id: String, summary: String },
    FilesTree { commit_id: String, summary: String },
}

/// Commit 面板：在同一槽位内管理列表/加载中/文件树三种子视图
pub struct CommitPanel {
    state: ListState,
    mode: CommitMode,
    list_multi_select: MultiSelectState<String>,
}

#[allow(dead_code)] // Methods reserved for future use
impl CommitPanel {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            mode: CommitMode::List,
            list_multi_select: MultiSelectState::default(),
        }
    }

    #[allow(dead_code)] // Reserved for future use
    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn selected_commit<'a>(&self, commits: &'a [CommitEntry]) -> Option<&'a CommitEntry> {
        self.state.selected().and_then(|idx| commits.get(idx))
    }

    #[allow(dead_code)] // Used for loading state transitions
    pub fn start_loading(&mut self, commit_id: String, summary: String) {
        self.clear_list_multi_select();
        self.mode = CommitMode::FilesLoading { commit_id, summary };
    }

    pub fn set_files_tree(&mut self, commit_id: String, summary: String, tree: TreePanel) {
        self.mode = CommitMode::FilesTree {
            commit_id,
            summary,
            tree,
        };
    }

    #[allow(dead_code)] // Reserved for future use
    pub fn show_list(&mut self) {
        if let CommitMode::FilesTree { tree, .. } = &mut self.mode {
            tree.clear_multi_select();
        }
        self.mode = CommitMode::List;
    }

    pub fn pending_commit_id(&self) -> Option<&str> {
        match &self.mode {
            CommitMode::FilesLoading { commit_id, .. } => Some(commit_id.as_str()),
            _ => None,
        }
    }

    pub fn mode_view(&self) -> CommitModeView {
        match &self.mode {
            CommitMode::List => CommitModeView::List,
            CommitMode::FilesLoading { commit_id, summary } => CommitModeView::FilesLoading {
                commit_id: commit_id.clone(),
                summary: summary.clone(),
            },
            CommitMode::FilesTree {
                commit_id, summary, ..
            } => CommitModeView::FilesTree {
                commit_id: commit_id.clone(),
                summary: summary.clone(),
            },
        }
    }

    pub fn selected_tree_node(&self) -> Option<(String, bool)> {
        match &self.mode {
            CommitMode::FilesTree { tree, .. } => tree
                .selected_node()
                .map(|node| (node.path.clone(), node.is_dir)),
            _ => None,
        }
    }

    pub fn selected_tree_targets(&self) -> Vec<(String, bool)> {
        match &self.mode {
            CommitMode::FilesTree { tree, .. } => tree.selected_targets(),
            _ => Vec::new(),
        }
    }

    pub fn refresh_list_multi_range(&mut self, commits: &[CommitEntry]) {
        let commit_ids = commit_ids(commits);
        self.refresh_multi_range(self.state.selected(), &commit_ids);
    }

    pub fn clear_list_multi_select(&mut self) {
        self.exit_multi_select();
    }

    pub fn is_list_multi_select_active(&self) -> bool {
        self.is_multi_active()
    }

    pub fn is_tree_multi_select_active(&self) -> bool {
        matches!(
            &self.mode,
            CommitMode::FilesTree { tree, .. } if tree.multi_select_active()
        )
    }

    pub fn handle_escape(&mut self) -> AppEvent {
        match &mut self.mode {
            CommitMode::List => {
                if self.list_multi_select.is_active() {
                    self.clear_list_multi_select();
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            CommitMode::FilesTree { tree, .. } => {
                if tree.multi_select_active() {
                    tree.clear_multi_select();
                    AppEvent::SelectionChanged
                } else {
                    self.show_list();
                    AppEvent::SelectionChanged
                }
            }
            CommitMode::FilesLoading { .. } => AppEvent::None,
        }
    }
}

impl Default for CommitPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiSelectableList for CommitPanel {
    type Key = String;

    fn multi_select_state(&self) -> &MultiSelectState<Self::Key> {
        &self.list_multi_select
    }

    fn multi_select_state_mut(&mut self) -> &mut MultiSelectState<Self::Key> {
        &mut self.list_multi_select
    }
}

impl CommitPanel {
    /// Temporary bridge method for old renderer (will be removed when renderer migrates to ComponentV2)
    pub fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
        data: &CachedData,
    ) {
        use crate::components::core::{
            accent_primary_color, multi_select_row_style, muted_text_style, panel_block,
            SelectableList, LIST_HIGHLIGHT_SYMBOL,
        };
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{ListItem, Paragraph};

        match &mut self.mode {
            CommitMode::List => {
                if data.commits.is_empty() {
                    SelectableList::render_empty(frame, area, "Commits", is_focused);
                    return;
                }

                let multi_active = self.is_list_multi_select_active();
                let title = if multi_active {
                    format!("Commits · MULTI:{}", self.multi_selected_count())
                } else {
                    "Commits".to_string()
                };
                let items: Vec<ListItem<'_>> = data
                    .commits
                    .iter()
                    .map(|commit| {
                        let mut item = ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{} ", commit.short_id),
                                Style::default().fg(accent_primary_color()),
                            ),
                            Span::raw(commit.summary.clone()),
                        ]));
                        if multi_active && self.is_multi_selected_key(&commit.id) {
                            item = item.style(multi_select_row_style());
                        }
                        item
                    })
                    .collect();

                let list = SelectableList::new(items, &title, is_focused, LIST_HIGHLIGHT_SYMBOL);
                list.render(frame, area, &mut self.state);
            }
            CommitMode::FilesLoading { summary, .. } => {
                let block = panel_block(format!("Files · {}", summary), is_focused);
                let paragraph = Paragraph::new("Loading commit files...\n\nPlease wait.")
                    .style(muted_text_style())
                    .block(block);
                frame.render_widget(paragraph, area);
            }
            CommitMode::FilesTree { tree, .. } => {
                tree.render_old(frame, area, is_focused, data);
            }
        }
    }
}

impl ComponentV2 for CommitPanel {
    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent, state: &AppState) -> AppEvent {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !state.data_cache.commits.is_empty() {
                    let current = self.state.selected().unwrap_or(0);
                    let next = (current + 1).min(state.data_cache.commits.len() - 1);
                    self.state.select(Some(next));
                    self.refresh_list_multi_range(&state.data_cache.commits);
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !state.data_cache.commits.is_empty() {
                    let current = self.state.selected().unwrap_or(0);
                    let prev = current.saturating_sub(1);
                    self.state.select(Some(prev));
                    self.refresh_list_multi_range(&state.data_cache.commits);
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Enter => {
                // Only activate if in List mode and not in multi-select
                match &self.mode {
                    CommitMode::List => {
                        if !self.list_multi_select.is_active() {
                            AppEvent::ActivatePanel
                        } else {
                            AppEvent::None
                        }
                    }
                    _ => AppEvent::None,
                }
            }
            KeyCode::Char('v') => {
                if matches!(self.mode, CommitMode::List) && !state.data_cache.commits.is_empty() {
                    let commit_ids = commit_ids(&state.data_cache.commits);
                    self.toggle_multi_select(self.state.selected(), &commit_ids);
                    self.refresh_list_multi_range(&state.data_cache.commits);
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Char('o') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                if let Some(commit) = self.selected_commit(&state.data_cache.commits) {
                    match Clipboard::new().and_then(|mut clip| clip.set_text(commit.id.clone())) {
                        Ok(_) => {
                            // Clipboard write succeeded, no UI feedback needed
                        }
                        Err(e) => {
                            log::warn!("Failed to copy commit hash to clipboard: {}", e);
                        }
                    }
                }
                AppEvent::None
            }
            KeyCode::Char('g') => AppEvent::Modal(crate::app::events::ModalEvent::ShowResetMenu),
            KeyCode::Char('n') => {
                if let Some(commit) = self.selected_commit(&state.data_cache.commits) {
                    AppEvent::Modal(crate::app::events::ModalEvent::ShowBranchCreateDialog {
                        from_branch: commit.id.clone(),
                    })
                } else {
                    AppEvent::None
                }
            }
            _ => AppEvent::None,
        }
    }

    fn render(&self, _area: Rect, _buf: &mut ratatui::buffer::Buffer, _state: &AppState) {
        // Render implementation will be added when ComponentV2 is fully integrated
        // For now, this is a stub to satisfy the trait
    }
}

#[allow(dead_code)] // Helper function for multi-select
fn commit_ids(commits: &[CommitEntry]) -> Vec<String> {
    commits.iter().map(|commit| commit.id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_panel_component_v2() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let mut state = mock_state();

        // Add commit entries so navigation works
        state.data_cache.commits = vec![
            CommitEntry {
                short_id: "abc1234".to_string(),
                id: "abc123".to_string(),
                summary: "Test commit 1".to_string(),
                body: None,
                author: "Author".to_string(),
                timestamp: 1704067200,
            },
            CommitEntry {
                short_id: "def4567".to_string(),
                id: "def456".to_string(),
                summary: "Test commit 2".to_string(),
                body: None,
                author: "Author".to_string(),
                timestamp: 1704153600,
            },
        ];

        // Test j key - should return SelectionChanged
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        // Test k key - should return SelectionChanged
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        // Test Enter key - should return ActivatePanel
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::ActivatePanel);
    }

    #[test]
    fn test_commit_panel_v_toggles_multi_select_mode() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let mut state = mock_state();
        state.data_cache.commits = vec![CommitEntry {
            short_id: "abc1234".to_string(),
            id: "abc123".to_string(),
            summary: "Test commit".to_string(),
            body: None,
            author: "Author".to_string(),
            timestamp: 1704067200,
        }];

        let key_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_v, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert!(panel.is_list_multi_select_active());

        let key_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_v, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert!(!panel.is_list_multi_select_active());
    }

    #[test]
    fn test_commit_panel_esc_clears_multi_select_mode() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let mut state = mock_state();
        state.data_cache.commits = vec![CommitEntry {
            short_id: "abc1234".to_string(),
            id: "abc123".to_string(),
            summary: "Test commit".to_string(),
            body: None,
            author: "Author".to_string(),
            timestamp: 1704067200,
        }];

        let key_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_v, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert!(panel.is_list_multi_select_active());

        let event = panel.handle_escape();
        assert_eq!(event, AppEvent::SelectionChanged);
        assert!(!panel.is_list_multi_select_active());
    }

    #[test]
    fn test_commit_panel_esc_in_files_tree_returns_to_list() {
        use crate::components::core::{GitFileStatus, TreeNode, TreePanel};

        let mut panel = CommitPanel::new();
        let mut state = mock_state();
        state.data_cache.commits = vec![CommitEntry {
            short_id: "abc1234".to_string(),
            id: "abc123".to_string(),
            summary: "Test commit".to_string(),
            body: None,
            author: "Author".to_string(),
            timestamp: 1704067200,
        }];

        let tree = TreePanel::new(
            "Files".to_string(),
            vec![TreeNode::new(
                "src/main.rs".to_string(),
                "main.rs".to_string(),
                false,
                0,
                Some(GitFileStatus::Modified),
            )],
            false,
        );
        panel.set_files_tree("abc123".to_string(), "Test commit".to_string(), tree);

        let event = panel.handle_escape();

        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(panel.mode_view(), CommitModeView::List);
    }

    fn mock_state() -> AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        AppState::new(cmd_tx, event_rx)
    }

    #[test]
    fn test_g_key_shows_reset_menu() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let state = mock_state();

        let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);

        assert_eq!(
            event,
            AppEvent::Modal(crate::app::events::ModalEvent::ShowResetMenu)
        );
    }

    #[test]
    fn test_ctrl_o_copies_hash() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let mut state = mock_state();
        state.data_cache.commits = vec![CommitEntry {
            short_id: "abc123de".to_string(),
            id: "abc123def456".to_string(),
            summary: "Test commit".to_string(),
            body: None,
            author: "Test Author".to_string(),
            timestamp: 1234567890,
        }];
        panel.state.select(Some(0));

        let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL);
        let event = panel.handle_key_event(key, &state);

        // Clipboard operation is side-effect, just verify no crash
        assert_eq!(event, AppEvent::None);
    }

    #[test]
    fn test_n_key_prompts_new_branch() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let mut state = mock_state();
        state.data_cache.commits = vec![CommitEntry {
            id: "abc123def456".to_string(),
            short_id: "abc123de".to_string(),
            summary: "Test".to_string(),
            author: "Author".to_string(),
            timestamp: 1234567890,
            body: None,
        }];
        panel.state.select(Some(0));

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);

        match event {
            AppEvent::Modal(crate::app::events::ModalEvent::ShowBranchCreateDialog {
                from_branch,
            }) => {
                assert_eq!(from_branch, "abc123def456");
            }
            _ => panic!("Expected ShowBranchCreateDialog event, got {:?}", event),
        }
    }
}
