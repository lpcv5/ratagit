use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::CachedData;
use crate::backend::git_ops::CommitEntry;
use crate::components::core::{
    accent_primary_color, multi_select_row_style, muted_text_style, panel_block,
    ActionMultiplicity, MultiSelectState, MultiSelectableList, SelectableList, TreePanel,
    LIST_HIGHLIGHT_SYMBOL,
};
use crate::components::Component;
use crate::components::Intent;

enum CommitMode {
    List,
    FilesLoading {
        commit_id: String,
        summary: String,
    },
    FilesTree {
        commit_id: String,
        summary: String,
        tree: TreePanel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

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

impl Component for CommitPanel {
    fn handle_event(&mut self, event: &Event, data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            if key.code == KeyCode::Esc {
                let list_multi_active = self.is_list_multi_select_active();
                match &mut self.mode {
                    CommitMode::List if list_multi_active => {
                        self.clear_list_multi_select();
                        return Intent::RefreshPanelDetail;
                    }
                    CommitMode::FilesTree { tree, .. } if tree.multi_select_active() => {
                        tree.clear_multi_select();
                        return Intent::RefreshPanelDetail;
                    }
                    CommitMode::List => {}
                    _ => {
                        self.show_list();
                        return Intent::SwitchFocus(crate::app::Panel::Commits);
                    }
                }
            }

            if matches!(self.mode, CommitMode::List) {
                return match key.code {
                    KeyCode::Char('v') if key.modifiers.is_empty() => {
                        let commit_ids = commit_ids(&data.commits);
                        self.toggle_multi_select(self.state.selected(), &commit_ids);
                        Intent::RefreshPanelDetail
                    }
                    KeyCode::Char('j') | KeyCode::Down => Intent::SelectNext,
                    KeyCode::Char('k') | KeyCode::Up => Intent::SelectPrevious,
                    KeyCode::Enter
                        if self.is_list_multi_select_active()
                            && self.enter_action_multiplicity()
                                == ActionMultiplicity::SingleOnly =>
                    {
                        Intent::None
                    }
                    KeyCode::Enter => Intent::ActivatePanel,
                    _ => Intent::None,
                };
            }
        }

        if let CommitMode::FilesTree { tree, .. } = &mut self.mode {
            let before_targets = tree.selected_targets();
            let before_multi = tree.multi_select_active();
            let intent = tree.handle_event(event, data);
            if !matches!(intent, Intent::None) {
                return intent;
            }

            let after_targets = tree.selected_targets();
            let after_multi = tree.multi_select_active();
            if before_targets != after_targets || before_multi != after_multi {
                return Intent::RefreshPanelDetail;
            }

            return Intent::None;
        }

        Intent::None
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        match &self.mode {
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
                let state = &mut self.state.clone();
                list.render(frame, area, state);
            }
            CommitMode::FilesLoading { summary, .. } => {
                let block = panel_block(format!("Files · {}", summary), is_focused);
                let paragraph = Paragraph::new("Loading commit files...\n\nPlease wait.")
                    .style(muted_text_style())
                    .block(block);
                frame.render_widget(paragraph, area);
            }
            CommitMode::FilesTree { tree, .. } => {
                tree.render(frame, area, is_focused, data);
            }
        }
    }
}

fn commit_ids(commits: &[CommitEntry]) -> Vec<String> {
    commits.iter().map(|commit| commit.id.clone()).collect()
}

impl CommitPanel {
    fn enter_action_multiplicity(&self) -> ActionMultiplicity {
        ActionMultiplicity::SingleOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_returns_to_list_mode_from_loading() {
        let mut panel = CommitPanel::new();
        panel.start_loading("abc12345".to_string(), "summary".to_string());

        let event = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ));
        let intent = panel.handle_event(&event, &CachedData::default());

        assert!(matches!(
            intent,
            Intent::SwitchFocus(crate::app::Panel::Commits)
        ));
        assert_eq!(panel.mode_view(), CommitModeView::List);
    }

    #[test]
    fn enter_activates_panel_in_list_mode() {
        let mut panel = CommitPanel::new();
        let event = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));

        let intent = panel.handle_event(&event, &CachedData::default());
        assert!(matches!(intent, Intent::ActivatePanel));
    }

    #[test]
    fn enter_is_disabled_when_list_multi_select_is_active() {
        let mut panel = CommitPanel::new();
        let data = CachedData {
            commits: vec![
                CommitEntry {
                    short_id: "a".to_string(),
                    id: "aaaaaaaa".to_string(),
                    summary: "a".to_string(),
                    body: None,
                    author: "a".to_string(),
                    timestamp: 0,
                },
                CommitEntry {
                    short_id: "b".to_string(),
                    id: "bbbbbbbb".to_string(),
                    summary: "b".to_string(),
                    body: None,
                    author: "b".to_string(),
                    timestamp: 0,
                },
            ],
            ..CachedData::default()
        };

        let enter_multi = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('v'),
            crossterm::event::KeyModifiers::NONE,
        ));
        panel.handle_event(&enter_multi, &data);

        let enter = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        let intent = panel.handle_event(&enter, &data);
        assert!(matches!(intent, Intent::None));
    }

    #[test]
    fn tree_navigation_refreshes_panel_detail() {
        let mut panel = CommitPanel::new();
        let nodes = vec![
            crate::components::core::TreeNode::new(
                "a.rs".to_string(),
                "a.rs".to_string(),
                false,
                0,
                None,
            ),
            crate::components::core::TreeNode::new(
                "b.rs".to_string(),
                "b.rs".to_string(),
                false,
                0,
                None,
            ),
        ];
        panel.set_files_tree(
            "abc12345".to_string(),
            "summary".to_string(),
            TreePanel::new("Files".to_string(), nodes, false),
        );

        let event = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        let intent = panel.handle_event(&event, &CachedData::default());

        assert!(matches!(intent, Intent::RefreshPanelDetail));
    }
}
