use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::CachedData;
use crate::components::core::{
    accent_primary_color, muted_text_style, panel_block, SelectableList, TreePanel,
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
}

impl CommitPanel {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            mode: CommitMode::List,
        }
    }

    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn start_loading(&mut self, commit_id: String, summary: String) {
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
}

impl Default for CommitPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for CommitPanel {
    fn handle_event(&mut self, event: &Event, data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            if key.code == KeyCode::Esc && !matches!(self.mode, CommitMode::List) {
                self.show_list();
                return Intent::SwitchFocus(crate::app::Panel::Commits);
            }

            if matches!(self.mode, CommitMode::List) {
                return match key.code {
                    KeyCode::Char('j') | KeyCode::Down => Intent::SelectNext,
                    KeyCode::Char('k') | KeyCode::Up => Intent::SelectPrevious,
                    KeyCode::Enter => Intent::ActivatePanel,
                    _ => Intent::None,
                };
            }
        }

        if let CommitMode::FilesTree { tree, .. } = &mut self.mode {
            let before = tree.selected_node().map(|node| node.path.clone());
            let intent = tree.handle_event(event, data);
            if !matches!(intent, Intent::None) {
                return intent;
            }

            let after = tree.selected_node().map(|node| node.path.clone());
            if before != after {
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

                let items: Vec<ListItem<'_>> = data
                    .commits
                    .iter()
                    .map(|commit| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{} ", commit.short_id),
                                Style::default().fg(accent_primary_color()),
                            ),
                            Span::raw(commit.summary.clone()),
                        ]))
                    })
                    .collect();

                let list = SelectableList::new(items, "Commits", is_focused, LIST_HIGHLIGHT_SYMBOL);
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
