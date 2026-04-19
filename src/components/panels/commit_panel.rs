use arboard::Clipboard;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex};

use crate::app::events::AppEvent;
use crate::app::events::GitEvent;
use crate::app::AppState;
use crate::app::CachedData;
use crate::backend::git_ops::{CommitDivergence, CommitEntry, CommitStatus};
use crate::components::component_v2::ComponentV2;
use crate::components::core::{MultiSelectState, MultiSelectableList, TreePanel};

#[derive(Debug, Clone, Default)]
pub struct SharedCommitClipboard {
    inner: Arc<Mutex<Vec<String>>>,
}

impl SharedCommitClipboard {
    fn with_ids<R>(&self, f: impl FnOnce(&Vec<String>) -> R) -> R {
        let guard = self.inner.lock().expect("shared clipboard poisoned");
        f(&guard)
    }

    fn with_ids_mut<R>(&self, f: impl FnOnce(&mut Vec<String>) -> R) -> R {
        let mut guard = self.inner.lock().expect("shared clipboard poisoned");
        f(&mut guard)
    }
}

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
    copied_commit_ids: SharedCommitClipboard,
}

#[allow(dead_code)] // Methods reserved for future use
impl CommitPanel {
    pub fn new() -> Self {
        Self::with_shared_clipboard(SharedCommitClipboard::default())
    }

    pub fn with_shared_clipboard(copied_commit_ids: SharedCommitClipboard) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            mode: CommitMode::List,
            list_multi_select: MultiSelectState::default(),
            copied_commit_ids,
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

    fn selected_commit_ids(&self, commits: &[CommitEntry]) -> Vec<String> {
        if self.is_list_multi_select_active() {
            let all_ids = commit_ids(commits);
            let ids = self.multi_selected_keys(&all_ids);
            if !ids.is_empty() {
                return ids;
            }
        }

        self.selected_commit(commits)
            .map(|commit| vec![commit.id.clone()])
            .unwrap_or_default()
    }

    fn toggle_copy_selection(&mut self, commits: &[CommitEntry]) -> bool {
        let selected_ids = self.selected_commit_ids(commits);
        if selected_ids.is_empty() {
            return false;
        }
        self.copied_commit_ids.with_ids_mut(|copied| {
            let all_selected_already_copied = selected_ids
                .iter()
                .all(|id| copied.iter().any(|copied_id| copied_id == id));
            if all_selected_already_copied {
                copied.retain(|copied_id| !selected_ids.iter().any(|id| id == copied_id));
                return true;
            }

            let mut changed = false;
            for commit_id in selected_ids {
                if !copied.iter().any(|id| id == &commit_id) {
                    copied.push(commit_id);
                    changed = true;
                }
            }
            changed
        })
    }

    fn reset_copied_commits(&mut self) -> bool {
        self.copied_commit_ids.with_ids_mut(|copied| {
            if copied.is_empty() {
                return false;
            }
            copied.clear();
            true
        })
    }

    fn copied_count(&self) -> usize {
        self.copied_commit_ids.with_ids(Vec::len)
    }

    fn has_copied_commit(&self, commit_id: &str) -> bool {
        self.copied_commit_ids
            .with_ids(|copied| copied.iter().any(|copied_id| copied_id == commit_id))
    }

