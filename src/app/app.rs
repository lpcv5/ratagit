use super::{diff_loader, refresh, revision_tree};
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
use std::collections::HashSet;
use std::path::PathBuf;

/// Documentation comment in English.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitFieldFocus {
    Message,
    Description,
}

/// Documentation comment in English.
pub struct App {
    pub running: bool,
    pub active_panel: SidePanel,

    repo: Box<dyn GitRepository>,
    pub status: GitStatus,

    /// Documentation comment in English.
    pub file_tree_nodes: Vec<FileTreeNode>,
    /// Documentation comment in English.
    expanded_dirs: HashSet<PathBuf>,

    pub files_panel: PanelState,
    pub files_visual_mode: bool,
    pub files_visual_anchor: Option<usize>,
    pub branches_panel: PanelState,
    pub commits_panel: PanelState,
    pub stash_panel: PanelState,

    pub command_log: Vec<CommandLogEntry>,
    pub branches: Vec<BranchInfo>,
    pub commits: Vec<CommitInfo>,
    pub commit_tree_mode: bool,
    pub commit_tree_nodes: Vec<FileTreeNode>,
    commit_tree_files: Vec<FileEntry>,
    commit_tree_expanded_dirs: HashSet<PathBuf>,
    pub commit_tree_commit_oid: Option<String>,
    pub stashes: Vec<StashInfo>,
    pub stash_tree_mode: bool,
    pub stash_tree_nodes: Vec<FileTreeNode>,
    stash_tree_files: Vec<FileEntry>,
    stash_tree_expanded_dirs: HashSet<PathBuf>,
    pub stash_tree_stash_index: Option<usize>,
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
        let commits = repo.commits(200).unwrap_or_default();
        let stashes = repo.stashes().unwrap_or_default();

        let mut app = Self {
            running: true,
            active_panel: SidePanel::Files,
            repo,
            status,
            file_tree_nodes,
            expanded_dirs,
            files_panel: PanelState::new(),
            files_visual_mode: false,
            files_visual_anchor: None,
            branches_panel: PanelState::new(),
            commits_panel: PanelState::new(),
            stash_panel: PanelState::new(),
            command_log: Vec::new(),
            branches,
            commits,
            commit_tree_mode: false,
            commit_tree_nodes: Vec::new(),
            commit_tree_files: Vec::new(),
            commit_tree_expanded_dirs: HashSet::new(),
            commit_tree_commit_oid: None,
            stashes,
            stash_tree_mode: false,
            stash_tree_nodes: Vec::new(),
            stash_tree_files: Vec::new(),
            stash_tree_expanded_dirs: HashSet::new(),
            stash_tree_stash_index: None,
            current_diff: Vec::new(),
            diff_scroll: 0,
            input_mode: None,
            input_buffer: String::new(),
            commit_message_buffer: String::new(),
            commit_description_buffer: String::new(),
            commit_focus: CommitFieldFocus::Message,
            stash_message_buffer: String::new(),
            stash_targets: Vec::new(),
            keymap,
        };
        app.load_diff();
        Ok(app)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<super::Message> {
        use super::Message;
        if self.input_mode.is_some() {
            return self.handle_input_key(key);
        }
        if key.code == KeyCode::Esc
            && self.active_panel == SidePanel::Files
            && self.files_visual_mode
        {
            return Some(Message::ToggleVisualSelectMode);
        }
        if key.code == KeyCode::Esc
            && ((self.active_panel == SidePanel::Stash && self.stash_tree_mode)
                || (self.active_panel == SidePanel::Commits && self.commit_tree_mode))
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
            if self.active_panel == SidePanel::Files && self.files_visual_mode {
                return Some(Message::PrepareCommitFromSelection);
            }
            return Some(Message::StartCommitInput);
        }

        // Comment in English.
        let panel = self.active_panel_name();
        let pm = |action| self.keymap.panel_matches(panel, action, &k);

        if pm("toggle_stage") {
            if self.active_panel == SidePanel::Files && self.files_visual_mode {
                return Some(Message::ToggleStageSelection);
            }
            if let Some(msg) = self.toggle_stage_for_selected_file() {
                return Some(msg);
            }
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

    pub fn refresh_status(&mut self) -> Result<()> {
        self.status = self.repo.status()?;
        let new_dirs = refresh::collect_all_dirs(&self.status);
        for d in new_dirs {
            self.expanded_dirs.insert(d);
        }
        self.rebuild_tree();
        self.branches = self.repo.branches().unwrap_or_default();
        self.commits = self.repo.commits(200).unwrap_or_default();
        if self.commit_tree_mode {
            if let Some(ref oid) = self.commit_tree_commit_oid {
                if self.commits.iter().any(|c| c.oid == *oid) {
                    self.commit_tree_files = self.repo.commit_files(oid).unwrap_or_default();
                    revision_tree::rebuild_tree_nodes(
                        &self.commit_tree_files,
                        &self.commit_tree_expanded_dirs,
                        &mut self.commit_tree_nodes,
                        &mut self.commits_panel.list_state,
                    );
                } else {
                    self.commit_close_tree();
                }
            } else {
                self.commit_close_tree();
            }
        }
        self.stashes = self.repo.stashes().unwrap_or_default();
        if self.stash_tree_mode {
            if let Some(index) = self.stash_tree_stash_index {
                if self.stashes.iter().any(|s| s.index == index) {
                    self.stash_tree_files = self.repo.stash_files(index).unwrap_or_default();
                    revision_tree::rebuild_tree_nodes(
                        &self.stash_tree_files,
                        &self.stash_tree_expanded_dirs,
                        &mut self.stash_tree_nodes,
                        &mut self.stash_panel.list_state,
                    );
                } else {
                    self.stash_close_tree();
                }
            } else {
                self.stash_close_tree();
            }
        }
        Ok(())
    }

    pub fn toggle_visual_select_mode(&mut self) {
        if self.active_panel != SidePanel::Files {
            return;
        }
        if self.files_visual_mode {
            self.files_visual_mode = false;
            self.files_visual_anchor = None;
            return;
        }

        self.files_visual_mode = true;
        self.files_visual_anchor = self.files_panel.list_state.selected();
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
        self.refresh_status()?;
        Ok(())
    }

    pub fn unstage_file(&mut self, path: PathBuf) -> Result<()> {
        self.repo.unstage(&path)?;
        self.refresh_status()?;
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
        self.refresh_status()?;
        Ok(oid)
    }

    pub fn create_branch(&mut self, name: &str) -> Result<()> {
        self.repo.create_branch(name)?;
        self.refresh_status()?;
        Ok(())
    }

    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        self.repo.checkout_branch(name)?;
        self.refresh_status()?;
        Ok(())
    }

