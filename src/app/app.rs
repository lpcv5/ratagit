use super::{diff_cache, diff_loader, dirty_flags, refresh, revision_tree};
use crate::config::keymap::{key_to_string, Keymap};
use crate::git::{
    BranchInfo, CommitInfo, DiffLine, FileEntry, Git2Repository, GitRepository, GitStatus,
    StashInfo,
};
use crate::ui::layout::render_layout;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;
use ratatui::Frame;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

/// Documentation comment in English.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SidePanel {
    #[default]
    Files,
    LocalBranches,
    Commits,
    Stash,
}

/// Documentation comment in English.
pub struct PanelState {
    pub list_state: ListState,
}

impl PanelState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }
}

impl Default for PanelState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TreeModeState<T> {
    pub active: bool,
    pub nodes: Vec<FileTreeNode>,
    pub files: Vec<FileEntry>,
    pub expanded_dirs: HashSet<PathBuf>,
    pub selected_source: Option<T>,
}

impl<T> Default for TreeModeState<T> {
    fn default() -> Self {
        Self {
            active: false,
            nodes: Vec::new(),
            files: Vec::new(),
            expanded_dirs: HashSet::new(),
            selected_source: None,
        }
    }
}

#[derive(Default)]
pub struct FilesPanelState {
    pub panel: PanelState,
    pub tree_nodes: Vec<FileTreeNode>,
    pub expanded_dirs: HashSet<PathBuf>,
    pub visual_mode: bool,
    pub visual_anchor: Option<usize>,
}

#[derive(Default)]
pub struct BranchesPanelState {
    pub panel: PanelState,
    pub items: Vec<BranchInfo>,
    pub is_fetching_remote: bool,
}

#[derive(Default)]
pub struct CommitsPanelState {
    pub panel: PanelState,
    pub items: Vec<CommitInfo>,
    pub dirty: bool,
    pub tree_mode: TreeModeState<String>,
}

#[derive(Default)]
pub struct StashPanelState {
    pub panel: PanelState,
    pub items: Vec<StashInfo>,
    pub tree_mode: TreeModeState<usize>,
}

