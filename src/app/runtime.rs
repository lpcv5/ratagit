use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::ListState,
    DefaultTerminal, Frame,
};
use tokio::sync::mpsc::error::TryRecvError;

use crate::backend::{BackendCommand, EventEnvelope, FrontendEvent};
use crate::components::Component;
use crate::components::Intent;

use super::state::AppState;
use super::Panel;

pub struct App {
    state: AppState,
    /// 等待响应的请求 ID 集合
    pending_requests: HashSet<u64>,
}

impl App {
    pub fn new(
        cmd_tx: tokio::sync::mpsc::UnboundedSender<crate::backend::CommandEnvelope>,
        event_rx: tokio::sync::mpsc::UnboundedReceiver<crate::backend::EventEnvelope>,
    ) -> Self {
        Self {
            state: AppState::new(cmd_tx, event_rx),
            pending_requests: HashSet::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        self.request_refresh_all();
        self.update_main_view_for_active_panel()?;

        let result = self.main_loop(&mut terminal).await;
        ratatui::restore();
        result
    }

    async fn main_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.state.should_quit {
            self.drain_backend_events().await?;
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(100))? {
                let input = event::read()?;
                self.handle_input(input)?;
            }
        }

        Ok(())
    }

    async fn drain_backend_events(&mut self) -> Result<()> {
        loop {
            match self.state.event_rx.try_recv() {
                Ok(envelope) => self.handle_backend_event(envelope)?,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.state.should_quit = true;
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_input(&mut self, event: Event) -> Result<()> {
        // 全局按键处理（q, 面板切换, 刷新, 退出）
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') => {
                    self.state.should_quit = true;
                    return Ok(());
                }
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') if key.modifiers.is_empty() => {
                    let intent = Intent::SwitchFocus(self.state.ui_state.active_panel.next());
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h')
                    if key.modifiers.is_empty() =>
                {
                    let intent = Intent::SwitchFocus(self.state.ui_state.active_panel.previous());
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::Char('r') if key.modifiers.is_empty() => {
                    self.request_refresh_all();
                    return Ok(());
                }
                _ => {}
            }
        }

        // 委派给当前活动面板组件处理
        let intent = self.state.components.dispatch_event(
            self.state.ui_state.active_panel,
            &event,
            &self.state.data_cache,
        );
        self.execute_intent(intent)
    }

    fn execute_intent(&mut self, intent: Intent) -> Result<()> {
        match intent {
            Intent::SelectNext => self.navigate_forward()?,
            Intent::SelectPrevious => self.navigate_backward()?,
            Intent::SwitchFocus(panel) => self.set_active_panel(panel)?,
            Intent::ScrollMainView(delta) => {
                self.state.components.scroll_main_view_by(delta);
            }
            Intent::ScrollLog(delta) => {
                self.state.components.scroll_log_by(delta);
            }
            Intent::ActivatePanel => self.activate_panel()?,
            Intent::ToggleStageFile => self.toggle_stage_selected_file()?,
            Intent::SendCommand(cmd) => {
                let request_id = self.state.send_command(cmd)?;
                self.pending_requests.insert(request_id);
            }
            Intent::None => {}
        }

        Ok(())
    }

    fn handle_backend_event(&mut self, envelope: EventEnvelope) -> Result<()> {
        let request_id = envelope.request_id;

        // 乱序防护：只接受已发送的请求 ID
        if let Some(id) = request_id {
            if !self.pending_requests.remove(&id) {
                // 过期/未知响应，丢弃
                return Ok(());
            }
        }

        match envelope.event {
            FrontendEvent::FilesUpdated { files } => {
                self.state.data_cache.files = files;
                self.state.sync_file_list_state();
                self.state.push_log(format!(
                    "Files refreshed: {} entries",
                    self.state.data_cache.files.len()
                ));

                if matches!(
                    self.state.ui_state.active_panel,
                    Panel::Files | Panel::MainView
                ) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::BranchesUpdated { branches } => {
                self.state.data_cache.branches = branches;
                self.state.sync_branch_list_state();
                self.state.push_log(format!(
                    "Branches refreshed: {} entries",
                    self.state.data_cache.branches.len()
                ));

                if matches!(
                    self.state.ui_state.active_panel,
                    Panel::Branches | Panel::MainView
                ) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::CommitsUpdated { commits } => {
                self.state.data_cache.commits = commits;
                self.state.sync_commit_list_state();
                self.state.push_log(format!(
                    "Commits refreshed: {} entries",
                    self.state.data_cache.commits.len()
                ));

                if matches!(
                    self.state.ui_state.active_panel,
                    Panel::Commits | Panel::MainView
                ) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::StashesUpdated { stashes } => {
                self.state.data_cache.stashes = stashes;
                self.state.sync_stash_list_state();
                self.state.push_log(format!(
                    "Stashes refreshed: {} entries",
                    self.state.data_cache.stashes.len()
                ));

                if matches!(
                    self.state.ui_state.active_panel,
                    Panel::Stash | Panel::MainView
                ) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::DiffLoaded {
                file_path, diff, ..
            } => {
                self.state.data_cache.current_diff = Some((file_path.clone(), diff.clone()));
                self.state.push_log(format!("Loaded diff for {file_path}"));
            }
            FrontendEvent::Error { message, .. } => {
                self.state.push_log(format!("Error: {message}"));
            }
            FrontendEvent::ActionSucceeded { message, .. } => {
                self.state.push_log(format!("OK: {message}"));
            }
            FrontendEvent::ActionFailed { message, .. } => {
                self.state.push_log(format!("FAILED: {message}"));
            }
        }

        Ok(())
    }

    fn request_refresh_all(&self) {
        // 刷新类事件不需要请求 ID（它们不是由用户操作触发的）
        let _ = self.state.cmd_tx.send(crate::backend::CommandEnvelope::new(
            0,
            BackendCommand::RefreshStatus,
        ));
        let _ = self.state.cmd_tx.send(crate::backend::CommandEnvelope::new(
            0,
            BackendCommand::RefreshBranches,
        ));
        let _ = self.state.cmd_tx.send(crate::backend::CommandEnvelope::new(
            0,
            BackendCommand::RefreshCommits { limit: 30 },
        ));
        let _ = self.state.cmd_tx.send(crate::backend::CommandEnvelope::new(
            0,
            BackendCommand::RefreshStashes,
        ));
    }

    fn set_active_panel(&mut self, panel: Panel) -> Result<()> {
        self.state.ui_state.active_panel = panel;
        self.state
            .push_log(format!("Focus moved to {}", panel.title()));
        self.update_main_view_for_active_panel()
    }

    fn activate_panel(&mut self) -> Result<()> {
        self.state.push_log(format!(
            "Activated {}",
            self.state.ui_state.active_panel.title()
        ));
        self.update_main_view_for_active_panel()
    }

    fn navigate_forward(&mut self) -> Result<()> {
        match self.state.ui_state.active_panel {
            Panel::Files => {
                cycle_selection(
                    self.state.components.file_list_state_mut(),
                    self.state.data_cache.files.len(),
                    1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Branches => {
                cycle_selection(
                    self.state.components.branch_list_state_mut(),
                    self.state.data_cache.branches.len(),
                    1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Commits => {
                cycle_selection(
                    self.state.components.commit_list_state_mut(),
                    self.state.data_cache.commits.len(),
                    1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Stash => {
                cycle_selection(
                    self.state.components.stash_list_state_mut(),
                    self.state.data_cache.stashes.len(),
                    1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::MainView => {
                self.state.components.scroll_main_view_by(1);
            }
            Panel::Log => {
                self.state.components.scroll_log_by(1);
            }
        }

        Ok(())
    }

    fn navigate_backward(&mut self) -> Result<()> {
        match self.state.ui_state.active_panel {
            Panel::Files => {
                cycle_selection(
                    self.state.components.file_list_state_mut(),
                    self.state.data_cache.files.len(),
                    -1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Branches => {
                cycle_selection(
                    self.state.components.branch_list_state_mut(),
                    self.state.data_cache.branches.len(),
                    -1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Commits => {
                cycle_selection(
                    self.state.components.commit_list_state_mut(),
                    self.state.data_cache.commits.len(),
                    -1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Stash => {
                cycle_selection(
                    self.state.components.stash_list_state_mut(),
                    self.state.data_cache.stashes.len(),
                    -1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::MainView => {
                self.state.components.scroll_main_view_by(-1);
            }
            Panel::Log => {
                self.state.components.scroll_log_by(-1);
            }
        }

        Ok(())
    }

    fn update_main_view_for_active_panel(&mut self) -> Result<()> {
        match self.state.ui_state.active_panel {
            Panel::MainView => self.show_repo_overview(),
            Panel::Files => self.request_selected_file_diff()?,
            Panel::Branches => self.show_branch_detail(),
            Panel::Commits => self.show_commit_detail(),
            Panel::Stash => self.show_stash_detail(),
            Panel::Log => self.show_log_detail(),
        }

        Ok(())
    }

    fn show_repo_overview(&mut self) {
        let current_branch = self
            .state
            .data_cache
            .branches
            .iter()
            .find(|branch| branch.is_head)
            .map(|branch| branch.name.clone())
            .unwrap_or_else(|| "(detached)".to_string());

        let staged = self
            .state
            .data_cache
            .files
            .iter()
            .filter(|file| file.is_staged)
            .count();
        let unstaged = self
            .state
            .data_cache
            .files
            .iter()
            .filter(|file| file.is_unstaged)
            .count();

        let content = format!(
            "Repository snapshot\n\nCurrent branch: {current_branch}\nFiles: {} (staged: {staged}, unstaged: {unstaged})\nBranches: {}\nCommits loaded: {}\nStashes: {}\n\nNavigation\n- Tab / Shift+Tab or h/l: switch panel focus\n- j/k or arrows: move inside the focused panel\n- Enter: refresh the current panel detail\n- r: refresh all Git-backed panels\n- Ctrl+d / Ctrl+u: fast scroll in Main View and Log\n- q: quit",
            self.state.data_cache.files.len(),
            self.state.data_cache.branches.len(),
            self.state.data_cache.commits.len(),
            self.state.data_cache.stashes.len()
        );

        self.state.data_cache.current_diff = Some(("Main View · Overview".to_string(), content));
        self.state.components.main_view_scroll_to(0);
    }

    fn request_selected_file_diff(&mut self) -> Result<()> {
        if let Some(path) = self.state.selected_file().map(|file| file.path.clone()) {
            self.state.data_cache.current_diff =
                Some((path.clone(), format!("Loading diff for {path}...")));
            self.state.components.main_view_scroll_to(0);

            let request_id = self
                .state
                .send_command(BackendCommand::GetDiff { file_path: path })?;
            self.pending_requests.insert(request_id);
        } else {
            self.state.data_cache.current_diff = Some((
                "Main View · Files".to_string(),
                "No file is currently selected.".to_string(),
            ));
            self.state.components.main_view_scroll_to(0);
        }

        Ok(())
    }

    fn toggle_stage_selected_file(&mut self) -> Result<()> {
        if let Some(file) = self.state.selected_file() {
            let path = file.path.clone();
            let is_staged = file.is_staged;

            let command = if is_staged {
                BackendCommand::UnstageFile { file_path: path }
            } else {
                BackendCommand::StageFile { file_path: path }
            };

            let request_id = self.state.send_command(command)?;
            self.pending_requests.insert(request_id);
        }

        Ok(())
    }

    fn show_branch_detail(&mut self) {
        if let Some((name, is_head, upstream)) = self
            .state
            .selected_branch()
            .map(|branch| (branch.name.clone(), branch.is_head, branch.upstream.clone()))
        {
            let upstream = upstream.unwrap_or_else(|| "(no upstream)".to_string());
            let head_status = if is_head { "yes" } else { "no" };

            let content = format!(
                "Branch detail\n\nName: {}\nChecked out: {head_status}\nUpstream: {upstream}\n\nThis panel is wired for navigation first. Branch actions can be layered on top of the same focus model later.",
                name
            );

            self.state.data_cache.current_diff =
                Some((format!("Main View · Branch · {name}"), content));
        } else {
            self.state.data_cache.current_diff = Some((
                "Main View · Branches".to_string(),
                "No branches available.".to_string(),
            ));
        }
        self.state.components.main_view_scroll_to(0);
    }

    fn show_commit_detail(&mut self) {
        if let Some((short_id, id, author, timestamp, summary, body)) =
            self.state.selected_commit().map(|commit| {
                (
                    commit.short_id.clone(),
                    commit.id.clone(),
                    commit.author.clone(),
                    commit.timestamp,
                    commit.summary.clone(),
                    commit.body.clone(),
                )
            })
        {
            let body = body
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or("(no body)");

            let content = format!(
                "Commit detail\n\nCommit: {}\nAuthor: {}\nTimestamp: {}\nSummary: {}\n\nBody:\n{}",
                id, author, timestamp, summary, body
            );

            self.state.data_cache.current_diff =
                Some((format!("Main View · Commit · {short_id}"), content));
        } else {
            self.state.data_cache.current_diff = Some((
                "Main View · Commits".to_string(),
                "No commits loaded.".to_string(),
            ));
        }
        self.state.components.main_view_scroll_to(0);
    }

    fn show_stash_detail(&mut self) {
        if let Some((index, id, message)) = self
            .state
            .selected_stash()
            .map(|stash| (stash.index, stash.id.clone(), stash.message.clone()))
        {
            let content = format!(
                "Stash detail\n\nIndex: {}\nId: {}\nMessage: {}\n\nStash actions can be added later without changing the panel navigation shell.",
                index, id, message
            );

            self.state.data_cache.current_diff =
                Some((format!("Main View · Stash · #{index}"), content));
        } else {
            self.state.data_cache.current_diff = Some((
                "Main View · Stash".to_string(),
                "No stashes found in this repository.".to_string(),
            ));
        }
        self.state.components.main_view_scroll_to(0);
    }

    fn show_log_detail(&mut self) {
        let content = "The Log panel records UI focus changes, refreshes, and backend responses.\n\nUse j/k or Ctrl+d / Ctrl+u while the Log panel is focused to scroll through recent messages.".to_string();
        self.state.data_cache.current_diff = Some(("Main View · Log Help".to_string(), content));
        self.state.components.main_view_scroll_to(0);
    }

    fn render(&mut self, frame: &mut Frame) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
            .split(frame.area());

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(28),
                Constraint::Percentage(24),
                Constraint::Percentage(28),
                Constraint::Percentage(20),
            ])
            .split(columns[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(12), Constraint::Length(8)])
            .split(columns[1]);

        // 使用组件渲染
        self.state.components.file_list_panel.render(
            frame,
            left[0],
            self.state.ui_state.active_panel == Panel::Files,
            &self.state.data_cache,
        );
        self.state.components.branch_list_panel.render(
            frame,
            left[1],
            self.state.ui_state.active_panel == Panel::Branches,
            &self.state.data_cache,
        );
        self.state.components.commit_list_panel.render(
            frame,
            left[2],
            self.state.ui_state.active_panel == Panel::Commits,
            &self.state.data_cache,
        );
        self.state.components.stash_list_panel.render(
            frame,
            left[3],
            self.state.ui_state.active_panel == Panel::Stash,
            &self.state.data_cache,
        );
        self.state.components.main_view_panel.render(
            frame,
            right[0],
            self.state.ui_state.active_panel == Panel::MainView,
            &self.state.data_cache,
        );
        self.state.components.log_panel.render(
            frame,
            right[1],
            self.state.ui_state.active_panel == Panel::Log,
            &self.state.data_cache,
        );
    }
}

fn cycle_selection(state: &mut ListState, len: usize, delta: i8) {
    if len == 0 {
        state.select(None);
        return;
    }

    let current = state.selected().unwrap_or(0);
    let next = if delta > 0 {
        if current + 1 >= len {
            0
        } else {
            current + 1
        }
    } else if current == 0 {
        len.saturating_sub(1)
    } else {
        current - 1
    };

    state.select(Some(next));
}
