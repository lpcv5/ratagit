use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Clear, ListState},
    DefaultTerminal, Frame,
};
use tokio::sync::mpsc::error::TryRecvError;

use crate::backend::{BackendCommand, DiffTarget, EventEnvelope, FrontendEvent};
use crate::components::panels::CommitModeView;
use crate::components::Component;
use crate::components::Intent;

use super::state::AppState;
use super::Panel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum UiSlot {
    Files,
    Branches,
    Commits,
    Stash,
    MainView,
    Log,
}

pub struct App {
    state: AppState,
    /// 等待响应的请求 ID 集合
    pending_requests: HashSet<u64>,
    /// 当前“最新 diff”请求 ID（用于丢弃过期 diff 响应）
    latest_diff_request_id: Option<u64>,
    /// 当前“最新分支图”请求 ID（用于丢弃过期分支图响应）
    latest_branch_graph_request_id: Option<u64>,
}

impl App {
    pub fn new(
        cmd_tx: tokio::sync::mpsc::UnboundedSender<crate::backend::CommandEnvelope>,
        event_rx: tokio::sync::mpsc::UnboundedReceiver<crate::backend::EventEnvelope>,
    ) -> Self {
        Self {
            state: AppState::new(cmd_tx, event_rx),
            pending_requests: HashSet::new(),
            latest_diff_request_id: None,
            latest_branch_graph_request_id: None,
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
                KeyCode::Char('1') if key.modifiers.is_empty() => {
                    let intent = Intent::SwitchFocus(Panel::Files);
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::Char('2') if key.modifiers.is_empty() => {
                    let intent = Intent::SwitchFocus(Panel::Branches);
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::Char('3') if key.modifiers.is_empty() => {
                    let intent = Intent::SwitchFocus(Panel::Commits);
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::Char('4') if key.modifiers.is_empty() => {
                    let intent = Intent::SwitchFocus(Panel::Stash);
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::Char('l') if key.modifiers.is_empty() => {
                    let intent =
                        Intent::SwitchFocus(next_left_panel(self.state.ui_state.active_panel));
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::Char('h') if key.modifiers.is_empty() => {
                    let intent =
                        Intent::SwitchFocus(previous_left_panel(self.state.ui_state.active_panel));
                    self.execute_intent(intent)?;
                    return Ok(());
                }
                KeyCode::Char('r') if key.modifiers.is_empty() => {
                    self.request_refresh_all();
                    return Ok(());
                }
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    self.scroll_main_view_by(-MAIN_VIEW_PAGE_SCROLL);
                    return Ok(());
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    self.scroll_main_view_by(MAIN_VIEW_PAGE_SCROLL);
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
        self.execute_intent(intent)?;

        if self.should_refresh_commit_tree_diff(&event) {
            self.update_main_view_for_active_panel()?;
        }

        Ok(())
    }

    fn should_refresh_commit_tree_diff(&self, event: &Event) -> bool {
        if self.state.ui_state.active_panel != Panel::Commits {
            return false;
        }

        if !matches!(
            self.state.components.commit_mode_view(),
            CommitModeView::FilesTree { .. }
        ) {
            return false;
        }

        let Event::Key(key) = event else {
            return false;
        };

        if key.kind != KeyEventKind::Press {
            return false;
        }

        matches!(
            key.code,
            KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Down | KeyCode::Up | KeyCode::Enter
        )
    }

    fn execute_intent(&mut self, intent: Intent) -> Result<()> {
        match intent {
            Intent::SelectNext => self.navigate_forward()?,
            Intent::SelectPrevious => self.navigate_backward()?,
            Intent::SwitchFocus(panel) => self.set_active_panel(panel)?,
            Intent::RefreshPanelDetail => self.update_main_view_for_active_panel()?,
            Intent::ScrollMainView(delta) => {
                self.scroll_main_view_by(delta);
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
                self.state
                    .data_cache
                    .branch_graphs
                    .retain(|branch_name, _| {
                        self.state
                            .data_cache
                            .branches
                            .iter()
                            .any(|branch| branch.name == *branch_name)
                    });
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
                self.state.data_cache.branch_graphs.clear();
                self.state.push_log(format!(
                    "Commits refreshed: {} entries",
                    self.state.data_cache.commits.len()
                ));

                if matches!(
                    self.state.ui_state.active_panel,
                    Panel::Commits | Panel::MainView | Panel::Branches
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
                self.state.components.main_view_scroll_to(0);
                self.state.push_log(format!("Loaded diff for {file_path}"));
            }
            FrontendEvent::CommitFilesLoaded {
                commit_id, files, ..
            } => {
                if self.state.components.commit_pending_commit_id() != Some(commit_id.as_str()) {
                    self.state.push_log(format!(
                        "Ignored stale commit files response for {}",
                        short_commit_id(&commit_id)
                    ));
                    return Ok(());
                }

                // 找到对应的 commit 摘要
                let summary = self
                    .state
                    .data_cache
                    .commits
                    .iter()
                    .find(|c| c.id == commit_id)
                    .map(|c| c.summary.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                self.state.data_cache.commit_files = Some((commit_id.clone(), files.clone()));
                self.state.push_log(format!(
                    "Loaded {} files for commit {}",
                    files.len(),
                    short_commit_id(&commit_id)
                ));

                // 构建树并更新 CommitPanel 子视图
                use crate::components::core::build_tree_from_paths;
                use std::collections::HashMap;

                let paths: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
                let status_map: HashMap<String, crate::components::core::GitFileStatus> =
                    files.iter().cloned().collect();
                let tree_nodes = build_tree_from_paths(&paths, Some(&status_map));

                let tree_panel = crate::components::core::TreePanel::new(
                    format!("Files · {}", &summary),
                    tree_nodes,
                    false, // commit files 不需要空格操作
                );

                self.state.components.commit_panel.set_files_tree(
                    commit_id.clone(),
                    summary,
                    tree_panel,
                );

                if self.state.ui_state.active_panel == Panel::Commits {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::BranchGraphLoaded {
                branch_name, graph, ..
            } => {
                self.state
                    .data_cache
                    .branch_graphs
                    .insert(branch_name.clone(), graph.clone());
                self.state
                    .push_log(format!("Loaded branch graph for {branch_name}"));

                if self.state.ui_state.active_panel == Panel::Branches
                    && self
                        .state
                        .selected_branch()
                        .map(|branch| branch.name.as_str())
                        == Some(branch_name.as_str())
                {
                    self.state.data_cache.current_diff =
                        Some((format!("Main View · Branch Graph · {branch_name}"), graph));
                    self.state.components.main_view_scroll_to(0);
                }
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
        if self.state.ui_state.active_panel != panel {
            self.state.ui_state.active_panel = panel;
            self.state
                .push_log(format!("Focus moved to {}", panel.title()));
        }
        self.update_main_view_for_active_panel()
    }

    fn activate_panel(&mut self) -> Result<()> {
        match self.state.ui_state.active_panel {
            Panel::Commits => {
                if self.state.components.is_commit_list_multi_select_active() {
                    self.state.push_log(
                        "Commit list multi-select is active; Enter is disabled in this mode."
                            .to_string(),
                    );
                    return self.update_main_view_for_active_panel();
                }

                // 请求 commit 文件列表
                if let Some(commit) = self.state.selected_commit() {
                    let commit_id = commit.id.clone();
                    let summary = commit.summary.clone();
                    self.state
                        .components
                        .commit_panel
                        .start_loading(commit_id.clone(), summary.clone());
                    self.state.push_log(format!(
                        "Loading files for commit {}...",
                        short_commit_id(&commit_id)
                    ));

                    let request_id = self
                        .state
                        .send_command(BackendCommand::GetCommitFiles { commit_id })?;
                    self.pending_requests.insert(request_id);
                }
            }
            _ => {
                self.state.push_log(format!(
                    "Activated {}",
                    self.state.ui_state.active_panel.title()
                ));
            }
        }
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
                    self.state.components.commit_state_mut(),
                    self.state.data_cache.commits.len(),
                    1,
                );
                self.state
                    .components
                    .refresh_commit_list_multi_range(&self.state.data_cache.commits);
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
                self.scroll_main_view_by(1);
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
                    self.state.components.commit_state_mut(),
                    self.state.data_cache.commits.len(),
                    -1,
                );
                self.state
                    .components
                    .refresh_commit_list_multi_range(&self.state.data_cache.commits);
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
                self.scroll_main_view_by(-1);
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
            Panel::Branches => self.show_branch_detail()?,
            Panel::Commits => self.show_commits_panel_detail()?,
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
            "Repository snapshot\n\nCurrent branch: {current_branch}\nFiles: {} (staged: {staged}, unstaged: {unstaged})\nBranches: {}\nCommits loaded: {}\nStashes: {}\n\nNavigation\n- h/l: switch focus across left panels\n- 1/2/3/4: jump to Files/Branches/Commits/Stash\n- j/k or arrows: move inside the focused panel\n- Enter: refresh the current panel detail\n- r: refresh all Git-backed panels\n- Ctrl+d / Ctrl+u: globally page Main View\n- q: quit",
            self.state.data_cache.files.len(),
            self.state.data_cache.branches.len(),
            self.state.data_cache.commits.len(),
            self.state.data_cache.stashes.len()
        );

        self.state.data_cache.current_diff = Some(("Main View · Overview".to_string(), content));
        self.state.components.main_view_scroll_to(0);
    }

    fn request_selected_file_diff(&mut self) -> Result<()> {
        let targets = self.state.components.selected_file_tree_targets();
        if self.state.components.is_file_multi_select_active() && !targets.is_empty() {
            let deduped = dedupe_targets_parent_first(to_diff_targets(&targets));
            self.state.data_cache.current_diff = Some((
                "Main View · Files".to_string(),
                format!("Loading diff for {} selected targets...", deduped.len()),
            ));
            self.state.components.main_view_scroll_to(0);
            self.send_latest_diff_command(BackendCommand::GetDiffBatch { targets: deduped })?;
            return Ok(());
        }

        if let Some((path, is_dir)) = self.state.components.selected_file_tree_node() {
            let pathspec = if is_dir && !path.ends_with('/') {
                format!("{path}/")
            } else {
                path.clone()
            };
            let label = if is_dir {
                format!("{path}/")
            } else {
                path.clone()
            };

            self.state.data_cache.current_diff =
                Some((label.clone(), format!("Loading diff for {label}...")));
            self.state.components.main_view_scroll_to(0);

            self.send_latest_diff_command(BackendCommand::GetDiff {
                file_path: pathspec,
            })?;
        } else {
            self.state.data_cache.current_diff = Some((
                "Main View · Files".to_string(),
                "No file or folder is currently selected.".to_string(),
            ));
            self.state.components.main_view_scroll_to(0);
        }

        Ok(())
    }

    fn request_selected_commit_tree_diff(&mut self, commit_id: &str, summary: &str) -> Result<()> {
        let targets = self.state.components.selected_commit_tree_targets();
        if self.state.components.is_commit_tree_multi_select_active() && !targets.is_empty() {
            let deduped = dedupe_targets_parent_first(to_diff_targets(&targets));
            self.send_latest_diff_command(BackendCommand::GetCommitDiffBatch {
                commit_id: commit_id.to_string(),
                targets: deduped,
            })?;
            return Ok(());
        }

        if let Some((path, is_dir)) = self.state.components.selected_commit_tree_node() {
            self.send_latest_diff_command(BackendCommand::GetCommitDiff {
                commit_id: commit_id.to_string(),
                path,
                is_dir,
            })?;
        } else {
            self.state.data_cache.current_diff = Some((
                format!("Main View · Commit Files · {}", short_commit_id(commit_id)),
                format!(
                    "Commit files tree for: {summary}\n\nMove the cursor to a file or folder to preview its diff."
                ),
            ));
            self.state.components.main_view_scroll_to(0);
        }

        Ok(())
    }

    fn toggle_stage_selected_file(&mut self) -> Result<()> {
        let selected_targets = self.state.components.selected_file_tree_targets();
        let selected_files: Vec<String> = selected_targets
            .into_iter()
            .filter(|(_, is_dir)| !is_dir)
            .map(|(path, _)| path)
            .collect();
        if selected_files.is_empty() {
            return Ok(());
        }

        let anchor_file = self
            .state
            .components
            .selected_file_anchor_target()
            .and_then(|(path, is_dir)| (!is_dir).then_some(path))
            .or_else(|| selected_files.first().cloned());

        let Some(pivot_path) = anchor_file else {
            return Ok(());
        };
        let Some(file) = self
            .state
            .data_cache
            .files
            .iter()
            .find(|entry| entry.path == pivot_path)
        else {
            return Ok(());
        };
        let should_unstage = file.is_staged;

        let command = if selected_files.len() == 1 {
            let file_path = selected_files.into_iter().next().unwrap_or_default();
            if should_unstage {
                BackendCommand::UnstageFile { file_path }
            } else {
                BackendCommand::StageFile { file_path }
            }
        } else if should_unstage {
            BackendCommand::UnstageFiles {
                file_paths: selected_files,
            }
        } else {
            BackendCommand::StageFiles {
                file_paths: selected_files,
            }
        };

        let request_id = self.state.send_command(command)?;
        self.pending_requests.insert(request_id);
        Ok(())
    }

    fn show_branch_detail(&mut self) -> Result<()> {
        if let Some((name, is_head, upstream)) = self
            .state
            .selected_branch()
            .map(|branch| (branch.name.clone(), branch.is_head, branch.upstream.clone()))
        {
            if let Some(graph) = self.state.data_cache.branch_graphs.get(&name) {
                self.state.data_cache.current_diff =
                    Some((format!("Main View · Branch Graph · {name}"), graph.clone()));
                self.state.components.main_view_scroll_to(0);
                return Ok(());
            }

            let upstream = upstream.unwrap_or_else(|| "(no upstream)".to_string());
            let head_status = if is_head { "yes" } else { "no" };

            let content = format!(
                "Loading branch graph...\n\nBranch: {name}\nChecked out: {head_status}\nUpstream: {upstream}\n\nRunning: git log --graph --decorate --color=always --max-count={BRANCH_GRAPH_LIMIT} refs/heads/{name}"
            );
            self.state.data_cache.current_diff =
                Some((format!("Main View · Branch Graph · {name}"), content));
            self.send_latest_branch_graph_command(name)?;
        } else {
            self.state.data_cache.current_diff = Some((
                "Main View · Branches".to_string(),
                "No branches available.".to_string(),
            ));
        }
        self.state.components.main_view_scroll_to(0);
        Ok(())
    }

    fn show_commits_panel_detail(&mut self) -> Result<()> {
        match self.state.components.commit_mode_view() {
            CommitModeView::List => self.show_commit_detail(),
            CommitModeView::FilesLoading { commit_id, summary } => {
                self.show_commit_files_loading_detail(&commit_id, &summary)
            }
            CommitModeView::FilesTree { commit_id, summary } => {
                self.show_commit_files_tree_detail(&commit_id, &summary)?
            }
        }

        Ok(())
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

    fn show_commit_files_loading_detail(&mut self, commit_id: &str, summary: &str) {
        self.state.data_cache.current_diff = Some((
            format!("Main View · Commit Files · {}", short_commit_id(commit_id)),
            format!("Loading files for commit: {}\n\nPlease wait...", summary),
        ));
        self.state.components.main_view_scroll_to(0);
    }

    fn show_commit_files_tree_detail(&mut self, commit_id: &str, summary: &str) -> Result<()> {
        self.request_selected_commit_tree_diff(commit_id, summary)
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
        let content = "The Log panel records UI focus changes, refreshes, and backend responses.\n\nUse j/k to scroll this panel.\nCtrl+d / Ctrl+u pages Main View globally.".to_string();
        self.state.data_cache.current_diff = Some(("Main View · Log Help".to_string(), content));
        self.state.components.main_view_scroll_to(0);
    }

    fn scroll_main_view_by(&mut self, delta: i16) {
        let max_scroll = self.current_main_view_max_scroll();
        self.state.components.scroll_main_view_by(delta, max_scroll);
    }

    fn current_main_view_max_scroll(&self) -> u16 {
        let Some((_, content)) = self.state.data_cache.current_diff.as_ref() else {
            return 0;
        };

        let max_lines = content.lines().count().saturating_sub(1);
        u16::try_from(max_lines).unwrap_or(u16::MAX)
    }

    fn send_latest_diff_command(&mut self, command: BackendCommand) -> Result<()> {
        let request_id = self.state.send_command(command)?;

        if let Some(previous) = self.latest_diff_request_id.replace(request_id) {
            self.pending_requests.remove(&previous);
        }

        self.pending_requests.insert(request_id);
        Ok(())
    }

    fn send_latest_branch_graph_command(&mut self, branch_name: String) -> Result<()> {
        let request_id = self.state.send_command(BackendCommand::GetBranchGraph {
            branch_name,
            limit: BRANCH_GRAPH_LIMIT,
        })?;

        if let Some(previous) = self.latest_branch_graph_request_id.replace(request_id) {
            self.pending_requests.remove(&previous);
        }

        self.pending_requests.insert(request_id);
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
            .split(frame.area());

        let left_heights =
            compute_left_panel_heights(columns[0].height, self.state.ui_state.active_panel);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints(left_heights.map(Constraint::Length))
            .split(columns[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(12), Constraint::Length(8)])
            .split(columns[1]);

        let mut rendered_slots = HashSet::new();

        Self::prepare_slot(frame, left[0], UiSlot::Files, &mut rendered_slots);
        self.state.components.file_list_panel.render(
            frame,
            left[0],
            self.state.ui_state.active_panel == Panel::Files,
            &self.state.data_cache,
        );

        Self::prepare_slot(frame, left[1], UiSlot::Branches, &mut rendered_slots);
        self.state.components.branch_list_panel.render(
            frame,
            left[1],
            self.state.ui_state.active_panel == Panel::Branches,
            &self.state.data_cache,
        );

        Self::prepare_slot(frame, left[2], UiSlot::Commits, &mut rendered_slots);
        self.state.components.commit_panel.render(
            frame,
            left[2],
            self.state.ui_state.active_panel == Panel::Commits,
            &self.state.data_cache,
        );

        Self::prepare_slot(frame, left[3], UiSlot::Stash, &mut rendered_slots);
        self.state.components.stash_list_panel.render(
            frame,
            left[3],
            self.state.ui_state.active_panel == Panel::Stash,
            &self.state.data_cache,
        );

        Self::prepare_slot(frame, right[0], UiSlot::MainView, &mut rendered_slots);
        self.state.components.main_view_panel.render(
            frame,
            right[0],
            self.state.ui_state.active_panel == Panel::MainView,
            &self.state.data_cache,
        );

        Self::prepare_slot(frame, right[1], UiSlot::Log, &mut rendered_slots);
        self.state.components.log_panel.render(
            frame,
            right[1],
            self.state.ui_state.active_panel == Panel::Log,
            &self.state.data_cache,
        );
    }

    fn prepare_slot(
        frame: &mut Frame,
        area: Rect,
        slot: UiSlot,
        rendered_slots: &mut HashSet<UiSlot>,
    ) {
        debug_assert!(
            rendered_slots.insert(slot),
            "UI slot rendered more than once in the same frame: {:?}",
            slot
        );
        frame.render_widget(Clear, area);
    }
}

const LEFT_FILES_INDEX: usize = 0;
const LEFT_BRANCHES_INDEX: usize = 1;
const LEFT_COMMITS_INDEX: usize = 2;
const LEFT_STASH_INDEX: usize = 3;
const LEFT_DYNAMIC_MIN_HEIGHT: u16 = 18;
const STASH_COLLAPSED_HEIGHT: u16 = 3;
const FOCUSED_PANEL_MIN_HEIGHT: u16 = 7;
const BRANCH_GRAPH_LIMIT: usize = 80;
const MAIN_VIEW_PAGE_SCROLL: i16 = 12;

fn compute_left_panel_heights(total_height: u16, active_panel: Panel) -> [u16; 4] {
    if total_height < LEFT_DYNAMIC_MIN_HEIGHT {
        return distribute_weighted(total_height, [28, 24, 28, 20]);
    }

    let mut heights = [0_u16; 4];

    match active_panel {
        Panel::Files | Panel::Branches | Panel::Commits => {
            let focused_index = match active_panel {
                Panel::Files => LEFT_FILES_INDEX,
                Panel::Branches => LEFT_BRANCHES_INDEX,
                Panel::Commits => LEFT_COMMITS_INDEX,
                _ => unreachable!("focused panel for this branch must be one of left top three"),
            };

            heights[LEFT_STASH_INDEX] = STASH_COLLAPSED_HEIGHT;
            let remaining_for_top_three = total_height.saturating_sub(STASH_COLLAPSED_HEIGHT);
            let focused_height = (remaining_for_top_three / 2)
                .max(FOCUSED_PANEL_MIN_HEIGHT)
                .min(remaining_for_top_three);

            heights[focused_index] = focused_height;

            let remaining = remaining_for_top_three.saturating_sub(focused_height);
            let mut non_focused_top_indices = [0_usize; 2];
            let mut write_index = 0;
            for index in [LEFT_FILES_INDEX, LEFT_BRANCHES_INDEX, LEFT_COMMITS_INDEX] {
                if index != focused_index {
                    non_focused_top_indices[write_index] = index;
                    write_index += 1;
                }
            }
            distribute_evenly_into(remaining, &non_focused_top_indices, &mut heights);
        }
        Panel::Stash => {
            let stash_height = (total_height / 2)
                .max(FOCUSED_PANEL_MIN_HEIGHT)
                .min(total_height);
            heights[LEFT_STASH_INDEX] = stash_height;
            let remaining = total_height.saturating_sub(stash_height);
            distribute_evenly_into(
                remaining,
                &[LEFT_FILES_INDEX, LEFT_BRANCHES_INDEX, LEFT_COMMITS_INDEX],
                &mut heights,
            );
        }
        Panel::MainView | Panel::Log => {
            heights[LEFT_STASH_INDEX] = STASH_COLLAPSED_HEIGHT;
            let remaining = total_height.saturating_sub(STASH_COLLAPSED_HEIGHT);
            distribute_evenly_into(
                remaining,
                &[LEFT_FILES_INDEX, LEFT_BRANCHES_INDEX, LEFT_COMMITS_INDEX],
                &mut heights,
            );
        }
    }

    heights
}

fn distribute_evenly_into(total: u16, target_indices: &[usize], heights: &mut [u16; 4]) {
    if target_indices.is_empty() {
        return;
    }

    let len = target_indices.len() as u16;
    let base = total / len;
    let mut remainder = total % len;

    for index in target_indices {
        let mut value = base;
        if remainder > 0 {
            value += 1;
            remainder -= 1;
        }
        heights[*index] = value;
    }
}

fn distribute_weighted(total: u16, weights: [u16; 4]) -> [u16; 4] {
    let mut heights = [0_u16; 4];
    let sum: u16 = weights.iter().sum();
    let mut consumed = 0_u16;

    for (idx, weight) in weights.into_iter().enumerate() {
        let value = total.saturating_mul(weight) / sum;
        heights[idx] = value;
        consumed = consumed.saturating_add(value);
    }

    let mut remainder = total.saturating_sub(consumed);
    for value in &mut heights {
        if remainder == 0 {
            break;
        }
        *value = value.saturating_add(1);
        remainder -= 1;
    }

    heights
}

fn next_left_panel(current: Panel) -> Panel {
    match current {
        Panel::Files => Panel::Branches,
        Panel::Branches => Panel::Commits,
        Panel::Commits => Panel::Stash,
        Panel::Stash => Panel::Files,
        Panel::MainView | Panel::Log => Panel::Files,
    }
}

fn previous_left_panel(current: Panel) -> Panel {
    match current {
        Panel::Files => Panel::Stash,
        Panel::Branches => Panel::Files,
        Panel::Commits => Panel::Branches,
        Panel::Stash => Panel::Commits,
        Panel::MainView | Panel::Log => Panel::Stash,
    }
}

fn short_commit_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn to_diff_targets(targets: &[(String, bool)]) -> Vec<DiffTarget> {
    targets
        .iter()
        .map(|(path, is_dir)| DiffTarget {
            path: path.clone(),
            is_dir: *is_dir,
        })
        .collect()
}

fn dedupe_targets_parent_first(targets: Vec<DiffTarget>) -> Vec<DiffTarget> {
    let selected_dirs: Vec<String> = targets
        .iter()
        .filter(|target| target.is_dir)
        .map(|target| normalize_path(&target.path))
        .collect();
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();

    for target in targets {
        let normalized = normalize_path(&target.path);
        let parent_selected = selected_dirs
            .iter()
            .any(|dir| dir != &normalized && normalized.starts_with(format!("{dir}/").as_str()));
        if parent_selected {
            continue;
        }

        let key = format!("{}:{}", normalized, target.is_dir);
        if seen.insert(key) {
            deduped.push(target);
        }
    }

    deduped
}

fn normalize_path(path: &str) -> String {
    path.trim_end_matches('/').to_string()
}

fn cycle_selection(state: &mut ListState, len: usize, delta: i8) {
    if len == 0 {
        state.select(None);
        return;
    }

    let current = state.selected().unwrap_or(0);
    let next = if delta > 0 {
        current.saturating_add(1).min(len.saturating_sub(1))
    } else {
        current.saturating_sub(1)
    };

    state.select(Some(next));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_heights_expand_focused_files_panel_and_collapse_stash() {
        let heights = compute_left_panel_heights(30, Panel::Files);

        assert_eq!(heights[LEFT_STASH_INDEX], STASH_COLLAPSED_HEIGHT);
        assert!(heights[LEFT_FILES_INDEX] > heights[LEFT_BRANCHES_INDEX]);
        assert!(heights[LEFT_FILES_INDEX] > heights[LEFT_COMMITS_INDEX]);
        assert_eq!(heights.iter().sum::<u16>(), 30);
    }

    #[test]
    fn left_heights_expand_stash_when_stash_is_focused() {
        let heights = compute_left_panel_heights(30, Panel::Stash);

        assert_eq!(heights[LEFT_STASH_INDEX], 15);
        assert_eq!(heights[LEFT_FILES_INDEX], 5);
        assert_eq!(heights[LEFT_BRANCHES_INDEX], 5);
        assert_eq!(heights[LEFT_COMMITS_INDEX], 5);
        assert_eq!(heights.iter().sum::<u16>(), 30);
    }

    #[test]
    fn left_heights_keep_stash_collapsed_when_right_side_is_focused() {
        let heights = compute_left_panel_heights(25, Panel::MainView);

        assert_eq!(heights[LEFT_STASH_INDEX], STASH_COLLAPSED_HEIGHT);
        let top_three = [
            heights[LEFT_FILES_INDEX],
            heights[LEFT_BRANCHES_INDEX],
            heights[LEFT_COMMITS_INDEX],
        ];
        let max = *top_three.iter().max().expect("top three should have items");
        let min = *top_three.iter().min().expect("top three should have items");
        assert!(max - min <= 1);
        assert_eq!(heights.iter().sum::<u16>(), 25);
    }

    #[test]
    fn left_heights_fall_back_to_legacy_weights_on_small_terminal() {
        let heights = compute_left_panel_heights(10, Panel::Files);

        assert_eq!(heights, [3, 3, 2, 2]);
        assert_eq!(heights.iter().sum::<u16>(), 10);
    }

    #[test]
    fn cycle_selection_does_not_wrap_forward() {
        let mut state = ListState::default();
        state.select(Some(2));

        cycle_selection(&mut state, 3, 1);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn cycle_selection_does_not_wrap_backward() {
        let mut state = ListState::default();
        state.select(Some(0));

        cycle_selection(&mut state, 3, -1);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn left_panel_navigation_stays_on_left_panels_only() {
        assert_eq!(next_left_panel(Panel::Files), Panel::Branches);
        assert_eq!(next_left_panel(Panel::Stash), Panel::Files);
        assert_eq!(next_left_panel(Panel::MainView), Panel::Files);

        assert_eq!(previous_left_panel(Panel::Files), Panel::Stash);
        assert_eq!(previous_left_panel(Panel::Branches), Panel::Files);
        assert_eq!(previous_left_panel(Panel::Log), Panel::Stash);
    }

    #[test]
    fn dedupe_targets_prefers_parent_directory() {
        let targets = vec![
            DiffTarget {
                path: "src".to_string(),
                is_dir: true,
            },
            DiffTarget {
                path: "src/main.rs".to_string(),
                is_dir: false,
            },
            DiffTarget {
                path: "README.md".to_string(),
                is_dir: false,
            },
        ];

        let deduped = dedupe_targets_parent_first(targets);
        assert_eq!(deduped.len(), 2);
        assert!(deduped
            .iter()
            .any(|target| target.path == "src" && target.is_dir));
        assert!(deduped
            .iter()
            .any(|target| target.path == "README.md" && !target.is_dir));
    }
}
