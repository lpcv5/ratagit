use crate::config::keymap::{key_to_string, Keymap};
use crate::git::{Git2Repository, GitRepository, GitStatus, DiffLine, BranchInfo, CommitInfo, StashInfo};
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
    CommitMessage,
    CreateBranch,
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
    pub branches_panel: PanelState,
    pub commits_panel: PanelState,
    pub stash_panel: PanelState,

    pub command_log: Vec<CommandLogEntry>,
    pub branches: Vec<BranchInfo>,
    pub commits: Vec<CommitInfo>,
    pub stashes: Vec<StashInfo>,
    pub current_diff: Vec<DiffLine>,
    /// Documentation comment in English.
    pub diff_scroll: usize,
    pub input_mode: Option<InputMode>,
    pub input_buffer: String,

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
            branches_panel: PanelState::new(),
            commits_panel: PanelState::new(),
            stash_panel: PanelState::new(),
            command_log: Vec::new(),
            branches,
            commits,
            stashes,
            current_diff: Vec::new(),
            diff_scroll: 0,
            input_mode: None,
            input_buffer: String::new(),
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
        if gm("commit")            { return Some(Message::StartCommitInput); }

        // Comment in English.
        let panel = match self.active_panel {
            SidePanel::Files         => "files",
            SidePanel::LocalBranches => "branches",
            SidePanel::Commits       => "commits",
            SidePanel::Stash         => "stash",
        };
        let pm = |action| self.keymap.panel_matches(panel, action, &k);

        if pm("toggle_stage") {
            if let Some(msg) = self.toggle_stage_for_selected_file() {
                return Some(msg);
            }
        }
        if pm("toggle_dir")   { return Some(Message::ToggleDir); }
        if pm("collapse_all") { return Some(Message::CollapseAll); }
        if pm("expand_all")   { return Some(Message::ExpandAll); }
        if pm("checkout_branch") { return Some(Message::CheckoutSelectedBranch); }
        if pm("create_branch") { return Some(Message::StartBranchCreateInput); }
        if pm("delete_branch") { return Some(Message::DeleteSelectedBranch); }

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
            KeyCode::Enter => {
                let value = self.input_buffer.trim().to_string();
                self.input_mode = None;
                self.input_buffer.clear();

                if value.is_empty() {
                    self.push_log("Empty input ignored", false);
                    return None;
                }

                match mode {
                    InputMode::CommitMessage => Some(Message::Commit(value)),
                    InputMode::CreateBranch => Some(Message::CreateBranch(value)),
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                None
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.input_buffer.push(c);
                }
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        render_layout(frame, self);
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
        self.stashes = self.repo.stashes().unwrap_or_default();
        Ok(())
    }

    pub fn start_commit_input(&mut self) {
        self.input_mode = Some(InputMode::CommitMessage);
        self.input_buffer.clear();
    }

    pub fn start_branch_create_input(&mut self) {
        self.input_mode = Some(InputMode::CreateBranch);
        self.input_buffer.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = None;
        self.input_buffer.clear();
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
        } else {
            let idx = selected.unwrap_or(0).min(count - 1);
            self.files_panel.list_state.select(Some(idx));
        }
    }

    fn active_panel_count(&self) -> usize {
        match self.active_panel {
            SidePanel::Files => self.file_tree_nodes.len(),
            SidePanel::LocalBranches => self.branches.len(),
            SidePanel::Commits => self.commits.len(),
            SidePanel::Stash => self.stashes.len(),
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
            _ => self.current_diff.clear(),
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
        let Some(idx) = self.commits_panel.list_state.selected() else {
            self.current_diff.clear();
            return;
        };
        let Some(commit) = self.commits.get(idx) else {
            self.current_diff.clear();
            return;
        };

        let mut lines = vec![
            DiffLine {
                kind: crate::git::DiffLineKind::Header,
                content: format!("commit {}", commit.oid),
            },
            DiffLine {
                kind: crate::git::DiffLineKind::Header,
                content: format!("Author: {}", commit.author),
            },
            DiffLine {
                kind: crate::git::DiffLineKind::Header,
                content: format!("Date:   {}", commit.time),
            },
            DiffLine {
                kind: crate::git::DiffLineKind::Header,
                content: String::new(),
            },
            DiffLine {
                kind: crate::git::DiffLineKind::Header,
                content: format!("    {}", commit.message),
            },
            DiffLine {
                kind: crate::git::DiffLineKind::Header,
                content: String::new(),
            },
        ];

        match self.repo.commit_diff(&commit.oid) {
            Ok(mut patch) => {
                lines.append(&mut patch);
                self.current_diff = lines;
            }
            Err(_) => self.current_diff = lines,
        }
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
