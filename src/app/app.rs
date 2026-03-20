use crate::config::keymap::{key_to_string, Keymap};
use crate::git::{Git2Repository, GitRepository, GitStatus, DiffLine, BranchInfo, CommitInfo, StashInfo, FileEntry};
use crate::ui::layout::render_layout;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeNodeStatus};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
        let status = repo.status()?;

        // Comment in English.
        let expanded_dirs = collect_all_dirs(&status);
        let file_tree_nodes = FileTree::from_git_status_with_expanded(
            &status.unstaged,
            &status.untracked,
            &status.staged,
            &expanded_dirs,
        );

        let keymap = Keymap::load();

        let branches = repo.branches().unwrap_or_default();
        let commits = repo.commits(200).unwrap_or_default();
        let stashes = repo.stashes().unwrap_or_default();

        let mut app = Self {
            running: true,
            active_panel: SidePanel::Files,
            repo: Box::new(repo),
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
        if key.code == KeyCode::Esc && self.active_panel == SidePanel::Files && self.files_visual_mode {
            return Some(Message::ToggleVisualSelectMode);
        }
        if key.code == KeyCode::Esc
            && ((self.active_panel == SidePanel::Stash && self.stash_tree_mode)
                || (self.active_panel == SidePanel::Commits && self.commit_tree_mode))
        {
            return Some(Message::RevisionCloseTree);
        }

        let k = key_to_string(&key);
        if k.is_empty() { return None; }

        let gm = |action| self.keymap.global_matches(action, &k);

        // Comment in English.
        if gm("quit")              { return Some(Message::Quit); }
        if gm("list_up")           { return Some(Message::ListUp); }
        if gm("list_down")         { return Some(Message::ListDown); }
        if gm("panel_next")        { return Some(Message::PanelNext); }
        if gm("panel_prev")        { return Some(Message::PanelPrev); }
        if gm("refresh")           { return Some(Message::RefreshStatus); }
        if gm("diff_scroll_up")    { return Some(Message::DiffScrollUp); }
        if gm("diff_scroll_down")  { return Some(Message::DiffScrollDown); }
        if gm("panel_1")           { return Some(Message::PanelGoto(1)); }
        if gm("panel_2")           { return Some(Message::PanelGoto(2)); }
        if gm("panel_3")           { return Some(Message::PanelGoto(3)); }
        if gm("panel_4")           { return Some(Message::PanelGoto(4)); }
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
        if pm("stash_push") { return Some(Message::StartStashInput); }
        if pm("toggle_visual_select") { return Some(Message::ToggleVisualSelectMode); }
        if pm("toggle_dir")   { return Some(Message::ToggleDir); }
        if pm("collapse_all") { return Some(Message::CollapseAll); }
        if pm("expand_all")   { return Some(Message::ExpandAll); }
        if pm("checkout_branch") { return Some(Message::CheckoutSelectedBranch); }
        if pm("create_branch") { return Some(Message::StartBranchCreateInput); }
        if pm("delete_branch") { return Some(Message::DeleteSelectedBranch); }
        if pm("fetch_remote") { return Some(Message::FetchRemote); }
        if pm("open_tree") { return Some(Message::RevisionOpenTreeOrToggleDir); }
        if pm("stash_apply") { return Some(Message::StashApplySelected); }
        if pm("stash_pop") { return Some(Message::StashPopSelected); }
        if pm("stash_drop") { return Some(Message::StashDropSelected); }

        None
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> Option<super::Message> {
        use super::Message;
        let mode = self.input_mode?;

        match key.code {
            KeyCode::Esc => {
                self.cancel_input();
                None
            }
            KeyCode::Tab => match mode {
                InputMode::CommitEditor => {
                    self.commit_focus = match self.commit_focus {
                        CommitFieldFocus::Message => CommitFieldFocus::Description,
                        CommitFieldFocus::Description => CommitFieldFocus::Message,
                    };
                    None
                }
                InputMode::CreateBranch | InputMode::StashEditor => None,
            },
            KeyCode::Enter => match mode {
                InputMode::CommitEditor => match self.commit_focus {
                    CommitFieldFocus::Message => {
                        let title = self.commit_message_buffer.trim().to_string();
                        if title.is_empty() {
                            self.push_log("Empty commit message ignored", false);
                            return None;
                        }
                        let description = self.commit_description_buffer.trim_end();
                        let value = if description.is_empty() {
                            title
                        } else {
                            format!("{}\n\n{}", title, description)
                        };
                        self.input_mode = None;
                        self.commit_message_buffer.clear();
                        self.commit_description_buffer.clear();
                        self.commit_focus = CommitFieldFocus::Message;
                        Some(Message::Commit(value))
                    }
                    CommitFieldFocus::Description => {
                        self.commit_description_buffer.push('\n');
                        None
                    }
                },
                InputMode::CreateBranch => {
                    let value = self.input_buffer.trim().to_string();
                    self.input_mode = None;
                    self.input_buffer.clear();

                    if value.is_empty() {
                        self.push_log("Empty input ignored", false);
                        return None;
                    }
                    Some(Message::CreateBranch(value))
                }
                InputMode::StashEditor => {
                    let value = self.stash_message_buffer.trim().to_string();
                    let paths = self.stash_targets.clone();
                    self.input_mode = None;
                    self.stash_message_buffer.clear();
                    self.stash_targets.clear();

                    if value.is_empty() {
                        self.push_log("Empty stash title ignored", false);
                        return None;
                    }
                    if paths.is_empty() {
                        self.push_log("stash blocked: no selected items", false);
                        return None;
                    }
                    Some(Message::StashPush {
                        message: value,
                        paths,
                    })
                }
            },
            KeyCode::Backspace => match mode {
                InputMode::CommitEditor => {
                    match self.commit_focus {
                        CommitFieldFocus::Message => {
                            self.commit_message_buffer.pop();
                        }
                        CommitFieldFocus::Description => {
                            self.commit_description_buffer.pop();
                        }
                    }
                    None
                }
                InputMode::CreateBranch => {
                    self.input_buffer.pop();
                    None
                }
                InputMode::StashEditor => {
                    self.stash_message_buffer.pop();
                    None
                }
            },
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return None;
                }
                match mode {
                    InputMode::CommitEditor => {
                        match self.commit_focus {
                            CommitFieldFocus::Message => self.commit_message_buffer.push(c),
                            CommitFieldFocus::Description => self.commit_description_buffer.push(c),
                        }
                        None
                    }
                    InputMode::CreateBranch => {
                        self.input_buffer.push(c);
                        None
                    }
                    InputMode::StashEditor => {
                        self.stash_message_buffer.push(c);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        render_layout(frame, self);
    }

    pub fn shortcut_hints(&self) -> Vec<(String, String)> {
        if let Some(mode) = self.input_mode {
            return match mode {
                InputMode::CommitEditor => vec![
                    ("Tab".to_string(), "SwitchField".to_string()),
                    ("Enter".to_string(), "Confirm/DescNewline".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                    ("Backspace".to_string(), "Delete".to_string()),
                ],
                InputMode::CreateBranch => vec![
                    ("Enter".to_string(), "Confirm".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                    ("Backspace".to_string(), "Delete".to_string()),
                ],
                InputMode::StashEditor => vec![
                    ("Enter".to_string(), "Confirm".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                    ("Backspace".to_string(), "Delete".to_string()),
                ],
            };
        }

        let mut hints = vec![
            (
                format!(
                    "{}/{}",
                    self.global_key_or("panel_prev", "h"),
                    self.global_key_or("panel_next", "l")
                ),
                "Panel".to_string(),
            ),
            (
                format!(
                    "{}/{}",
                    self.global_key_or("list_up", "k"),
                    self.global_key_or("list_down", "j")
                ),
                "Move".to_string(),
            ),
            (self.global_key_or("refresh", "r"), "Refresh".to_string()),
            (self.global_key_or("commit", "c"), "Commit".to_string()),
            (
                format!(
                    "{}/{}",
                    self.global_key_or("diff_scroll_up", "C-u"),
                    self.global_key_or("diff_scroll_down", "C-d")
                ),
                "DiffScroll".to_string(),
            ),
            (self.global_key_or("quit", "q"), "Quit".to_string()),
        ];

        let panel = self.active_panel_name();
        match self.active_panel {
            SidePanel::Files => {
                hints.push((
                    self.panel_key_or(panel, "toggle_visual_select", "v"),
                    if self.files_visual_mode {
                        "VisualOn".to_string()
                    } else {
                        "Visual".to_string()
                    },
                ));
                hints.push((
                    self.panel_key_or(panel, "toggle_stage", "Space"),
                    if self.files_visual_mode {
                        "BatchToggle".to_string()
                    } else {
                        "Stage".to_string()
                    },
                ));
                hints.push((
                    self.panel_key_or(panel, "toggle_dir", "Enter"),
                    "ToggleDir".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "collapse_all", "-"),
                    "Collapse".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "expand_all", "="),
                    "Expand".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_push", "s"),
                    "Stash".to_string(),
                ));
            }
            SidePanel::LocalBranches => {
                hints.push((
                    self.panel_key_or(panel, "checkout_branch", "Enter"),
                    "Checkout".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "create_branch", "n"),
                    "NewBranch".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "delete_branch", "d"),
                    "Delete".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "fetch_remote", "f"),
                    "Fetch".to_string(),
                ));
            }
            SidePanel::Stash => {
                hints.push((
                    self.panel_key_or(panel, "open_tree", "Enter"),
                    if self.stash_tree_mode {
                        "ToggleDir".to_string()
                    } else {
                        "Files".to_string()
                    },
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_apply", "a"),
                    "Apply".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_pop", "p"),
                    "Pop".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_drop", "d"),
                    "Drop".to_string(),
                ));
            }
            SidePanel::Commits => {
                hints.push((
                    self.panel_key_or(panel, "open_tree", "Enter"),
                    if self.commit_tree_mode {
                        "ToggleDir".to_string()
                    } else {
                        "Files".to_string()
                    },
                ));
            }
        }

        hints
    }

    pub fn refresh_status(&mut self) -> Result<()> {
        self.status = self.repo.status()?;
        let new_dirs = collect_all_dirs(&self.status);
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
                    self.rebuild_commit_tree();
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
                    self.rebuild_stash_tree();
                } else {
                    self.stash_close_tree();
                }
            } else {
                self.stash_close_tree();
            }
        }
        Ok(())
    }

    pub fn start_commit_editor(&mut self) {
        self.input_mode = Some(InputMode::CommitEditor);
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
    }

    pub fn start_commit_editor_guarded(&mut self) -> bool {
        if self.status.staged.is_empty() {
            self.push_log("commit blocked: no staged changes", false);
            return false;
        }
        self.start_commit_editor();
        true
    }

    pub fn start_branch_create_input(&mut self) {
        self.input_mode = Some(InputMode::CreateBranch);
        self.input_buffer.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = None;
        self.input_buffer.clear();
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
        self.stash_message_buffer.clear();
        self.stash_targets.clear();
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
        let Some(node) = self.selected_tree_node() else { return; };
        if !node.is_dir { return; }
        let path = node.path.clone();
        if self.expanded_dirs.contains(&path) {
            self.expanded_dirs.remove(&path);
        } else {
            self.expanded_dirs.insert(path);
        }
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn collapse_all(&mut self) {
        self.expanded_dirs.clear();
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn expand_all(&mut self) {
        self.expanded_dirs = collect_all_dirs(&self.status);
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
        let selected = self.files_panel.list_state.selected();
        self.file_tree_nodes = FileTree::from_git_status_with_expanded(
            &self.status.unstaged,
            &self.status.untracked,
            &self.status.staged,
            &self.expanded_dirs,
        );
        // Comment in English.
        let count = self.file_tree_nodes.len();
        if count == 0 {
            self.files_panel.list_state.select(None);
            self.files_visual_anchor = None;
        } else {
            let idx = selected.unwrap_or(0).min(count - 1);
            self.files_panel.list_state.select(Some(idx));
            if let Some(anchor) = self.files_visual_anchor {
                self.files_visual_anchor = Some(anchor.min(count - 1));
            }
        }
    }

    fn active_panel_count(&self) -> usize {
        match self.active_panel {
            SidePanel::Files => self.file_tree_nodes.len(),
            SidePanel::LocalBranches => self.branches.len(),
            SidePanel::Commits => {
                if self.commit_tree_mode {
                    self.commit_tree_nodes.len()
                } else {
                    self.commits.len()
                }
            }
            SidePanel::Stash => {
                if self.stash_tree_mode {
                    self.stash_tree_nodes.len()
                } else {
                    self.stashes.len()
                }
            }
        }
    }

    pub fn active_panel_state_mut(&mut self) -> &mut PanelState {
        match self.active_panel {
            SidePanel::Files => &mut self.files_panel,
            SidePanel::LocalBranches => &mut self.branches_panel,
            SidePanel::Commits => &mut self.commits_panel,
            SidePanel::Stash => &mut self.stash_panel,
        }
    }

    pub fn list_down(&mut self) {
        let count = self.active_panel_count();
        if count == 0 { return; }
        let state = self.active_panel_state_mut();
        let next = state.list_state.selected().map(|i| (i + 1).min(count - 1)).unwrap_or(0);
        state.list_state.select(Some(next));
    }

    pub fn list_up(&mut self) {
        let count = self.active_panel_count();
        if count == 0 { return; }
        let state = self.active_panel_state_mut();
        let prev = state.list_state.selected().map(|i| i.saturating_sub(1)).unwrap_or(0);
        state.list_state.select(Some(prev));
    }

    pub fn visual_selected_indices(&self) -> HashSet<usize> {
        let mut set = HashSet::new();
        if self.active_panel != SidePanel::Files || !self.files_visual_mode {
            return set;
        }
        let Some(current) = self.files_panel.list_state.selected() else {
            return set;
        };
        let anchor = self.files_visual_anchor.unwrap_or(current);
        let (start, end) = if anchor <= current {
            (anchor, current)
        } else {
            (current, anchor)
        };
        for idx in start..=end {
            set.insert(idx);
        }
        set
    }

    pub fn toggle_stage_visual_selection(&mut self) -> Result<(usize, usize)> {
        let selected = self.visual_selected_indices();
        if selected.is_empty() {
            return Ok((0, 0));
        }

        let (stage_paths, unstage_paths) = self.partition_toggle_targets(&selected);
        if !stage_paths.is_empty() {
            self.repo.stage_paths(&stage_paths)?;
        }
        if !unstage_paths.is_empty() {
            self.repo.unstage_paths(&unstage_paths)?;
        }
        self.refresh_status()?;
        Ok((stage_paths.len(), unstage_paths.len()))
    }

    pub fn prepare_commit_from_visual_selection(&mut self) -> Result<usize> {
        let selected = self.visual_selected_indices();
        let targets = self.collect_commit_targets(&selected);
        if targets.is_empty() {
            return Ok(0);
        }

        self.repo.stage_paths(&targets)?;
        self.refresh_status()?;
        self.files_visual_mode = false;
        self.files_visual_anchor = None;
        Ok(targets.len())
    }

    pub fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf> {
        if self.active_panel != SidePanel::Files {
            return Vec::new();
        }

        if self.files_visual_mode {
            let selected = self.visual_selected_indices();
            return self.collect_commit_targets(&selected);
        }

        let Some(node) = self.selected_tree_node() else {
            return Vec::new();
        };
        vec![node.path.clone()]
    }

    pub fn start_stash_editor(&mut self, targets: Vec<PathBuf>) {
        self.input_mode = Some(InputMode::StashEditor);
        self.stash_targets = targets;
        self.stash_message_buffer.clear();
    }

    pub fn selected_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Files { return None; }
        let idx = self.files_panel.list_state.selected()?;
        self.file_tree_nodes.get(idx)
    }

    pub fn selected_branch_name(&self) -> Option<String> {
        if self.active_panel != SidePanel::LocalBranches {
            return None;
        }
        let idx = self.branches_panel.list_state.selected()?;
        self.branches.get(idx).map(|b| b.name.clone())
    }

    pub fn selected_commit_oid(&self) -> Option<String> {
        if self.active_panel != SidePanel::Commits {
            return None;
        }
        if self.commit_tree_mode {
            return self.commit_tree_commit_oid.clone();
        }
        let idx = self.commits_panel.list_state.selected()?;
        self.commits.get(idx).map(|c| c.oid.clone())
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        if self.active_panel != SidePanel::Stash {
            return None;
        }
        if self.stash_tree_mode {
            return self.stash_tree_stash_index;
        }
        let idx = self.stash_panel.list_state.selected()?;
        self.stashes.get(idx).map(|s| s.index)
    }

    pub fn stash_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.active_panel != SidePanel::Stash {
            return Ok(());
        }

        if !self.stash_tree_mode {
            let Some(index) = self.selected_stash_index() else {
                return Ok(());
            };
            self.stash_tree_files = self.repo.stash_files(index)?;
            self.stash_tree_expanded_dirs = collect_dirs_from_entries(&self.stash_tree_files);
            self.stash_tree_stash_index = Some(index);
            self.stash_tree_mode = true;
            self.rebuild_stash_tree();
            if self.stash_tree_nodes.is_empty() {
                self.stash_panel.list_state.select(None);
            } else {
                self.stash_panel.list_state.select(Some(0));
            }
            return Ok(());
        }

        let Some(node) = self.selected_stash_tree_node() else {
            return Ok(());
        };
        if !node.is_dir {
            return Ok(());
        }
        let path = node.path.clone();
        if self.stash_tree_expanded_dirs.contains(&path) {
            self.stash_tree_expanded_dirs.remove(&path);
        } else {
            self.stash_tree_expanded_dirs.insert(path);
        }
        self.rebuild_stash_tree();
        Ok(())
    }

    pub fn stash_close_tree(&mut self) {
        if !self.stash_tree_mode {
            return;
        }

        self.stash_tree_mode = false;
        self.stash_tree_files.clear();
        self.stash_tree_nodes.clear();
        self.stash_tree_expanded_dirs.clear();

        if let Some(stash_index) = self.stash_tree_stash_index.take() {
            if let Some(idx) = self.stashes.iter().position(|s| s.index == stash_index) {
                self.stash_panel.list_state.select(Some(idx));
                return;
            }
        }
        if self.stashes.is_empty() {
            self.stash_panel.list_state.select(None);
        } else {
            self.stash_panel.list_state.select(Some(0));
        }
    }

    pub fn load_diff(&mut self) {
        self.diff_scroll = 0;
        match self.active_panel {
            SidePanel::Files => {
                let Some(node) = self.selected_tree_node() else {
                    self.current_diff.clear();
                    return;
                };

                if node.is_dir {
                    self.load_dir_diff(node.path.clone());
                } else {
                    self.load_file_diff(node.path.clone(), node.status.clone());
                }
            }
            SidePanel::Commits => self.load_selected_commit_diff(),
            SidePanel::Stash => self.load_selected_stash_diff(),
            _ => self.current_diff.clear(),
        }
    }

    pub fn commit_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.active_panel != SidePanel::Commits {
            return Ok(());
        }

        if !self.commit_tree_mode {
            let Some(oid) = self.selected_commit_oid() else {
                return Ok(());
            };
            self.commit_tree_files = self.repo.commit_files(&oid)?;
            self.commit_tree_expanded_dirs = collect_dirs_from_entries(&self.commit_tree_files);
            self.commit_tree_commit_oid = Some(oid);
            self.commit_tree_mode = true;
            self.rebuild_commit_tree();
            if self.commit_tree_nodes.is_empty() {
                self.commits_panel.list_state.select(None);
            } else {
                self.commits_panel.list_state.select(Some(0));
            }
            return Ok(());
        }

        let Some(node) = self.selected_commit_tree_node() else {
            return Ok(());
        };
        if !node.is_dir {
            return Ok(());
        }
        let path = node.path.clone();
        if self.commit_tree_expanded_dirs.contains(&path) {
            self.commit_tree_expanded_dirs.remove(&path);
        } else {
            self.commit_tree_expanded_dirs.insert(path);
        }
        self.rebuild_commit_tree();
        Ok(())
    }

    pub fn commit_close_tree(&mut self) {
        if !self.commit_tree_mode {
            return;
        }
        self.commit_tree_mode = false;
        self.commit_tree_files.clear();
        self.commit_tree_nodes.clear();
        self.commit_tree_expanded_dirs.clear();

        if let Some(ref oid) = self.commit_tree_commit_oid {
            if let Some(idx) = self.commits.iter().position(|c| c.oid == *oid) {
                self.commits_panel.list_state.select(Some(idx));
                self.commit_tree_commit_oid = None;
                return;
            }
        }
        self.commit_tree_commit_oid = None;
        if self.commits.is_empty() {
            self.commits_panel.list_state.select(None);
        } else {
            self.commits_panel.list_state.select(Some(0));
        }
    }

    fn load_file_diff(&mut self, path: PathBuf, status: FileTreeNodeStatus) {
        self.current_diff = match &status {
            FileTreeNodeStatus::Staged(_) => self.repo.diff_staged(&path).unwrap_or_default(),
            FileTreeNodeStatus::Untracked => self.repo.diff_untracked(&path).unwrap_or_default(),
            FileTreeNodeStatus::Unstaged(_) => self.repo.diff_unstaged(&path).unwrap_or_default(),
            FileTreeNodeStatus::Directory => vec![],
        };
    }

    fn load_dir_diff(&mut self, dir_path: PathBuf) {
        const MAX_LINES: usize = 2000;
        let mut result = Vec::new();

        let file_nodes: Vec<(PathBuf, FileTreeNodeStatus)> = self
            .file_tree_nodes
            .iter()
            .filter(|n| !n.is_dir && n.path.starts_with(&dir_path))
            .map(|n| (n.path.clone(), n.status.clone()))
            .collect();

        for (path, status) in file_nodes {
            if result.len() >= MAX_LINES { break; }
            let lines = match &status {
                FileTreeNodeStatus::Staged(_) => self.repo.diff_staged(&path).unwrap_or_default(),
                FileTreeNodeStatus::Untracked => self.repo.diff_untracked(&path).unwrap_or_default(),
                FileTreeNodeStatus::Unstaged(_) => self.repo.diff_unstaged(&path).unwrap_or_default(),
                FileTreeNodeStatus::Directory => continue,
            };
            let remaining = MAX_LINES - result.len();
            result.extend(lines.into_iter().take(remaining));
        }

        self.current_diff = result;
    }

    fn load_selected_commit_diff(&mut self) {
        let Some(oid) = self.selected_commit_oid() else {
            self.current_diff.clear();
            return;
        };
        let path = if self.commit_tree_mode {
            self.selected_commit_tree_node().map(|n| n.path.as_path())
        } else {
            None
        };
        self.current_diff = self
            .repo
            .commit_diff_scoped(&oid, path)
            .unwrap_or_default();
    }

    fn load_selected_stash_diff(&mut self) {
        let Some(stash_index) = self.selected_stash_index() else {
            self.current_diff.clear();
            return;
        };

        let path = if self.stash_tree_mode {
            self.selected_stash_tree_node().map(|n| n.path.as_path())
        } else {
            None
        };

        self.current_diff = self.repo.stash_diff(stash_index, path).unwrap_or_default();
    }

    fn toggle_stage_for_selected_file(&self) -> Option<super::Message> {
        use super::Message;

        let node = self.selected_tree_node()?;
        if node.is_dir {
            return Some(Message::ToggleDir);
        }

        match &node.status {
            FileTreeNodeStatus::Staged(_) => Some(Message::UnstageFile(node.path.clone())),
            FileTreeNodeStatus::Unstaged(_) | FileTreeNodeStatus::Untracked => {
                Some(Message::StageFile(node.path.clone()))
            }
            FileTreeNodeStatus::Directory => None,
        }
    }

    fn partition_toggle_targets(&self, selected: &HashSet<usize>) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut stage_paths = Vec::new();
        let mut unstage_paths = Vec::new();

        for target in self.collect_selection_targets(selected) {
            if target.all_staged {
                unstage_paths.push(target.path);
            } else {
                stage_paths.push(target.path);
            }
        }

        (stage_paths, unstage_paths)
    }

    fn collect_commit_targets(&self, selected: &HashSet<usize>) -> Vec<PathBuf> {
        self.collect_selection_targets(selected)
            .into_iter()
            .map(|t| t.path)
            .collect()
    }

    fn collect_selection_targets(&self, selected: &HashSet<usize>) -> Vec<SelectionTarget> {
        let mut targets = Vec::new();
        let mut covered = HashSet::new();
        let mut ordered: Vec<usize> = selected.iter().copied().collect();
        ordered.sort_unstable();

        for idx in ordered {
            if covered.contains(&idx) {
                continue;
            }
            let Some(node) = self.file_tree_nodes.get(idx) else {
                continue;
            };

            if node.is_dir {
                let end = self.subtree_end_index(idx);
                let fully_covered = (idx..=end).all(|i| selected.contains(&i));
                if fully_covered {
                    let all_staged = self.selected_files_are_all_staged(selected, &node.path);
                    targets.push(SelectionTarget {
                        path: node.path.clone(),
                        all_staged,
                    });
                    for i in idx..=end {
                        covered.insert(i);
                    }
                }
                continue;
            }

            let all_staged = matches!(node.status, FileTreeNodeStatus::Staged(_));
            targets.push(SelectionTarget {
                path: node.path.clone(),
                all_staged,
            });
            covered.insert(idx);
        }

        dedup_targets(targets)
    }

    fn selected_files_are_all_staged(&self, selected: &HashSet<usize>, dir_path: &std::path::Path) -> bool {
        let mut has_file = false;
        for idx in selected {
            let Some(node) = self.file_tree_nodes.get(*idx) else {
                continue;
            };
            if node.is_dir || !node.path.starts_with(dir_path) {
                continue;
            }
            has_file = true;
            if !matches!(node.status, FileTreeNodeStatus::Staged(_)) {
                return false;
            }
        }
        has_file
    }

    fn subtree_end_index(&self, index: usize) -> usize {
        let Some(node) = self.file_tree_nodes.get(index) else {
            return index;
        };
        if !node.is_dir {
            return index;
        }

        let base_depth = node.depth;
        let mut end = index;
        for i in index + 1..self.file_tree_nodes.len() {
            let n = &self.file_tree_nodes[i];
            if n.depth <= base_depth {
                break;
            }
            end = i;
        }
        end
    }

    fn active_panel_name(&self) -> &'static str {
        match self.active_panel {
            SidePanel::Files => "files",
            SidePanel::LocalBranches => "branches",
            SidePanel::Commits => "commits",
            SidePanel::Stash => "stash",
        }
    }

    fn global_key_or(&self, action: &str, fallback: &str) -> String {
        self.keymap
            .first_global_key(action)
            .unwrap_or_else(|| fallback.to_string())
    }

    fn panel_key_or(&self, panel: &str, action: &str, fallback: &str) -> String {
        self.keymap
            .first_panel_key(panel, action)
            .unwrap_or_else(|| fallback.to_string())
    }

    fn selected_commit_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Commits || !self.commit_tree_mode {
            return None;
        }
        let idx = self.commits_panel.list_state.selected()?;
        self.commit_tree_nodes.get(idx)
    }

    fn rebuild_commit_tree(&mut self) {
        let selected = self.commits_panel.list_state.selected();
        self.commit_tree_nodes = FileTree::from_git_status_with_expanded(
            &self.commit_tree_files,
            &[],
            &[],
            &self.commit_tree_expanded_dirs,
        );
        let count = self.commit_tree_nodes.len();
        if count == 0 {
            self.commits_panel.list_state.select(None);
            return;
        }
        let idx = selected.unwrap_or(0).min(count - 1);
        self.commits_panel.list_state.select(Some(idx));
    }

    fn selected_stash_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Stash || !self.stash_tree_mode {
            return None;
        }
        let idx = self.stash_panel.list_state.selected()?;
        self.stash_tree_nodes.get(idx)
    }

    fn rebuild_stash_tree(&mut self) {
        let selected = self.stash_panel.list_state.selected();
        self.stash_tree_nodes = FileTree::from_git_status_with_expanded(
            &self.stash_tree_files,
            &[],
            &[],
            &self.stash_tree_expanded_dirs,
        );
        let count = self.stash_tree_nodes.len();
        if count == 0 {
            self.stash_panel.list_state.select(None);
            return;
        }
        let idx = selected.unwrap_or(0).min(count - 1);
        self.stash_panel.list_state.select(Some(idx));
    }
}

#[derive(Debug)]
struct SelectionTarget {
    path: PathBuf,
    all_staged: bool,
}

fn dedup_targets(mut targets: Vec<SelectionTarget>) -> Vec<SelectionTarget> {
    let mut seen = HashSet::<PathBuf>::new();
    targets.retain(|t| seen.insert(t.path.clone()));
    targets
}

/// Documentation comment in English.
fn collect_all_dirs(status: &GitStatus) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    let all_files = status.unstaged.iter().map(|f| &f.path)
        .chain(status.untracked.iter().map(|f| &f.path))
        .chain(status.staged.iter().map(|f| &f.path));

    for path in all_files {
        let mut p = path.as_path();
        while let Some(parent) = p.parent() {
            if parent == std::path::Path::new("") { break; }
            dirs.insert(parent.to_path_buf());
            p = parent;
        }
    }
    dirs
}

fn collect_dirs_from_entries(entries: &[FileEntry]) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    for entry in entries {
        let mut p = entry.path.as_path();
        while let Some(parent) = p.parent() {
            if parent == std::path::Path::new("") {
                break;
            }
            dirs.insert(parent.to_path_buf());
            p = parent;
        }
    }
    dirs
}
