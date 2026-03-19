use crate::git::{Git2Repository, GitRepository, GitStatus};
use crate::ui::layout::render_layout;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::widgets::ListState;
use ratatui::Frame;

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
    pub item_count: usize,
}

impl PanelState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state, item_count: 0 }  // item_count reserved for future use
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

    pub files_panel: PanelState,
    pub branches_panel: PanelState,
    pub commits_panel: PanelState,
    pub stash_panel: PanelState,

    pub command_log: Vec<CommandLogEntry>,
    pub branches: Vec<String>,
    pub commits: Vec<String>,
    pub stashes: Vec<String>,
}

impl App {
    pub fn new() -> Result<Self> {
        let repo = Git2Repository::discover()?;
        let status = repo.status()?;

        Ok(Self {
            running: true,
            current_tab: Tab::Status,
            active_panel: SidePanel::Files,
            repo: Box::new(repo),
            status,
            files_panel: PanelState::new(),
            branches_panel: PanelState::new(),
            commits_panel: PanelState::new(),
            stash_panel: PanelState::new(),
            command_log: Vec::new(),
            branches: Vec::new(),
            commits: Vec::new(),
            stashes: Vec::new(),
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<super::Message> {
        use crossterm::event::KeyCode;
        use super::Message;

        match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('r') => Some(Message::RefreshStatus),
            KeyCode::Tab => Some(Message::PanelNext),
            KeyCode::BackTab => Some(Message::PanelPrev),
            KeyCode::Char('j') | KeyCode::Down => Some(Message::ListDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::ListUp),
            _ => None,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        render_layout(frame, self);
    }

    pub fn refresh_status(&mut self) -> Result<()> {
        self.status = self.repo.status()?;
        Ok(())
    }

    pub fn stage_file(&mut self, path: std::path::PathBuf) -> Result<()> {
        self.repo.stage(&path)?;
        self.refresh_status()?;
        Ok(())
    }

    pub fn unstage_file(&mut self, path: std::path::PathBuf) -> Result<()> {
        self.repo.unstage(&path)?;
        self.refresh_status()?;
        Ok(())
    }

    /// 获取当前活跃面板的列表项数量
    fn active_panel_count(&self) -> usize {
        match self.active_panel {
            SidePanel::Files => {
                self.status.unstaged.len() + self.status.untracked.len() + self.status.staged.len()
            }
            SidePanel::LocalBranches => self.branches.len(),
            SidePanel::Commits => self.commits.len(),
            SidePanel::Stash => self.stashes.len(),
        }
    }

    /// 获取当前活跃面板的 list_state（可变）
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
}