    fn copied_commits_for_paste(&self) -> Vec<String> {
        self.copied_commit_ids.with_ids(|copied| copied.clone())
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

#[derive(Debug)]
struct CommitRowRenderData {
    columns: [String; 6],
    hash_style: ratatui::style::Style,
    author_style: ratatui::style::Style,
    graph_cells: Vec<crate::backend::git_ops::GraphCell>,
    branch_head_marker: bool,
    tags: String,
    summary: String,
}

fn author_with_length(author_name: &str, length: usize) -> String {
    if length < 2 {
        return String::new();
    }
    if length == 2 {
        let parts: Vec<&str> = author_name.split_whitespace().collect();
        return match parts.as_slice() {
            [] => String::new(),
            [single] => single.chars().take(2).collect(),
            [first, second, ..] => {
                let first_char = first.chars().next().unwrap_or_default();
                let second_char = second.chars().next().unwrap_or_default();
                format!("{first_char}{second_char}")
            }
        };
    }

    let mut result: String = author_name.chars().take(length).collect();
    let padding = length.saturating_sub(result.chars().count());
    if padding > 0 {
        result.push_str(&" ".repeat(padding));
    }
    result
}

fn text_width(text: &str) -> usize {
    text.chars().count()
}

fn right_pad(text: &str, width: usize) -> String {
    let current = text_width(text);
    if current >= width {
        return text.to_string();
    }
    let mut result = String::with_capacity(width);
    result.push_str(text);
    result.push_str(&" ".repeat(width - current));
    result
}

fn visible_columns(rows: &[CommitRowRenderData]) -> Vec<usize> {
    (0..6)
        .filter(|idx| rows.iter().any(|row| !row.columns[*idx].is_empty()))
        .collect()
}

fn column_widths(rows: &[CommitRowRenderData], columns: &[usize]) -> [usize; 6] {
    let mut widths = [0_usize; 6];
    for column in columns {
        widths[*column] = rows
            .iter()
            .map(|row| text_width(&row.columns[*column]))
            .max()
            .unwrap_or(0);
    }
    widths
}

fn commit_hash_style(commit: &CommitEntry, copied: bool) -> ratatui::style::Style {
    use ratatui::style::Color;

    if copied {
        return ratatui::style::Style::default().fg(Color::Blue);
    }
    match commit.status {
        CommitStatus::Unpushed => ratatui::style::Style::default().fg(Color::Red),
        CommitStatus::Pushed => ratatui::style::Style::default().fg(Color::Yellow),
        CommitStatus::Merged => ratatui::style::Style::default().fg(Color::Green),
        CommitStatus::Rebasing
        | CommitStatus::CherryPickingOrReverting
        | CommitStatus::Conflicted => ratatui::style::Style::default().fg(Color::Blue),
        CommitStatus::None => ratatui::style::Style::default(),
    }
}

fn graph_cell_style(color_idx: u8) -> ratatui::style::Style {
    use ratatui::style::Color;
    let color = match color_idx % 8 {
        0 => Color::DarkGray,
        1 => Color::Cyan,
        2 => Color::Yellow,
        3 => Color::Green,
        4 => Color::Magenta,
        5 => Color::Blue,
        6 => Color::Red,
        7 => Color::White,
        _ => Color::DarkGray,
    };
    ratatui::style::Style::default().fg(color)
}

fn row_columns(commit: &CommitEntry, copied: bool) -> CommitRowRenderData {
    use crate::components::core::accent_secondary_color;
    use ratatui::style::Style;

    let divergence = match commit.divergence {
        CommitDivergence::Left => "↑".to_string(),
        CommitDivergence::Right => "↓".to_string(),
        CommitDivergence::None => String::new(),
    };
    let hash_style = commit_hash_style(commit, copied);
    let author_name = if commit.author_name.is_empty() {
        commit.author.as_str()
    } else {
        commit.author_name.as_str()
    };
    let author = author_with_length(author_name, 2);

    CommitRowRenderData {
        columns: [
            divergence,
            commit.short_id.clone(),
            String::new(),
            String::new(),
            String::new(),
            author,
        ],
        hash_style,
        author_style: Style::default().fg(accent_secondary_color()),
        graph_cells: commit.graph_cells.clone(),
        branch_head_marker: commit.is_branch_head && commit.status != CommitStatus::Merged,
        tags: if commit.tags.is_empty() {
            String::new()
        } else {
            format!("{} ", commit.tags.join(" "))
        },
        summary: commit.summary.clone(),
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
            multi_select_row_style, muted_text_style, panel_block, theme, SelectableList,
            LIST_HIGHLIGHT_SYMBOL,
        };
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{ListItem, Paragraph};

        match &mut self.mode {
            CommitMode::List => {
                if data.commits.is_empty() {
                    SelectableList::render_empty(frame, area, "Commits", is_focused);
                    return;
                }

                let multi_active = self.is_list_multi_select_active();
                let mut title = "Commits".to_string();
                if multi_active {
                    title.push_str(&format!(" · MULTI:{}", self.multi_selected_count()));
                }
                if self.copied_count() > 0 {
                    title.push_str(&format!(" · COPIED:{}", self.copied_count()));
                }

                let rows: Vec<CommitRowRenderData> = data
                    .commits
                    .iter()
                    .map(|commit| row_columns(commit, self.has_copied_commit(&commit.id)))
                    .collect();
                let visible = visible_columns(&rows);
                let widths = column_widths(&rows, &visible);
                let palette = theme();

                let items: Vec<ListItem<'_>> = data
                    .commits
                    .iter()
                    .zip(rows.iter())
                    .map(|(commit, row)| {
                        let mut spans: Vec<Span<'_>> = Vec::new();
                        for column in &visible {
                            let text = right_pad(&row.columns[*column], widths[*column]);
                            let style = match *column {
                                0 | 1 => row.hash_style,
                                5 => row.author_style,
                                _ => Style::default(),
                            };
                            spans.push(Span::styled(format!("{text} "), style));
                        }
                        for cell in &row.graph_cells {
                            spans.push(Span::styled(
                                cell.chars.clone(),
                                graph_cell_style(cell.color_idx),
                            ));
                        }
                        if row.branch_head_marker {
                            spans.push(Span::styled(
                                "* ",
                                Style::default()
                                    .fg(palette.diff_header)
                                    .add_modifier(Modifier::BOLD),
                            ));
                        }
                        if !row.tags.is_empty() {
                            spans.push(Span::styled(
                                row.tags.clone(),
                                Style::default()
                                    .fg(palette.accent_primary)
                                    .add_modifier(Modifier::BOLD),
                            ));
                        }
                        spans.push(Span::raw(row.summary.clone()));

                        let mut item = ListItem::new(Line::from(spans));
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

        match &mut self.mode {
            CommitMode::FilesLoading { .. } => AppEvent::None,
            CommitMode::FilesTree { tree, .. } => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    tree.select_next();
                    AppEvent::SelectionChanged
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    tree.select_previous();
                    AppEvent::SelectionChanged
                }
                KeyCode::Enter => {
                    tree.toggle_selected_dir();
                    AppEvent::SelectionChanged
                }
                KeyCode::Char('-') => {
                    tree.collapse_all_dirs();
                    AppEvent::SelectionChanged
                }
                KeyCode::Char('=') => {
                    tree.expand_all_dirs();
                    AppEvent::SelectionChanged
                }
                KeyCode::Char('v') => {
                    if tree.selected_node().is_some() {
                        tree.toggle_multi_select_at_cursor();
                        AppEvent::SelectionChanged
                    } else {
                        AppEvent::None
                    }
                }
                _ => AppEvent::None,
            },
            CommitMode::List => match key.code {
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
                    if !self.list_multi_select.is_active() {
                        AppEvent::ActivatePanel
                    } else {
                        AppEvent::None
                    }
                }
                KeyCode::Char('v') => {
                    if !state.data_cache.commits.is_empty() {
                        let commit_ids = commit_ids(&state.data_cache.commits);
                        self.toggle_multi_select(self.state.selected(), &commit_ids);
                        self.refresh_list_multi_range(&state.data_cache.commits);
                        AppEvent::SelectionChanged
                    } else {
                        AppEvent::None
                    }
                }
                KeyCode::Char('o')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    if let Some(commit) = self.selected_commit(&state.data_cache.commits) {
                        match Clipboard::new()
                            .and_then(|mut clip| clip.set_text(commit.short_id.clone()))
                        {
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
                KeyCode::Char('C') => {
                    if self.toggle_copy_selection(&state.data_cache.commits) {
                        AppEvent::SelectionChanged
                    } else {
                        AppEvent::None
                    }
                }
                KeyCode::Char('V') => {
                    let commit_ids = self.copied_commits_for_paste();
                    if commit_ids.is_empty() {
                        AppEvent::None
                    } else {
                        AppEvent::Git(GitEvent::CherryPickCommits { commit_ids })
                    }
                }
                KeyCode::Char('r')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    if self.reset_copied_commits() {
                        AppEvent::SelectionChanged
                    } else {
                        AppEvent::None
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(commit) = self.selected_commit(&state.data_cache.commits) {
                        AppEvent::Git(GitEvent::CheckoutCommit {
                            commit_id: commit.id.clone(),
                        })
                    } else {
                        AppEvent::None
                    }
                }
                KeyCode::Char('g') => {
                    AppEvent::Modal(crate::app::events::ModalEvent::ShowResetMenu)
                }
                KeyCode::Char('n') => {
                    if let Some(commit) = self.selected_commit(&state.data_cache.commits) {
                        AppEvent::Modal(crate::app::events::ModalEvent::ShowBranchCreateDialog {
                            from_branch: commit.id.clone(),
                        })
                    } else {
                        AppEvent::None
                    }
                }
                KeyCode::Char('t') => {
                    if let Some(commit) = self.selected_commit(&state.data_cache.commits) {
                        AppEvent::Git(GitEvent::RevertCommit {
                            commit_id: commit.id.clone(),
                        })
                    } else {
                        AppEvent::None
                    }
                }
                _ => AppEvent::None,
            },
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
                ..Default::default()
            },
            CommitEntry {
                short_id: "def4567".to_string(),
                id: "def456".to_string(),
                summary: "Test commit 2".to_string(),
                body: None,
                author: "Author".to_string(),
                timestamp: 1704153600,
                ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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

    #[test]
    fn test_commit_files_tree_navigation_shortcuts_work() {
        use crate::components::core::{build_tree_from_paths, TreePanel};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let state = mock_state();
        let paths = vec!["README.md".to_string(), "src/main.rs".to_string()];
        let tree_nodes = build_tree_from_paths(&paths, None);
        panel.set_files_tree(
            "abc123".to_string(),
            "Test commit".to_string(),
            TreePanel::new("Files".to_string(), tree_nodes, false),
        );

        // Move to `src` directory
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(panel.selected_tree_node(), Some(("src".to_string(), true)));

        // Collapse selected directory
        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key_enter, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        // When collapsed, moving down stays on `src`
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(panel.selected_tree_node(), Some(("src".to_string(), true)));

        // Expand all and move into child file
        let key_equal = KeyEvent::new(KeyCode::Char('='), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_equal, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(
            panel.selected_tree_node(),
            Some(("src/main.rs".to_string(), false))
        );

        // Collapse all jumps/keeps selection on visible ancestor
        let key_minus = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_minus, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
    }

    #[test]
    fn test_commit_files_tree_ignores_list_only_shortcuts() {
        use crate::components::core::{build_tree_from_paths, TreePanel};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let state = mock_state();
        let paths = vec!["src/main.rs".to_string()];
        let tree_nodes = build_tree_from_paths(&paths, None);
        panel.set_files_tree(
            "abc123".to_string(),
            "Test commit".to_string(),
            TreePanel::new("Files".to_string(), tree_nodes, false),
        );

        let key_space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_space, &state);
        assert_eq!(event, AppEvent::None);
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
            ..Default::default()
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
            ..Default::default()
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

    #[test]
    fn test_n_key_with_empty_commits_returns_none() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = CommitPanel::new();
        let state = mock_state();
        // Empty commits list

        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);

        assert_eq!(event, AppEvent::None);
    }

    #[test]
    fn test_c_key_toggles_copied_commit_selection() {
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
            ..Default::default()
        }];
        panel.state.select(Some(0));

        let key_copy = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT);
        let event = panel.handle_key_event(key_copy, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(panel.copied_count(), 1);

        let key_copy = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT);
        let event = panel.handle_key_event(key_copy, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(panel.copied_count(), 0);
    }

    #[test]
    fn test_v_key_pastes_copied_commits() {
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
            ..Default::default()
        }];
        panel.state.select(Some(0));

        let key_copy = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT);
        let _ = panel.handle_key_event(key_copy, &state);

        let key_paste = KeyEvent::new(KeyCode::Char('V'), KeyModifiers::SHIFT);
        let event = panel.handle_key_event(key_paste, &state);
        assert_eq!(
            event,
            AppEvent::Git(GitEvent::CherryPickCommits {
                commit_ids: vec!["abc123def456".to_string()],
            })
        );
    }

    #[test]
    fn test_ctrl_r_resets_copied_commits() {
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
            ..Default::default()
        }];
        panel.state.select(Some(0));

        let key_copy = KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT);
        let _ = panel.handle_key_event(key_copy, &state);
        assert_eq!(panel.copied_count(), 1);

        let key_reset = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        let event = panel.handle_key_event(key_reset, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(panel.copied_count(), 0);
    }

    #[test]
    fn test_space_key_checkouts_selected_commit() {
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
            ..Default::default()
        }];
        panel.state.select(Some(0));

        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(
            event,
            AppEvent::Git(GitEvent::CheckoutCommit {
                commit_id: "abc123def456".to_string(),
            })
        );
    }
}
