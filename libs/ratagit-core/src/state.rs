use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};

use crate::{CommitFilesUiState, FilesUiState};

pub trait MenuChoice: Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn next(self) -> Self {
        let Some(index) = Self::ALL.iter().position(|choice| *choice == self) else {
            return self;
        };

        Self::ALL[(index + 1).min(Self::ALL.len() - 1)]
    }

    fn prev(self) -> Self {
        let Some(index) = Self::ALL.iter().position(|choice| *choice == self) else {
            return self;
        };

        Self::ALL[index.saturating_sub(1)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu<T> {
    pub active: bool,
    pub selected: T,
}

impl<T: Default> Default for Menu<T> {
    fn default() -> Self {
        Self {
            active: false,
            selected: T::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetBranchMenu<T> {
    pub menu: Menu<T>,
    pub target_branch: String,
}

impl<T: Default> Default for TargetBranchMenu<T> {
    fn default() -> Self {
        Self {
            menu: Menu::default(),
            target_branch: String::new(),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveLeftView {
    Files,
    BranchesList,
    BranchCommits,
    BranchCommitFiles,
    Commits,
    CommitFiles,
    Stash,
}

impl ActiveLeftView {
    pub fn search_scope(self) -> SearchScope {
        match self {
            Self::Files => SearchScope::Files,
            Self::BranchesList => SearchScope::Branches,
            Self::BranchCommits | Self::Commits => SearchScope::Commits,
            Self::BranchCommitFiles | Self::CommitFiles => SearchScope::CommitFiles,
            Self::Stash => SearchScope::Stash,
        }
    }

    pub fn commit_list_target(self) -> Option<CommitListTarget> {
        match self {
            Self::BranchCommits => Some(CommitListTarget::Branch),
            Self::Commits => Some(CommitListTarget::Main),
            _ => None,
        }
    }

    pub fn commit_files_target(self) -> Option<CommitFilesTarget> {
        match self {
            Self::BranchCommitFiles => Some(CommitFilesTarget::Branch),
            Self::CommitFiles => Some(CommitFilesTarget::Main),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitListTarget {
    Main,
    Branch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitFilesTarget {
    Main,
    Branch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPaletteSection {
    Local,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPaletteCommand {
    Pull,
    Push,
    RefreshAll,
    FocusPrev,
    FocusNext,
    FocusPanel(PanelFocus),
    DetailsScrollUp,
    DetailsScrollDown,
    Quit,
    ToggleSelectedDirectory,
    ToggleSelectedFileStage,
    EnterFilesMultiSelect,
    OpenCommitEditor,
    OpenStashEditor,
    OpenResetMenu,
    OpenDiscardConfirm,
    AmendStagedChanges,
    OpenBranchCommitsPanel,
    CloseBranchCommitsPanel,
    OpenBranchCommitFilesPanel,
    CloseBranchCommitFilesPanel,
    ToggleBranchCommitFilesDirectory,
    OpenBranchCreateInput,
    EnterBranchesMultiSelect,
    CheckoutSelectedBranch,
    OpenBranchDeleteMenu,
    OpenBranchRebaseMenu,
    OpenCommitFilesPanel,
    CloseCommitFilesPanel,
    ToggleCommitFilesDirectory,
    EnterCommitFilesMultiSelect,
    EnterCommitsMultiSelect,
    SquashSelectedCommits,
    FixupSelectedCommits,
    OpenCommitRewordEditor,
    DeleteSelectedCommits,
    CheckoutSelectedCommitDetached,
    StashPopSelected,
}

impl CommandPaletteCommand {
    pub fn is_quit(self) -> bool {
        self == Self::Quit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPaletteEntry {
    pub section: CommandPaletteSection,
    pub key: &'static str,
    pub label: &'static str,
    pub command: CommandPaletteCommand,
}

impl CommandPaletteEntry {
    const fn local(key: &'static str, label: &'static str, command: CommandPaletteCommand) -> Self {
        Self {
            section: CommandPaletteSection::Local,
            key,
            label,
            command,
        }
    }

    const fn global(
        key: &'static str,
        label: &'static str,
        command: CommandPaletteCommand,
    ) -> Self {
        Self {
            section: CommandPaletteSection::Global,
            key,
            label,
            command,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchState {
    pub active: bool,
    pub scope: Option<SearchScope>,
    pub query: String,
    pub matches: Vec<String>,
    pub current_match: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandPaletteState {
    pub active: bool,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinearListSelection {
    pub selected: usize,
    pub scroll_offset: usize,
    pub selected_rows: BTreeSet<String>,
    pub selection_anchor: Option<String>,
}

pub(crate) fn move_linear_selection(state: &mut LinearListSelection, len: usize, move_up: bool) {
    crate::scroll::move_selected_index(&mut state.selected, len, move_up);
}

pub(crate) fn move_linear_selection_in_viewport(
    state: &mut LinearListSelection,
    len: usize,
    move_up: bool,
    visible_lines: usize,
) {
    crate::scroll::move_selected_index_with_scroll_offset(
        &mut state.selected,
        &mut state.scroll_offset,
        len,
        move_up,
        visible_lines,
    );
}

pub(crate) fn clamp_linear_selection(state: &mut LinearListSelection, len: usize) {
    state.selected = if len == 0 {
        0
    } else {
        state.selected.min(len - 1)
    };
    state.scroll_offset = 0;
}

pub(crate) fn enter_linear_range_select(
    state: &mut LinearListSelection,
    selected_key: Option<String>,
) {
    state.selection_anchor = selected_key.clone();
    state.selected_rows.clear();
    if let Some(key) = selected_key {
        state.selected_rows.insert(key);
    }
}

pub(crate) fn leave_linear_range_select(state: &mut LinearListSelection) {
    state.selection_anchor = None;
    state.selected_rows.clear();
}

pub(crate) fn refresh_linear_range(state: &mut LinearListSelection, keys: &[String]) {
    state.selected_rows.clear();
    let Some(anchor) = state.selection_anchor.as_deref() else {
        return;
    };
    let Some(anchor_index) = keys.iter().position(|key| key == anchor) else {
        return;
    };
    if keys.is_empty() {
        return;
    }
    let selected = state.selected.min(keys.len() - 1);
    let (start, end) = if anchor_index <= selected {
        (anchor_index, selected)
    } else {
        (selected, anchor_index)
    };
    state
        .selected_rows
        .extend(keys[start..=end].iter().cloned());
}

pub(crate) fn reconcile_linear_valid_keys(state: &mut LinearListSelection, keys: &[String]) {
    let valid_rows = keys.iter().cloned().collect::<BTreeSet<_>>();
    state.selected_rows.retain(|key| valid_rows.contains(key));
}

pub(crate) fn ensure_linear_selection_anchor(state: &mut LinearListSelection, keys: &[String]) {
    if state
        .selection_anchor
        .as_ref()
        .is_some_and(|anchor| keys.iter().any(|key| key == anchor))
    {
        return;
    }
    state.selection_anchor = keys.get(state.selected).cloned();
}

pub(crate) fn linear_key_at_selection(
    state: &LinearListSelection,
    keys: &[String],
) -> Option<String> {
    keys.get(state.selected).cloned()
}

pub(crate) fn linear_key_is_selected(state: &LinearListSelection, key: &str) -> bool {
    state.selected_rows.contains(key)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchDeleteChoice {
    #[default]
    Local,
    Remote,
    Both,
}

impl BranchDeleteChoice {
    pub const ALL: [Self; 3] = [Self::Local, Self::Remote, Self::Both];

    pub fn delete_mode(self) -> BranchDeleteMode {
        match self {
            Self::Local => BranchDeleteMode::Local,
            Self::Remote => BranchDeleteMode::Remote,
            Self::Both => BranchDeleteMode::Both,
        }
    }
}

impl MenuChoice for BranchDeleteChoice {
    const ALL: &'static [Self] = &BranchDeleteChoice::ALL;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchRebaseChoice {
    #[default]
    Simple,
    Interactive,
    OriginMain,
}

impl BranchRebaseChoice {
    pub const ALL: [Self; 3] = [Self::Simple, Self::Interactive, Self::OriginMain];
}

impl MenuChoice for BranchRebaseChoice {
    const ALL: &'static [Self] = &BranchRebaseChoice::ALL;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoStashOperation {
    Checkout { branch: String },
    CheckoutCommitDetached { commit_id: String },
    Rebase { target: String, interactive: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageAllOperation {
    CreateCommit { message: String },
    AmendStagedChanges { commit_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StageAllConfirmContext {
    pub operation: Option<StageAllOperation>,
    pub paths: Vec<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailsRequestId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailsRequestTarget {
    FilesDiff {
        targets: Vec<crate::FileDiffTarget>,
        truncated_from: Option<usize>,
    },
    BranchLog {
        branch: String,
    },
    CommitDiff {
        commit_id: String,
    },
    CommitFileDiff {
        target: CommitFileDiffTarget,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailsRequest {
    pub id: DetailsRequestId,
    pub target: DetailsRequestTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RefreshWork {
    pub refresh_pending: bool,
    pub pending_refreshes: BTreeSet<RefreshTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DetailsWork {
    pub details_pending: bool,
    pub next_details_request_id: u64,
    pub details_request: Option<DetailsRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MutationWork {
    pub operation_pending: Option<String>,
    pub last_completed_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaginationWork {
    pub commits_loading_more: bool,
    pub commits_pending_select_after_load: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitFilesWork {
    pub commit_files_loading: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkStatusState {
    pub refresh: RefreshWork,
    pub details: DetailsWork,
    pub mutation: MutationWork,
    pub pagination: PaginationWork,
    pub commit_files: CommitFilesWork,
}

impl WorkStatusState {
    pub fn mark_refresh_all_pending(&mut self) {
        self.refresh.refresh_pending = true;
        self.refresh.pending_refreshes = RefreshTarget::ALL.into_iter().collect();
    }

    pub fn mark_refresh_target_pending(&mut self, target: RefreshTarget) {
        self.refresh.refresh_pending = true;
        self.refresh.pending_refreshes.insert(target);
    }

    pub fn finish_refresh_target(&mut self, target: RefreshTarget, label: impl Into<String>) {
        self.refresh.pending_refreshes.remove(&target);
        self.refresh.refresh_pending = !self.refresh.pending_refreshes.is_empty();
        self.mutation.last_completed_command = Some(label.into());
        if !self.refresh.refresh_pending {
            self.mutation.last_completed_command = Some("refresh".to_string());
        }
    }

    pub fn clear_refresh(&mut self) {
        self.refresh.pending_refreshes.clear();
        self.refresh.refresh_pending = false;
        self.mutation.last_completed_command = Some("refresh".to_string());
    }

    pub fn clear_details_pending(&mut self) {
        self.details.details_pending = false;
        self.details.details_request = None;
    }

    pub fn mark_command_completed(&mut self, label: impl Into<String>) {
        self.mutation.last_completed_command = Some(label.into());
    }

    pub fn record_operation_completed(&mut self, label: impl Into<String>) {
        self.mutation.operation_pending = None;
        self.mutation.last_completed_command = Some(label.into());
    }
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
    pub selection: LinearListSelection,
    pub files: CommitFilesUiState,
    pub mode: CommitInputMode,
    pub draft_message: String,
}

impl Deref for CommitsUiState {
    type Target = LinearListSelection;

    fn deref(&self) -> &Self::Target {
        &self.selection
    }
}

impl DerefMut for CommitsUiState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.selection
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResetChoice {
    #[default]
    Mixed,
    Soft,
    Hard,
    Nuke,
}

impl ResetChoice {
    pub const ALL: [Self; 4] = [Self::Mixed, Self::Soft, Self::Hard, Self::Nuke];

    pub fn reset_mode(self) -> Option<ResetMode> {
        match self {
            Self::Mixed => Some(ResetMode::Mixed),
            Self::Soft => Some(ResetMode::Soft),
            Self::Hard => Some(ResetMode::Hard),
            Self::Nuke => None,
        }
    }
}

impl MenuChoice for ResetChoice {
    const ALL: &'static [Self] = &ResetChoice::ALL;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResetMenuState {
    pub menu: Menu<ResetChoice>,
    pub danger_confirm: Option<ResetChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfirmDialog<T> {
    pub active: bool,
    pub context: T,
}

pub type DiscardConfirmState = ConfirmDialog<Vec<String>>;
pub type PushForceConfirmState = ConfirmDialog<String>;
pub type StageAllConfirmState = ConfirmDialog<StageAllConfirmContext>;

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
    pub selection: LinearListSelection,
    pub subview: BranchesSubview,
    pub subview_branch: Option<String>,
    pub commits: CommitsUiState,
    pub commit_files: CommitFilesUiState,
    pub mode: BranchInputMode,
    pub create: BranchCreateState,
    pub delete_menu: BranchDeleteMenuState,
    pub delete_confirm: BranchDeleteConfirmState,
    pub force_delete_confirm: BranchForceDeleteConfirmState,
    pub rebase_menu: BranchRebaseMenuState,
    pub auto_stash_confirm: AutoStashConfirmState,
}

impl Deref for BranchesUiState {
    type Target = LinearListSelection;

    fn deref(&self) -> &Self::Target {
        &self.selection
    }
}

impl DerefMut for BranchesUiState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.selection
    }
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

pub type BranchDeleteMenuState = TargetBranchMenu<BranchDeleteChoice>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchDeleteConfirmContext {
    pub target_branch: String,
    pub mode: Option<BranchDeleteMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchForceDeleteConfirmContext {
    pub target_branch: String,
    pub mode: Option<BranchDeleteMode>,
    pub reason: String,
}

pub type BranchDeleteConfirmState = ConfirmDialog<BranchDeleteConfirmContext>;
pub type BranchForceDeleteConfirmState = ConfirmDialog<BranchForceDeleteConfirmContext>;

pub type BranchRebaseMenuState = TargetBranchMenu<BranchRebaseChoice>;

pub type AutoStashConfirmState = ConfirmDialog<Option<AutoStashOperation>>;

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
    pub command_palette: CommandPaletteState,
    pub files: FilesUiState,
    pub commits: CommitsUiState,
    pub branches: BranchesUiState,
    pub stash: StashUiState,
    pub details: DetailsUiState,
    pub editor: EditorState,
    pub reset_menu: ResetMenuState,
    pub discard_confirm: DiscardConfirmState,
    pub push_force_confirm: PushForceConfirmState,
    pub stage_all_confirm: StageAllConfirmState,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            focus: PanelFocus::Files,
            last_left_focus: PanelFocus::Files,
            search: SearchState::default(),
            command_palette: CommandPaletteState::default(),
            files: FilesUiState::default(),
            commits: CommitsUiState::default(),
            branches: BranchesUiState::default(),
            stash: StashUiState::default(),
            details: DetailsUiState::default(),
            editor: EditorState::default(),
            reset_menu: ResetMenuState::default(),
            discard_confirm: DiscardConfirmState::default(),
            push_force_confirm: PushForceConfirmState::default(),
            stage_all_confirm: StageAllConfirmState::default(),
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
    pub fn active_left_view(&self) -> Option<ActiveLeftView> {
        left_view_for_focus(
            self.ui.focus,
            self.ui.branches.subview,
            self.ui.commits.files.active,
        )
    }

    pub fn details_left_view(&self) -> Option<ActiveLeftView> {
        left_view_for_focus(
            self.ui.last_left_focus,
            self.ui.branches.subview,
            self.ui.commits.files.active,
        )
    }

    pub fn active_commit_list_target(&self) -> Option<CommitListTarget> {
        self.active_left_view()?.commit_list_target()
    }

    pub fn active_commit_files_target(&self) -> Option<CommitFilesTarget> {
        self.active_left_view()?.commit_files_target()
    }

    pub fn details_commit_list_target(&self) -> Option<CommitListTarget> {
        self.details_left_view()?.commit_list_target()
    }

    pub fn details_commit_files_target(&self) -> Option<CommitFilesTarget> {
        self.details_left_view()?.commit_files_target()
    }

    pub fn active_search_scope(&self) -> Option<SearchScope> {
        self.active_left_view().map(ActiveLeftView::search_scope)
    }

    pub fn command_palette_entries(&self) -> Vec<CommandPaletteEntry> {
        let mut entries = local_command_palette_entries(self);
        entries.extend(global_command_palette_entries());
        entries
    }

    pub fn selected_command_palette_entry(&self) -> Option<CommandPaletteEntry> {
        self.command_palette_entries()
            .get(self.ui.command_palette.selected)
            .copied()
    }
}

fn left_view_for_focus(
    focus: PanelFocus,
    branches_subview: BranchesSubview,
    commit_files_active: bool,
) -> Option<ActiveLeftView> {
    match focus {
        PanelFocus::Files => Some(ActiveLeftView::Files),
        PanelFocus::Branches => Some(match branches_subview {
            BranchesSubview::List => ActiveLeftView::BranchesList,
            BranchesSubview::Commits => ActiveLeftView::BranchCommits,
            BranchesSubview::CommitFiles => ActiveLeftView::BranchCommitFiles,
        }),
        PanelFocus::Commits if commit_files_active => Some(ActiveLeftView::CommitFiles),
        PanelFocus::Commits => Some(ActiveLeftView::Commits),
        PanelFocus::Stash => Some(ActiveLeftView::Stash),
        PanelFocus::Details | PanelFocus::Log => None,
    }
}

fn local_command_palette_entries(state: &AppContext) -> Vec<CommandPaletteEntry> {
    use CommandPaletteCommand as Command;
    use CommandPaletteEntry as Entry;

    match state.active_left_view() {
        Some(ActiveLeftView::Files) => vec![
            Entry::local("space", "stage/unstage", Command::ToggleSelectedFileStage),
            Entry::local("d", "discard", Command::OpenDiscardConfirm),
            Entry::local("A", "amend", Command::AmendStagedChanges),
            Entry::local("c", "commit", Command::OpenCommitEditor),
            Entry::local("s", "stash", Command::OpenStashEditor),
            Entry::local("D", "reset", Command::OpenResetMenu),
            Entry::local("enter", "expand", Command::ToggleSelectedDirectory),
            Entry::local("v", "visual select", Command::EnterFilesMultiSelect),
        ],
        Some(ActiveLeftView::BranchesList) => vec![
            Entry::local("enter", "commits", Command::OpenBranchCommitsPanel),
            Entry::local("space", "checkout", Command::CheckoutSelectedBranch),
            Entry::local("n", "new branch", Command::OpenBranchCreateInput),
            Entry::local("d", "delete branch", Command::OpenBranchDeleteMenu),
            Entry::local("r", "rebase", Command::OpenBranchRebaseMenu),
            Entry::local("v", "visual select", Command::EnterBranchesMultiSelect),
        ],
        Some(ActiveLeftView::BranchCommits) => vec![
            Entry::local("enter", "files", Command::OpenBranchCommitFilesPanel),
            Entry::local("Esc", "back", Command::CloseBranchCommitsPanel),
            Entry::local("v", "visual select", Command::EnterCommitsMultiSelect),
        ],
        Some(ActiveLeftView::BranchCommitFiles) => vec![
            Entry::local("enter", "expand", Command::ToggleBranchCommitFilesDirectory),
            Entry::local("Esc", "back", Command::CloseBranchCommitFilesPanel),
            Entry::local("v", "visual select", Command::EnterCommitFilesMultiSelect),
        ],
        Some(ActiveLeftView::CommitFiles) => vec![
            Entry::local("Esc", "back", Command::CloseCommitFilesPanel),
            Entry::local("enter", "expand", Command::ToggleCommitFilesDirectory),
            Entry::local("v", "visual select", Command::EnterCommitFilesMultiSelect),
        ],
        Some(ActiveLeftView::Commits) => vec![
            Entry::local("enter", "files", Command::OpenCommitFilesPanel),
            Entry::local("A", "amend", Command::AmendStagedChanges),
            Entry::local("s", "squash", Command::SquashSelectedCommits),
            Entry::local("f", "fixup", Command::FixupSelectedCommits),
            Entry::local("r", "reword", Command::OpenCommitRewordEditor),
            Entry::local("d", "delete", Command::DeleteSelectedCommits),
            Entry::local("space", "detach", Command::CheckoutSelectedCommitDetached),
            Entry::local("c", "commit", Command::OpenCommitEditor),
            Entry::local("v", "visual select", Command::EnterCommitsMultiSelect),
        ],
        Some(ActiveLeftView::Stash) => {
            vec![Entry::local("O", "stash pop", Command::StashPopSelected)]
        }
        None => Vec::new(),
    }
}

fn global_command_palette_entries() -> Vec<CommandPaletteEntry> {
    use CommandPaletteCommand as Command;
    use CommandPaletteEntry as Entry;

    vec![
        Entry::global("p", "pull", Command::Pull),
        Entry::global("P", "push", Command::Push),
        Entry::global("r", "refresh", Command::RefreshAll),
        Entry::global("h", "focus previous", Command::FocusPrev),
        Entry::global("l", "focus next", Command::FocusNext),
        Entry::global("1", "focus files", Command::FocusPanel(PanelFocus::Files)),
        Entry::global(
            "2",
            "focus branches",
            Command::FocusPanel(PanelFocus::Branches),
        ),
        Entry::global(
            "3",
            "focus commits",
            Command::FocusPanel(PanelFocus::Commits),
        ),
        Entry::global("4", "focus stash", Command::FocusPanel(PanelFocus::Stash)),
        Entry::global(
            "5",
            "focus details",
            Command::FocusPanel(PanelFocus::Details),
        ),
        Entry::global("6", "focus log", Command::FocusPanel(PanelFocus::Log)),
        Entry::global("Ctrl+U", "details scroll up", Command::DetailsScrollUp),
        Entry::global("Ctrl+D", "details scroll down", Command::DetailsScrollDown),
        Entry::global("q", "quit", Command::Quit),
    ]
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
        assert!(!state.ui.command_palette.active);
        assert_eq!(state.ui.command_palette.selected, 0);
        assert_eq!(state.ui.details.scroll_offset, 0);

        assert!(!state.work.refresh.refresh_pending);
        assert!(!state.work.details.details_pending);
        assert!(state.work.mutation.operation_pending.is_none());
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

    #[test]
    fn menu_choice_trait_moves_to_edges_without_wrapping() {
        fn assert_bounded<T>(first: T, middle: T, last: T)
        where
            T: MenuChoice + std::fmt::Debug,
        {
            assert_eq!(<T as MenuChoice>::prev(first), first);
            assert_eq!(<T as MenuChoice>::next(middle), last);
            assert_eq!(<T as MenuChoice>::next(last), last);
        }

        assert_bounded(
            BranchDeleteChoice::Local,
            BranchDeleteChoice::Remote,
            BranchDeleteChoice::Both,
        );
        assert_bounded(
            BranchRebaseChoice::Simple,
            BranchRebaseChoice::Interactive,
            BranchRebaseChoice::OriginMain,
        );
        assert_bounded(ResetChoice::Mixed, ResetChoice::Hard, ResetChoice::Nuke);
    }

    #[test]
    fn confirm_dialog_default_starts_inactive_with_default_context() {
        let discard_confirm = DiscardConfirmState::default();
        let push_confirm = PushForceConfirmState::default();
        let stage_all_confirm = StageAllConfirmState::default();
        let branch_delete_confirm = BranchDeleteConfirmState::default();
        let branch_force_delete_confirm = BranchForceDeleteConfirmState::default();
        let auto_stash_confirm = AutoStashConfirmState::default();

        assert!(!discard_confirm.active);
        assert!(discard_confirm.context.is_empty());
        assert!(!push_confirm.active);
        assert!(push_confirm.context.is_empty());
        assert!(!stage_all_confirm.active);
        assert_eq!(stage_all_confirm.context.operation, None);
        assert!(stage_all_confirm.context.paths.is_empty());
        assert!(!branch_delete_confirm.active);
        assert!(branch_delete_confirm.context.target_branch.is_empty());
        assert_eq!(branch_delete_confirm.context.mode, None);
        assert!(!branch_force_delete_confirm.active);
        assert!(branch_force_delete_confirm.context.target_branch.is_empty());
        assert_eq!(branch_force_delete_confirm.context.mode, None);
        assert!(branch_force_delete_confirm.context.reason.is_empty());
        assert!(!auto_stash_confirm.active);
        assert_eq!(auto_stash_confirm.context, None);
    }

    #[test]
    fn active_left_view_routes_branch_and_main_subviews() {
        let mut state = AppContext::default();
        assert_eq!(state.active_left_view(), Some(ActiveLeftView::Files));

        state.ui.focus = PanelFocus::Branches;
        assert_eq!(state.active_left_view(), Some(ActiveLeftView::BranchesList));

        state.ui.branches.subview = BranchesSubview::Commits;
        assert_eq!(
            state.active_left_view(),
            Some(ActiveLeftView::BranchCommits)
        );
        assert_eq!(
            state.active_commit_list_target(),
            Some(CommitListTarget::Branch)
        );

        state.ui.branches.subview = BranchesSubview::CommitFiles;
        assert_eq!(
            state.active_left_view(),
            Some(ActiveLeftView::BranchCommitFiles)
        );
        assert_eq!(
            state.active_commit_files_target(),
            Some(CommitFilesTarget::Branch)
        );

        state.ui.focus = PanelFocus::Commits;
        assert_eq!(state.active_left_view(), Some(ActiveLeftView::Commits));
        assert_eq!(
            state.active_commit_list_target(),
            Some(CommitListTarget::Main)
        );

        state.ui.commits.files.active = true;
        assert_eq!(state.active_left_view(), Some(ActiveLeftView::CommitFiles));
        assert_eq!(
            state.active_commit_files_target(),
            Some(CommitFilesTarget::Main)
        );
    }

    #[test]
    fn details_left_view_uses_last_left_focus() {
        let mut state = AppContext::default();
        state.ui.focus = PanelFocus::Details;
        state.ui.last_left_focus = PanelFocus::Branches;
        state.ui.branches.subview = BranchesSubview::CommitFiles;

        assert_eq!(state.active_left_view(), None);
        assert_eq!(
            state.details_left_view(),
            Some(ActiveLeftView::BranchCommitFiles)
        );
        assert_eq!(
            state.details_commit_files_target(),
            Some(CommitFilesTarget::Branch)
        );
    }
}