/// Documentation comment in English.
pub struct CommandLogEntry {
    pub command: String,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    CommitEditor,
    CreateBranch,
    StashEditor,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitFieldFocus {
    Message,
    Description,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshKind {
    StatusOnly,
    StatusAndRefs,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SearchScopeKey {
    pub panel: SidePanel,
    pub commit_tree_mode: bool,
    pub stash_tree_mode: bool,
}

/// Documentation comment in English.
pub struct App {
    pub running: bool,
    pub active_panel: SidePanel,

    repo: Box<dyn GitRepository>,
    pub status: GitStatus,

    pub files: FilesPanelState,
    pub branches: BranchesPanelState,
    pub commits: CommitsPanelState,
    pub stash: StashPanelState,

    pub command_log: Vec<CommandLogEntry>,
    pub current_diff: Vec<DiffLine>,
    /// Documentation comment in English.
    pub diff_scroll: usize,
    pub input_mode: Option<InputMode>,
    pub input_buffer: String,
    pub commit_message_buffer: String,
    pub commit_description_buffer: String,
    pub commit_focus: CommitFieldFocus,
    pub stash_message_buffer: String,
    pub stash_targets: Vec<PathBuf>,
    pub search_query: String,
    pub(super) search_matches: Vec<usize>,
    pub(super) search_scope: SearchScopeKey,
    pub(super) search_queries: HashMap<SearchScopeKey, String>,
    pending_refresh: Option<RefreshKind>,
    pending_diff_reload: bool,
    pending_diff_reload_at: Option<Instant>,

    diff_cache: diff_cache::DiffCache,
    last_diff_key: Option<diff_cache::DiffCacheKey>,
    pub dirty: dirty_flags::DirtyFlags,

    keymap: Keymap,
}

impl App {
    pub fn new() -> Result<Self> {
        let repo = Git2Repository::discover()?;
        Self::build_with_repo(Box::new(repo), Keymap::load())
    }

    #[cfg(test)]
    pub fn from_repo(repo: Box<dyn GitRepository>) -> Result<Self> {
        Self::build_with_repo(repo, Keymap::default())
    }

    fn build_with_repo(repo: Box<dyn GitRepository>, keymap: Keymap) -> Result<Self> {
        let status = repo.status()?;

        // Comment in English.
        let expanded_dirs = refresh::collect_all_dirs(&status);
        let file_tree_nodes = FileTree::from_git_status_with_expanded(
            &status.unstaged,
            &status.untracked,
            &status.staged,
            &expanded_dirs,
        );

        let branches = repo.branches().unwrap_or_default();
        let commits = repo.commits(100).unwrap_or_default();
        let stashes = repo.stashes().unwrap_or_default();

        let mut app = Self {
            running: true,
            active_panel: SidePanel::Files,
            repo,
            status,
            files: FilesPanelState {
                panel: PanelState::new(),
                tree_nodes: file_tree_nodes,
                expanded_dirs,
                visual_mode: false,
                visual_anchor: None,
            },
            branches: BranchesPanelState {
                panel: PanelState::new(),
                items: branches,
                is_fetching_remote: false,
            },
            commits: CommitsPanelState {
                panel: PanelState::new(),
                items: commits,
                dirty: false,
                tree_mode: TreeModeState::default(),
            },
            stash: StashPanelState {
                panel: PanelState::new(),
                items: stashes,
                tree_mode: TreeModeState::default(),
            },
            command_log: Vec::new(),
            current_diff: Vec::new(),
            diff_scroll: 0,
            input_mode: None,
            input_buffer: String::new(),
            commit_message_buffer: String::new(),
            commit_description_buffer: String::new(),
            commit_focus: CommitFieldFocus::Message,
            stash_message_buffer: String::new(),
            stash_targets: Vec::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            search_scope: SearchScopeKey {
                panel: SidePanel::Files,
                commit_tree_mode: false,
                stash_tree_mode: false,
            },
            search_queries: HashMap::new(),
            pending_refresh: None,
            pending_diff_reload: false,
            pending_diff_reload_at: None,
            diff_cache: diff_cache::DiffCache::new(),
            last_diff_key: None,
            dirty: dirty_flags::DirtyFlags::default(),
            keymap,
        };
        app.dirty.mark();
        app.reload_diff_now();
        Ok(app)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<super::Message> {
        use super::Message;
        if self.input_mode.is_some() {
            return self.handle_input_key(key);
        }
        if key.code == KeyCode::Esc && self.has_search_query_for_active_scope() {
            return Some(Message::SearchClear);
        }
        if key.code == KeyCode::Esc
            && self.active_panel == SidePanel::Files
            && self.files.visual_mode
        {
            return Some(Message::ToggleVisualSelectMode);
        }
        if key.code == KeyCode::Esc
            && ((self.active_panel == SidePanel::Stash && self.stash.tree_mode.active)
                || (self.active_panel == SidePanel::Commits && self.commits.tree_mode.active))
        {
            return Some(Message::RevisionCloseTree);
        }

        let k = key_to_string(&key);
        if k.is_empty() {
            return None;
        }

        let gm = |action| self.keymap.global_matches(action, &k);

        // Comment in English.
        if gm("quit") {
            return Some(Message::Quit);
        }
        if gm("list_up") {
            return Some(Message::ListUp);
        }
        if gm("list_down") {
            return Some(Message::ListDown);
        }
        if gm("panel_next") {
            return Some(Message::PanelNext);
        }
        if gm("panel_prev") {
            return Some(Message::PanelPrev);
        }
        if gm("refresh") {
            return Some(Message::RefreshStatus);
        }
        if gm("diff_scroll_up") {
            return Some(Message::DiffScrollUp);
        }
        if gm("diff_scroll_down") {
            return Some(Message::DiffScrollDown);
        }
        if gm("panel_1") {
            return Some(Message::PanelGoto(1));
        }
        if gm("panel_2") {
            return Some(Message::PanelGoto(2));
        }
        if gm("panel_3") {
            return Some(Message::PanelGoto(3));
        }
        if gm("panel_4") {
            return Some(Message::PanelGoto(4));
        }
        if gm("commit") {
            if self.active_panel == SidePanel::Files && self.files.visual_mode {
                return Some(Message::PrepareCommitFromSelection);
            }
            return Some(Message::StartCommitInput);
        }
        if gm("search_start") {
            return Some(Message::StartSearchInput);
        }
        if gm("search_next") && self.has_search_for_active_scope() {
            return Some(Message::SearchNext);
        }
        if gm("search_prev") && self.has_search_for_active_scope() {
            return Some(Message::SearchPrev);
        }

        // Comment in English.
        let panel = self.active_panel_name();
        let pm = |action| self.keymap.panel_matches(panel, action, &k);

        if pm("toggle_stage") {
            if self.active_panel == SidePanel::Files && self.files.visual_mode {
                return Some(Message::ToggleStageSelection);
            }
            if let Some(msg) = self.toggle_stage_for_selected_file() {
                return Some(msg);
            }
        }
        if pm("discard") && self.active_panel == SidePanel::Files {
            if self.files.visual_mode {
                return Some(Message::DiscardSelection);
            }
            let paths = self.prepare_discard_targets_from_selection();
            if paths.is_empty() {
                return None;
            }
            return Some(Message::DiscardPaths(paths));
        }
        if pm("stash_push") {
            return Some(Message::StartStashInput);
        }
        if pm("toggle_visual_select") {
            return Some(Message::ToggleVisualSelectMode);
        }
        if pm("toggle_dir") {
            return Some(Message::ToggleDir);
        }
        if pm("collapse_all") {
            return Some(Message::CollapseAll);
        }
        if pm("expand_all") {
            return Some(Message::ExpandAll);
        }
        if pm("checkout_branch") {
            return Some(Message::CheckoutSelectedBranch);
        }
        if pm("create_branch") {
            return Some(Message::StartBranchCreateInput);
        }
        if pm("delete_branch") {
            return Some(Message::DeleteSelectedBranch);
        }
        if pm("fetch_remote") {
            return Some(Message::FetchRemote);
        }
        if pm("open_tree") {
            return Some(Message::RevisionOpenTreeOrToggleDir);
        }
        if pm("stash_apply") {
            return Some(Message::StashApplySelected);
        }
        if pm("stash_pop") {
            return Some(Message::StashPopSelected);
        }
        if pm("stash_drop") {
            return Some(Message::StashDropSelected);
        }

        None
    }

    pub fn render(&self, frame: &mut Frame) {
        render_layout(frame, self);
    }

    fn apply_refresh(&mut self, kind: RefreshKind) -> Result<()> {
        self.status = self.repo.status()?;
        let new_dirs = refresh::collect_all_dirs(&self.status);
        for d in new_dirs {
            self.files.expanded_dirs.insert(d);
        }
        self.rebuild_tree();

        if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full) {
            self.branches.items = self.repo.branches().unwrap_or_default();
            self.stash.items = self.repo.stashes().unwrap_or_default();
            if self.stash.tree_mode.active {
                if let Some(index) = self.stash.tree_mode.selected_source {
                    if self.stash.items.iter().any(|s| s.index == index) {
                        self.stash.tree_mode.files =
                            self.repo.stash_files(index).unwrap_or_default();
                        revision_tree::rebuild_tree_nodes(
                            &self.stash.tree_mode.files,
                            &self.stash.tree_mode.expanded_dirs,
                            &mut self.stash.tree_mode.nodes,
                            &mut self.stash.panel.list_state,
                        );
                    } else {
                        self.stash_close_tree();
                    }
                } else {
                    self.stash_close_tree();
                }
            }
        }

        if matches!(kind, RefreshKind::Full) {
            self.commits.dirty = true;
            if self.should_load_commits_now() {
                self.reload_commits_now();
            }
        }

        Ok(())
    }

    pub fn request_refresh(&mut self, kind: RefreshKind) {
        if matches!(kind, RefreshKind::Full) {
            self.commits.dirty = true;
        }
        self.diff_cache.invalidate_files();
        self.last_diff_key = None;
        self.pending_refresh = Some(match self.pending_refresh {
            None => kind,
            Some(existing) => Self::max_refresh_kind(existing, kind),
        });
    }

    pub fn flush_pending_refresh(&mut self) -> Result<bool> {
        let Some(kind) = self.pending_refresh.take() else {
            return Ok(false);
        };
        self.apply_refresh(kind)?;
        self.reload_diff_now();
        Ok(true)
    }

    pub fn schedule_diff_reload(&mut self) {
        self.pending_diff_reload = true;
        self.pending_diff_reload_at = Some(Instant::now());
    }

    pub fn has_pending_diff_reload(&self) -> bool {
        self.pending_diff_reload
    }

    pub fn diff_reload_debounce_elapsed(&self, debounce: Duration) -> bool {
        self.pending_diff_reload_at
            .is_some_and(|requested_at| requested_at.elapsed() >= debounce)
    }

    pub fn flush_pending_diff_reload(&mut self) {
        if !self.pending_diff_reload {
            return;
        }
        self.reload_diff_now();
        self.dirty.mark();
    }

    pub(super) fn pending_refresh_kind(&self) -> Option<RefreshKind> {
        self.pending_refresh
    }

    pub fn ensure_commits_loaded_for_active_panel(&mut self) {
        if self.active_panel == SidePanel::Commits && self.commits.dirty {
            self.reload_commits_now();
            self.reload_diff_now();
        }
    }

    pub fn toggle_visual_select_mode(&mut self) {
        if self.active_panel != SidePanel::Files {
            return;
        }
        if self.files.visual_mode {
            self.files.visual_mode = false;
            self.files.visual_anchor = None;
            return;
        }

        self.files.visual_mode = true;
        self.files.visual_anchor = self.files.panel.list_state.selected();
    }

    pub fn push_log<S: Into<String>>(&mut self, command: S, success: bool) {
        self.command_log.push(CommandLogEntry {
            command: command.into(),
            success,
        });
        const MAX_LOG_ENTRIES: usize = 200;
        if self.command_log.len() > MAX_LOG_ENTRIES {
            let drain_count = self.command_log.len() - MAX_LOG_ENTRIES;
            self.command_log.drain(0..drain_count);
        }
    }

    pub fn stage_file(&mut self, path: PathBuf) -> Result<()> {
        self.repo.stage(&path)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    pub fn unstage_file(&mut self, path: PathBuf) -> Result<()> {
        self.repo.unstage(&path)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    pub fn discard_paths(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.repo.discard_paths(paths)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    pub(super) fn stage_paths_internal(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.repo.stage_paths(paths)?;
        Ok(())
    }

    pub(super) fn unstage_paths_internal(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.repo.unstage_paths(paths)?;
        Ok(())
    }

    pub fn commit(&mut self, message: &str) -> Result<String> {
        let oid = self.repo.commit(message)?;
        self.request_refresh(RefreshKind::Full);
        Ok(oid)
    }

    pub fn create_branch(&mut self, name: &str) -> Result<()> {
        self.repo.create_branch(name)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        self.repo.checkout_branch(name)?;
        self.request_refresh(RefreshKind::Full);
        Ok(())
    }

    pub fn delete_branch(&mut self, name: &str) -> Result<()> {
        self.repo.delete_branch(name)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn fetch_remote_async(&self) -> Result<Receiver<super::Message>> {
        let repo_rx = self.repo.fetch_default_async()?;
        let (msg_tx, msg_rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let message = match repo_rx.recv() {
                Ok(Ok(remote)) => super::Message::FetchRemoteFinished(Ok(remote)),
                Ok(Err(err)) => super::Message::FetchRemoteFinished(Err(err.to_string())),
                Err(err) => super::Message::FetchRemoteFinished(Err(err.to_string())),
            };
            let _ = msg_tx.send(message);
        });
        Ok(msg_rx)
    }

    pub fn stash_push(&mut self, paths: &[PathBuf], message: &str) -> Result<usize> {
        let index = self.repo.stash_push_paths(paths, message)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(index)
    }

    pub fn stash_apply(&mut self, index: usize) -> Result<()> {
        self.repo.stash_apply(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn stash_pop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_pop(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn stash_drop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_drop(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    /// Documentation comment in English.
    pub fn toggle_selected_dir(&mut self) {
        let selected_dir_path = self.selected_tree_node().and_then(|node| {
            if node.is_dir {
                Some(node.path.clone())
            } else {
                None
            }
        });
        refresh::toggle_selected_dir(&mut self.files.expanded_dirs, selected_dir_path);
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn collapse_all(&mut self) {
        refresh::collapse_all(&mut self.files.expanded_dirs);
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn expand_all(&mut self) {
        refresh::expand_all(&mut self.files.expanded_dirs, &self.status);
        self.rebuild_tree();
    }

    pub fn diff_scroll_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(10);
    }

    pub fn diff_scroll_down(&mut self) {
        let max = self.current_diff.len().saturating_sub(1);
        self.diff_scroll = (self.diff_scroll + 10).min(max);
    }

    fn rebuild_tree(&mut self) {
        refresh::rebuild_tree(
            &self.status,
            &self.files.expanded_dirs,
            &mut self.files.tree_nodes,
            &mut self.files.panel,
            &mut self.files.visual_anchor,
        );
    }

    pub fn stash_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.active_panel != SidePanel::Stash {
            return Ok(());
        }

        if !self.stash.tree_mode.active {
            let Some(index) = self.selected_stash_index() else {
                return Ok(());
            };
            let files = self.repo.stash_files(index)?;
            revision_tree::enter_tree_mode(
                index,
                files,
                revision_tree::TreeModeState {
                    tree_mode: &mut self.stash.tree_mode.active,
                    tree_nodes: &mut self.stash.tree_mode.nodes,
                    tree_files: &mut self.stash.tree_mode.files,
                    expanded_dirs: &mut self.stash.tree_mode.expanded_dirs,
                    selected_tree_revision: &mut self.stash.tree_mode.selected_source,
                    list_state: &mut self.stash.panel.list_state,
                },
            );
            return Ok(());
        }

        let selected_dir_path = self.selected_stash_tree_node().and_then(|node| {
            if node.is_dir {
                Some(node.path.clone())
            } else {
                None
            }
        });
        revision_tree::toggle_tree_dir(
            selected_dir_path,
            &self.stash.tree_mode.files,
            &mut self.stash.tree_mode.expanded_dirs,
            &mut self.stash.tree_mode.nodes,
            &mut self.stash.panel.list_state,
        );
        Ok(())
    }

    pub fn stash_close_tree(&mut self) {
        let selected_source_index = self
            .stash
            .tree_mode
            .selected_source
            .and_then(|stash_index| self.stash.items.iter().position(|s| s.index == stash_index));

        let was_open = self.stash.tree_mode.active;
        revision_tree::close_tree_mode(
            &mut self.stash.tree_mode.active,
            &mut self.stash.tree_mode.nodes,
            &mut self.stash.tree_mode.files,
            &mut self.stash.tree_mode.expanded_dirs,
            &mut self.stash.panel.list_state,
            selected_source_index,
            self.stash.items.len(),
        );
        if was_open {
            self.stash.tree_mode.selected_source = None;
        }
    }

    pub fn reload_diff_now(&mut self) {
        let target = self.selected_diff_target();
        let key = self.diff_target_to_cache_key(&target);

        // Check if same as last load
        if let Some(ref last) = self.last_diff_key {
            if last == &key {
                self.pending_diff_reload = false;
                self.pending_diff_reload_at = None;
                return;
            }
        }

        self.diff_scroll = 0;

        // Try cache first
        if let Some(cached) = self.diff_cache.get_cloned(&key) {
            self.current_diff = cached;
        } else {
            let diff = diff_loader::load_diff(self.repo.as_ref(), target);
            self.diff_cache.insert(key.clone(), diff.clone());
            self.current_diff = diff;
        }

        self.last_diff_key = Some(key);
        self.pending_diff_reload = false;
        self.pending_diff_reload_at = None;
    }

    fn diff_target_to_cache_key(
        &self,
        target: &diff_loader::DiffTarget,
    ) -> diff_cache::DiffCacheKey {
        use crate::ui::widgets::file_tree::FileTreeNodeStatus;
        use diff_loader::DiffTarget;

        match target {
            DiffTarget::File { path, status } => {
                let is_staged = matches!(status, FileTreeNodeStatus::Staged(_));
                diff_cache::DiffCacheKey::File {
                    path: path.clone(),
                    is_staged,
                }
            }
            DiffTarget::Directory { path } => {
                let hash = self
                    .files
                    .tree_nodes
                    .iter()
                    .filter(|n| n.path.starts_with(path))
                    .map(|n| n.path.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("|")
                    .bytes()
                    .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
                diff_cache::DiffCacheKey::Directory {
                    path: path.clone(),
                    files_hash: hash,
                }
            }
            DiffTarget::Commit { oid, path } => diff_cache::DiffCacheKey::Commit {
                oid: oid.clone(),
                path: path.clone(),
            },
            DiffTarget::Stash { index, path } => diff_cache::DiffCacheKey::Stash {
                index: *index,
                path: path.clone(),
            },
            DiffTarget::None => diff_cache::DiffCacheKey::File {
                path: PathBuf::new(),
                is_staged: false,
            },
        }
    }

    pub fn commit_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.active_panel != SidePanel::Commits {
            return Ok(());
        }
        if self.commits.dirty {
            self.reload_commits_now();
        }

        if !self.commits.tree_mode.active {
            let Some(oid) = self.selected_commit_oid() else {
                return Ok(());
            };
            let files = self.repo.commit_files(&oid)?;
            revision_tree::enter_tree_mode(
                oid,
                files,
                revision_tree::TreeModeState {
                    tree_mode: &mut self.commits.tree_mode.active,
                    tree_nodes: &mut self.commits.tree_mode.nodes,
                    tree_files: &mut self.commits.tree_mode.files,
                    expanded_dirs: &mut self.commits.tree_mode.expanded_dirs,
                    selected_tree_revision: &mut self.commits.tree_mode.selected_source,
                    list_state: &mut self.commits.panel.list_state,
                },
            );
            return Ok(());
        }

        let selected_dir_path = self.selected_commit_tree_node().and_then(|node| {
            if node.is_dir {
                Some(node.path.clone())
            } else {
                None
            }
        });
        revision_tree::toggle_tree_dir(
            selected_dir_path,
            &self.commits.tree_mode.files,
            &mut self.commits.tree_mode.expanded_dirs,
            &mut self.commits.tree_mode.nodes,
            &mut self.commits.panel.list_state,
        );
        Ok(())
    }

    pub fn commit_close_tree(&mut self) {
        let selected_source_index = self
            .commits
            .tree_mode
            .selected_source
            .as_ref()
            .and_then(|oid| self.commits.items.iter().position(|c| c.oid == *oid));

        let was_open = self.commits.tree_mode.active;
        revision_tree::close_tree_mode(
            &mut self.commits.tree_mode.active,
            &mut self.commits.tree_mode.nodes,
            &mut self.commits.tree_mode.files,
            &mut self.commits.tree_mode.expanded_dirs,
            &mut self.commits.panel.list_state,
            selected_source_index,
            self.commits.items.len(),
        );
        if was_open {
            self.commits.tree_mode.selected_source = None;
        }
    }

    pub(super) fn global_key_or(&self, action: &str, fallback: &str) -> String {
        self.keymap
            .first_global_key(action)
            .unwrap_or_else(|| fallback.to_string())
    }

    pub(super) fn panel_key_or(&self, panel: &str, action: &str, fallback: &str) -> String {
        self.keymap
            .first_panel_key(panel, action)
            .unwrap_or_else(|| fallback.to_string())
    }

    fn max_refresh_kind(a: RefreshKind, b: RefreshKind) -> RefreshKind {
        use RefreshKind::*;
        match (a, b) {
            (Full, _) | (_, Full) => Full,
            (StatusAndRefs, _) | (_, StatusAndRefs) => StatusAndRefs,
            _ => StatusOnly,
        }
    }

    fn should_load_commits_now(&self) -> bool {
        self.active_panel == SidePanel::Commits || self.commits.tree_mode.active
    }

    fn reload_commits_now(&mut self) {
        self.commits.items = self.repo.commits(100).unwrap_or_default();
        self.commits.dirty = false;
        if self.commits.tree_mode.active {
            if let Some(ref oid) = self.commits.tree_mode.selected_source {
                if self.commits.items.iter().any(|c| c.oid == *oid) {
                    self.commits.tree_mode.files = self.repo.commit_files(oid).unwrap_or_default();
                    revision_tree::rebuild_tree_nodes(
                        &self.commits.tree_mode.files,
                        &self.commits.tree_mode.expanded_dirs,
                        &mut self.commits.tree_mode.nodes,
                        &mut self.commits.panel.list_state,
                    );
                } else {
                    self.commit_close_tree();
                }
            } else {
                self.commit_close_tree();
            }
        }
    }
}
