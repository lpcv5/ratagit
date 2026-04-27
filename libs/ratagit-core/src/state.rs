use std::collections::BTreeSet;

use crate::{CommitFilesUiState, FilesUiState};

fn next_choice<T: Copy + PartialEq, const N: usize>(choices: [T; N], selected: T) -> T {
    let index = choices
        .iter()
        .position(|choice| *choice == selected)
        .unwrap_or(0);
    choices[(index + 1).min(N - 1)]
}

fn prev_choice<T: Copy + PartialEq, const N: usize>(choices: [T; N], selected: T) -> T {
    let index = choices
        .iter()
        .position(|choice| *choice == selected)
        .unwrap_or(0);
    choices[index.saturating_sub(1)]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Files,
    Branches,
    Commits,
    Stash,
    Details,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    Files,
    Branches,
    Commits,
    Stash,
    CommitFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchState {
    pub active: bool,
    pub scope: Option<SearchScope>,
    pub query: String,
    pub matches: Vec<String>,
    pub current_match: Option<usize>,
}

impl SearchState {
    pub fn is_input_active_for(&self, scope: SearchScope) -> bool {
        self.active && self.scope == Some(scope)
    }

    pub fn has_query_for(&self, scope: SearchScope) -> bool {
        self.scope == Some(scope) && !self.query.is_empty()
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.scope = None;
        self.query.clear();
        self.matches.clear();
        self.current_match = None;
    }
}

impl PanelFocus {
    pub fn next_left(self) -> Self {
        match self {
            Self::Files => Self::Branches,
            Self::Branches => Self::Commits,
            Self::Commits => Self::Stash,
            Self::Stash => Self::Files,
            Self::Details | Self::Log => Self::Files,
        }
    }

    pub fn prev_left(self) -> Self {
        match self {
            Self::Files => Self::Stash,
            Self::Branches => Self::Files,
            Self::Commits => Self::Branches,
            Self::Stash => Self::Commits,
            Self::Details | Self::Log => Self::Stash,
        }
    }

    pub fn is_left_panel(self) -> bool {
        matches!(
            self,
            Self::Files | Self::Branches | Self::Commits | Self::Stash
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitEntry {
    pub id: String,
    pub full_id: String,
    pub summary: String,
    pub message: String,
    pub author_name: String,
    pub graph: String,
    pub hash_status: CommitHashStatus,
    pub is_merge: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitHashStatus {
    MergedToMain,
    Pushed,
    Unpushed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitInputMode {
    #[default]
    Normal,
    MultiSelect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchInputMode {
    #[default]
    Normal,
    MultiSelect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchEntry {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchDeleteMode {
    Local,
    Remote,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchDeleteChoice {
    Local,
    Remote,
    Both,
}

impl BranchDeleteChoice {
    pub const ALL: [Self; 3] = [Self::Local, Self::Remote, Self::Both];

    pub fn next(self) -> Self {
        next_choice(Self::ALL, self)
    }

    pub fn prev(self) -> Self {
        prev_choice(Self::ALL, self)
    }

    pub fn delete_mode(self) -> BranchDeleteMode {
        match self {
            Self::Local => BranchDeleteMode::Local,
            Self::Remote => BranchDeleteMode::Remote,
            Self::Both => BranchDeleteMode::Both,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchRebaseChoice {
    Simple,
    Interactive,
    OriginMain,
}

impl BranchRebaseChoice {
    pub const ALL: [Self; 3] = [Self::Simple, Self::Interactive, Self::OriginMain];

    pub fn next(self) -> Self {
        next_choice(Self::ALL, self)
    }

    pub fn prev(self) -> Self {
        prev_choice(Self::ALL, self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoStashOperation {
    Checkout { branch: String },
    CheckoutCommitDetached { commit_id: String },
    Rebase { target: String, interactive: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSnapshot {
    pub status_summary: String,
    pub current_branch: String,
    pub detached_head: bool,
    pub files: Vec<crate::FileEntry>,
    pub commits: Vec<CommitEntry>,
    pub branches: Vec<BranchEntry>,
    pub stashes: Vec<StashEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesSnapshot {
    pub status_summary: String,
    pub current_branch: String,
    pub detached_head: bool,
    pub files: Vec<crate::FileEntry>,
    pub index_entry_count: usize,
    pub large_repo_mode: bool,
    pub status_truncated: bool,
    pub status_scan_skipped: bool,
    pub untracked_scan_skipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusPanelState {
    pub summary: String,
    pub current_branch: String,
    pub detached_head: bool,
    pub refresh_count: u64,
    pub last_error: Option<String>,
    pub index_entry_count: usize,
    pub large_repo_mode: bool,
    pub status_truncated: bool,
    pub status_scan_skipped: bool,
    pub untracked_scan_skipped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusMode {
    Full,
    LargeRepoFast,
    HugeRepoMetadataOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefreshTarget {
    Files,
    Branches,
    Commits,
    Stash,
}

impl RefreshTarget {
    pub const ALL: [Self; 4] = [Self::Files, Self::Branches, Self::Commits, Self::Stash];
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkStatusState {
    pub refresh_pending: bool,
    pub pending_refreshes: BTreeSet<RefreshTarget>,
    pub details_pending: bool,
    pub operation_pending: Option<String>,
    pub last_completed_command: Option<String>,
    pub commits_loading_more: bool,
    pub commits_pending_select_after_load: bool,
    pub commit_files_loading: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilesRepoState {
    pub items: Vec<crate::FileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitFilesRepoState {
    pub items: Vec<crate::CommitFileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitsRepoState {
    pub items: Vec<CommitEntry>,
    pub files: CommitFilesRepoState,
    pub has_more: bool,
    pub pagination_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchesRepoState {
    pub items: Vec<BranchEntry>,
    pub commits: Vec<CommitEntry>,
    pub commit_files: CommitFilesRepoState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashRepoState {
    pub items: Vec<StashEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitsUiState {
    pub selected: usize,
    pub scroll_offset: usize,
    pub files: CommitFilesUiState,
    pub selected_rows: BTreeSet<String>,
    pub selection_anchor: Option<String>,
    pub mode: CommitInputMode,
    pub draft_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFileDiffTarget {
    pub commit_id: String,
    pub paths: Vec<CommitFileDiffPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFileDiffPath {
    pub path: String,
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitField {
    Message,
    Body,
}

impl CommitField {
    pub fn next(self) -> Self {
        match self {
            Self::Message => Self::Body,
            Self::Body => Self::Message,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StashScope {
    All,
    SelectedPaths(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    Mixed,
    Soft,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetChoice {
    Mixed,
    Soft,
    Hard,
    Nuke,
}

impl ResetChoice {
    pub const ALL: [Self; 4] = [Self::Mixed, Self::Soft, Self::Hard, Self::Nuke];

    pub fn next(self) -> Self {
        next_choice(Self::ALL, self)
    }

    pub fn prev(self) -> Self {
        prev_choice(Self::ALL, self)
    }

    pub fn reset_mode(self) -> Option<ResetMode> {
        match self {
            Self::Mixed => Some(ResetMode::Mixed),
            Self::Soft => Some(ResetMode::Soft),
            Self::Hard => Some(ResetMode::Hard),
            Self::Nuke => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetMenuState {
    pub active: bool,
    pub selected: ResetChoice,
    pub danger_confirm: Option<ResetChoice>,
}

impl Default for ResetMenuState {
    fn default() -> Self {
        Self {
            active: false,
            selected: ResetChoice::Mixed,
            danger_confirm: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscardConfirmState {
    pub active: bool,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PushForceConfirmState {
    pub active: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorKind {
    Commit {
        message: String,
        message_cursor: usize,
        body: String,
        body_cursor: usize,
        active_field: CommitField,
        intent: CommitEditorIntent,
    },
    Stash {
        title: String,
        title_cursor: usize,
        scope: StashScope,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitEditorIntent {
    Create,
    Reword { commit_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorState {
    pub kind: Option<EditorKind>,
}

impl EditorState {
    pub fn is_active(&self) -> bool {
        self.kind.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchesUiState {
    pub selected: usize,
    pub scroll_offset: usize,
    pub subview: BranchesSubview,
    pub subview_branch: Option<String>,
    pub commits: CommitsUiState,
    pub commit_files: CommitFilesUiState,
    pub selected_rows: BTreeSet<String>,
    pub selection_anchor: Option<String>,
    pub mode: BranchInputMode,
    pub create: BranchCreateState,
    pub delete_menu: BranchDeleteMenuState,
    pub delete_confirm: BranchDeleteConfirmState,
    pub force_delete_confirm: BranchForceDeleteConfirmState,
    pub rebase_menu: BranchRebaseMenuState,
    pub auto_stash_confirm: AutoStashConfirmState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchesSubview {
    #[default]
    List,
    Commits,
    CommitFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchCreateState {
    pub active: bool,
    pub name: String,
    pub cursor: usize,
    pub start_point: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchDeleteMenuState {
    pub active: bool,
    pub selected: BranchDeleteChoice,
    pub target_branch: String,
}

impl Default for BranchDeleteMenuState {
    fn default() -> Self {
        Self {
            active: false,
            selected: BranchDeleteChoice::Local,
            target_branch: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchDeleteConfirmState {
    pub active: bool,
    pub target_branch: String,
    pub mode: Option<BranchDeleteMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchForceDeleteConfirmState {
    pub active: bool,
    pub target_branch: String,
    pub mode: Option<BranchDeleteMode>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRebaseMenuState {
    pub active: bool,
    pub selected: BranchRebaseChoice,
    pub target_branch: String,
}

impl Default for BranchRebaseMenuState {
    fn default() -> Self {
        Self {
            active: false,
            selected: BranchRebaseChoice::Simple,
            target_branch: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AutoStashConfirmState {
    pub active: bool,
    pub operation: Option<AutoStashOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashUiState {
    pub selected: usize,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DetailsRepoState {
    pub files_diff: String,
    pub files_targets: Vec<String>,
    pub files_error: Option<String>,
    pub files_diff_truncated_from: Option<usize>,
    pub cached_files_diffs: Vec<CachedFilesDiff>,
    pub branch_log: String,
    pub branch_log_target: Option<String>,
    pub branch_log_error: Option<String>,
    pub cached_branch_logs: Vec<CachedBranchLog>,
    pub commit_diff: String,
    pub commit_diff_target: Option<String>,
    pub commit_diff_error: Option<String>,
    pub cached_commit_diffs: Vec<CachedCommitDiff>,
    pub commit_file_diff: String,
    pub commit_file_diff_target: Option<CommitFileDiffTarget>,
    pub commit_file_diff_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DetailsUiState {
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedFilesDiff {
    pub paths: Vec<String>,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedBranchLog {
    pub branch: String,
    pub log: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedCommitDiff {
    pub commit_id: String,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoState {
    pub status: StatusPanelState,
    pub files: FilesRepoState,
    pub commits: CommitsRepoState,
    pub branches: BranchesRepoState,
    pub stash: StashRepoState,
    pub details: DetailsRepoState,
}

impl Default for RepoState {
    fn default() -> Self {
        Self {
            status: StatusPanelState {
                summary: "No data yet".to_string(),
                current_branch: "unknown".to_string(),
                detached_head: false,
                refresh_count: 0,
                last_error: None,
                index_entry_count: 0,
                large_repo_mode: false,
                status_truncated: false,
                status_scan_skipped: false,
                untracked_scan_skipped: false,
            },
            files: FilesRepoState::default(),
            commits: CommitsRepoState::default(),
            branches: BranchesRepoState::default(),
            stash: StashRepoState::default(),
            details: DetailsRepoState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    pub focus: PanelFocus,
    pub last_left_focus: PanelFocus,
    pub search: SearchState,
    pub files: FilesUiState,
    pub commits: CommitsUiState,
    pub branches: BranchesUiState,
    pub stash: StashUiState,
    pub details: DetailsUiState,
    pub editor: EditorState,
    pub reset_menu: ResetMenuState,
    pub discard_confirm: DiscardConfirmState,
    pub push_force_confirm: PushForceConfirmState,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            focus: PanelFocus::Files,
            last_left_focus: PanelFocus::Files,
            search: SearchState::default(),
            files: FilesUiState::default(),
            commits: CommitsUiState::default(),
            branches: BranchesUiState::default(),
            stash: StashUiState::default(),
            details: DetailsUiState::default(),
            editor: EditorState::default(),
            reset_menu: ResetMenuState::default(),
            discard_confirm: DiscardConfirmState::default(),
            push_force_confirm: PushForceConfirmState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppContext {
    pub repo: RepoState,
    pub ui: UiState,
    pub work: WorkStatusState,
    pub notices: Vec<String>,
    pub last_operation: Option<String>,
}

impl Default for AppContext {
    fn default() -> Self {
        Self {
            repo: RepoState::default(),
            ui: UiState::default(),
            work: WorkStatusState::default(),
            notices: vec!["Ready".to_string()],
            last_operation: None,
        }
    }
}

impl AppContext {
    pub fn active_search_scope(&self) -> Option<SearchScope> {
        match self.ui.focus {
            PanelFocus::Files => Some(SearchScope::Files),
            PanelFocus::Branches => match self.ui.branches.subview {
                BranchesSubview::List => Some(SearchScope::Branches),
                BranchesSubview::Commits => Some(SearchScope::Commits),
                BranchesSubview::CommitFiles => Some(SearchScope::CommitFiles),
            },
            PanelFocus::Commits if self.ui.commits.files.active => Some(SearchScope::CommitFiles),
            PanelFocus::Commits => Some(SearchScope::Commits),
            PanelFocus::Stash => Some(SearchScope::Stash),
            PanelFocus::Details | PanelFocus::Log => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_context_default_groups_repo_ui_and_work_state() {
        let state = AppContext::default();

        assert_eq!(state.repo.status.summary, "No data yet");
        assert!(state.repo.files.items.is_empty());
        assert!(state.repo.commits.items.is_empty());
        assert!(state.repo.branches.items.is_empty());
        assert!(state.repo.stash.items.is_empty());
        assert!(state.repo.details.files_diff.is_empty());

        assert_eq!(state.ui.focus, PanelFocus::Files);
        assert_eq!(state.ui.last_left_focus, PanelFocus::Files);
        assert!(!state.ui.search.active);
        assert_eq!(state.ui.details.scroll_offset, 0);

        assert!(!state.work.refresh_pending);
        assert!(!state.work.details_pending);
        assert!(state.work.operation_pending.is_none());
        assert_eq!(state.notices, vec!["Ready".to_string()]);
    }

    #[test]
    fn bounded_choices_move_to_edges_without_wrapping() {
        assert_eq!(BranchDeleteChoice::Local.prev(), BranchDeleteChoice::Local);
        assert_eq!(BranchDeleteChoice::Local.next(), BranchDeleteChoice::Remote);
        assert_eq!(BranchDeleteChoice::Both.next(), BranchDeleteChoice::Both);

        assert_eq!(
            BranchRebaseChoice::Simple.prev(),
            BranchRebaseChoice::Simple
        );
        assert_eq!(
            BranchRebaseChoice::Interactive.next(),
            BranchRebaseChoice::OriginMain
        );
        assert_eq!(
            BranchRebaseChoice::OriginMain.next(),
            BranchRebaseChoice::OriginMain
        );

        assert_eq!(ResetChoice::Mixed.prev(), ResetChoice::Mixed);
        assert_eq!(ResetChoice::Soft.next(), ResetChoice::Hard);
        assert_eq!(ResetChoice::Nuke.next(), ResetChoice::Nuke);
    }
}