    pub fn delete_branch(&mut self, name: &str) -> Result<()> {
        self.repo.delete_branch(name)?;
        self.refresh_status()?;
        Ok(())
    }

    pub fn fetch_remote(&mut self) -> Result<String> {
        let remote = self.repo.fetch_default()?;
        self.refresh_status()?;
        Ok(remote)
    }

    pub fn stash_push(&mut self, paths: &[PathBuf], message: &str) -> Result<usize> {
        let index = self.repo.stash_push_paths(paths, message)?;
        self.refresh_status()?;
        Ok(index)
    }

    pub fn stash_apply(&mut self, index: usize) -> Result<()> {
        self.repo.stash_apply(index)?;
        self.refresh_status()?;
        Ok(())
    }

    pub fn stash_pop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_pop(index)?;
        self.refresh_status()?;
        Ok(())
    }

    pub fn stash_drop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_drop(index)?;
        self.refresh_status()?;
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
        refresh::toggle_selected_dir(&mut self.expanded_dirs, selected_dir_path);
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn collapse_all(&mut self) {
        refresh::collapse_all(&mut self.expanded_dirs);
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn expand_all(&mut self) {
        refresh::expand_all(&mut self.expanded_dirs, &self.status);
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
            &self.expanded_dirs,
            &mut self.file_tree_nodes,
            &mut self.files_panel,
            &mut self.files_visual_anchor,
        );
    }

    pub fn stash_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.active_panel != SidePanel::Stash {
            return Ok(());
        }

        if !self.stash_tree_mode {
            let Some(index) = self.selected_stash_index() else {
                return Ok(());
            };
            let files = self.repo.stash_files(index)?;
            revision_tree::enter_tree_mode(
                index,
                files,
                revision_tree::TreeModeState {
                    tree_mode: &mut self.stash_tree_mode,
                    tree_nodes: &mut self.stash_tree_nodes,
                    tree_files: &mut self.stash_tree_files,
                    expanded_dirs: &mut self.stash_tree_expanded_dirs,
                    selected_tree_revision: &mut self.stash_tree_stash_index,
                    list_state: &mut self.stash_panel.list_state,
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
            &self.stash_tree_files,
            &mut self.stash_tree_expanded_dirs,
            &mut self.stash_tree_nodes,
            &mut self.stash_panel.list_state,
        );
        Ok(())
    }

    pub fn stash_close_tree(&mut self) {
        let selected_source_index = self
            .stash_tree_stash_index
            .and_then(|stash_index| self.stashes.iter().position(|s| s.index == stash_index));

        let was_open = self.stash_tree_mode;
        revision_tree::close_tree_mode(
            &mut self.stash_tree_mode,
            &mut self.stash_tree_nodes,
            &mut self.stash_tree_files,
            &mut self.stash_tree_expanded_dirs,
            &mut self.stash_panel.list_state,
            selected_source_index,
            self.stashes.len(),
        );
        if was_open {
            self.stash_tree_stash_index = None;
        }
    }

    pub fn load_diff(&mut self) {
        self.diff_scroll = 0;
        let target = self.selected_diff_target();
        self.current_diff =
            diff_loader::load_diff(self.repo.as_ref(), &self.file_tree_nodes, target);
    }

    pub fn commit_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.active_panel != SidePanel::Commits {
            return Ok(());
        }

        if !self.commit_tree_mode {
            let Some(oid) = self.selected_commit_oid() else {
                return Ok(());
            };
            let files = self.repo.commit_files(&oid)?;
            revision_tree::enter_tree_mode(
                oid,
                files,
                revision_tree::TreeModeState {
                    tree_mode: &mut self.commit_tree_mode,
                    tree_nodes: &mut self.commit_tree_nodes,
                    tree_files: &mut self.commit_tree_files,
                    expanded_dirs: &mut self.commit_tree_expanded_dirs,
                    selected_tree_revision: &mut self.commit_tree_commit_oid,
                    list_state: &mut self.commits_panel.list_state,
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
            &self.commit_tree_files,
            &mut self.commit_tree_expanded_dirs,
            &mut self.commit_tree_nodes,
            &mut self.commits_panel.list_state,
        );
        Ok(())
    }

    pub fn commit_close_tree(&mut self) {
        let selected_source_index = self
            .commit_tree_commit_oid
            .as_ref()
            .and_then(|oid| self.commits.iter().position(|c| c.oid == *oid));

        let was_open = self.commit_tree_mode;
        revision_tree::close_tree_mode(
            &mut self.commit_tree_mode,
            &mut self.commit_tree_nodes,
            &mut self.commit_tree_files,
            &mut self.commit_tree_expanded_dirs,
            &mut self.commits_panel.list_state,
            selected_source_index,
            self.commits.len(),
        );
        if was_open {
            self.commit_tree_commit_oid = None;
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
}
