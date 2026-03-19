use crate::config::keymap::{key_to_string, Keymap};
use crate::git::{Git2Repository, GitRepository, GitStatus, DiffLine};
use crate::ui::layout::render_layout;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeNodeStatus};
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::widgets::ListState;
use ratatui::Frame;
use std::collections::HashSet;
use std::path::PathBuf;

/// Tab 类型
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Tab {
    #[default]
    Status,
    Commits,
    Branches,
    Stash,
}

/// 左侧活跃面板
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SidePanel {
    #[default]
    Files,
    LocalBranches,
    Commits,
    Stash,
}

/// 面板列表状态
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

/// 命令日志条目
pub struct CommandLogEntry {
    pub command: String,
    pub success: bool,
}

/// 应用状态（TEA 架构中的 Model）
pub struct App {
    pub running: bool,
    pub current_tab: Tab,
    pub active_panel: SidePanel,

    repo: Box<dyn GitRepository>,
    pub status: GitStatus,

    /// 文件树节点（扁平化可见列表）
    pub file_tree_nodes: Vec<FileTreeNode>,
    /// 当前展开的目录集合
    expanded_dirs: HashSet<PathBuf>,

    pub files_panel: PanelState,
    pub branches_panel: PanelState,
    pub commits_panel: PanelState,
    pub stash_panel: PanelState,

    pub command_log: Vec<CommandLogEntry>,
    pub branches: Vec<String>,
    pub commits: Vec<String>,
    pub stashes: Vec<String>,
    pub current_diff: Vec<DiffLine>,
    /// diff 面板滚动偏移
    pub diff_scroll: usize,

    keymap: Keymap,
}

impl App {
    pub fn new() -> Result<Self> {
        let repo = Git2Repository::discover()?;
        let status = repo.status()?;

        // 默认全部展开
        let expanded_dirs = collect_all_dirs(&status);
        let file_tree_nodes = FileTree::from_git_status_with_expanded(
            &status.unstaged,
            &status.untracked,
            &status.staged,
            &expanded_dirs,
        );

        let keymap = Keymap::load();

        let mut app = Self {
            running: true,
            current_tab: Tab::Status,
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
            branches: Vec::new(),
            commits: Vec::new(),
            stashes: Vec::new(),
            current_diff: Vec::new(),
            diff_scroll: 0,
            keymap,
        };
        app.load_diff();
        Ok(app)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<super::Message> {
        use super::Message;
        let k = key_to_string(&key);
        if k.is_empty() { return None; }

        let gm = |action| self.keymap.global_matches(action, &k);

        // 全局快捷键
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

        // 面板本地快捷键
        let panel = match self.active_panel {
            SidePanel::Files         => "files",
            SidePanel::LocalBranches => "branches",
            SidePanel::Commits       => "commits",
            SidePanel::Stash         => "stash",
        };
        let pm = |action| self.keymap.panel_matches(panel, action, &k);

        if pm("toggle_dir")   { return Some(Message::ToggleDir); }
        if pm("collapse_all") { return Some(Message::CollapseAll); }
        if pm("expand_all")   { return Some(Message::ExpandAll); }

        None
    }

    pub fn render(&self, frame: &mut Frame) {
        render_layout(frame, self);
    }

    pub fn refresh_status(&mut self) -> Result<()> {
        self.status = self.repo.status()?;
        // 保留当前展开状态，只新增目录
        let new_dirs = collect_all_dirs(&self.status);
        for d in new_dirs {
            self.expanded_dirs.insert(d);
        }
        self.rebuild_tree();
        Ok(())
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

    /// 折叠/展开当前选中目录
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

    /// 折叠所有目录
    pub fn collapse_all(&mut self) {
        self.expanded_dirs.clear();
        self.rebuild_tree();
    }

    /// 展开所有目录
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
        // 修正选中索引不越界
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

    pub fn load_diff(&mut self) {
        self.diff_scroll = 0;
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
}

/// 从 GitStatus 收集所有目录路径
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
