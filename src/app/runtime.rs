use std::time::Duration;

use anyhow::Result;
use crossterm::event;
use tokio::sync::mpsc::error::TryRecvError;

use super::events::AppEvent;
use super::processors::{git_processor::GitProcessor, modal_processor::ModalProcessor};
use super::request_tracker::RequestTracker;
use super::state::AppState;
use super::Panel;
use crate::backend::{EventEnvelope, FrontendEvent};
use crate::components::core::build_tree_from_paths;

pub struct App {
    pub(super) state: AppState,
    pub(super) requests: RequestTracker,
    git_processor: GitProcessor,
    modal_processor: ModalProcessor,
}

impl App {
    pub fn new(
        cmd_tx: tokio::sync::mpsc::Sender<crate::backend::CommandEnvelope>,
        event_rx: tokio::sync::mpsc::Receiver<crate::backend::EventEnvelope>,
    ) -> Self {
        Self {
            state: AppState::new(cmd_tx, event_rx),
            requests: RequestTracker::new(),
            git_processor: GitProcessor,
            modal_processor: ModalProcessor,
        }
    }

    /// Process an AppEvent by routing it to the appropriate processor
    pub fn process_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::None => {}
            AppEvent::Git(git_event) => {
                let commands = self.git_processor.process(git_event, &self.state);
                for cmd in commands {
                    if let Err(e) = self.state.send_command(cmd) {
                        self.state.push_log(format!("Failed to send command: {}", e));
                    }
                }
            }
            AppEvent::Modal(modal_event) => {
                self.modal_processor.process(modal_event, &mut self.state);
            }
            AppEvent::SwitchPanel(panel) => {
                self.state.ui_state.active_panel = panel;
            }
            AppEvent::ActivatePanel => {
                self.handle_activate_panel();
            }
            AppEvent::SelectionChanged => {
                // Selection changed - no action needed, just UI update
            }
        }
    }

    fn handle_activate_panel(&mut self) {
        match self.state.ui_state.active_panel {
            Panel::Files => {
                // Show diff for selected file
                // (implementation depends on your diff logic)
            }
            Panel::Branches => {
                // Checkout selected branch
                // (implementation depends on your checkout logic)
            }
            // Handle other panels...
            _ => {}
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

    async fn main_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        while !self.state.should_quit {
            self.drain_backend_events().await?;
            terminal.draw(|frame| self.render(frame))?;
            if event::poll(Duration::from_millis(100))? {
                let input = event::read()?;

                // If modal is active, let it handle the input first
                if let Some(ref mut modal) = self.state.active_modal {
                    let app_event = modal.handle_event_v2(&input);
                    self.process_event(app_event);
                } else {
                    self.handle_input_v2(input)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn request_refresh_all(&mut self) {
        use crate::backend::BackendCommand;
        for cmd in [
            BackendCommand::RefreshStatus,
            BackendCommand::RefreshBranches,
            BackendCommand::RefreshCommits { limit: 30 },
            BackendCommand::RefreshStashes,
        ] {
            if let Err(e) = self.state.send_command(cmd) {
                self.state.push_log(format!("Failed to send refresh command: {}", e));
            }
        }
    }

    pub(super) fn update_main_view_for_active_panel(&mut self) -> Result<()> {
        // TODO: Implement proper main view updates for each panel
        // This is a stub to allow compilation after Intent system deletion
        // The old implementation was in intent_executor.rs and needs to be
        // refactored to work with the new event-driven architecture
        Ok(())
    }

    pub(super) fn scroll_main_view_by(&mut self, delta: i16) {
        // Calculate max scroll based on current diff content
        let max_scroll = if let Some((_, content)) = self.state.data_cache.current_diff.as_ref() {
            let max_lines = content.lines().count().saturating_sub(1);
            u16::try_from(max_lines).unwrap_or(u16::MAX)
        } else {
            0
        };
        self.state.components.scroll_main_view_by(delta, max_scroll);
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

    fn handle_backend_event(&mut self, envelope: EventEnvelope) -> Result<()> {
        let request_id = envelope.request_id;
        if let Some(id) = request_id {
            if !self.requests.complete(id) {
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
                            .any(|b| b.name == *branch_name)
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

                let paths: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
                let status_map: std::collections::HashMap<
                    String,
                    crate::components::core::GitFileStatus,
                > = files.iter().cloned().collect();
                let tree_nodes = build_tree_from_paths(&paths, Some(&status_map));
                let tree_panel = crate::components::core::TreePanel::new(
                    format!("Files · {}", &summary),
                    tree_nodes,
                    false,
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
            FrontendEvent::BranchCommitsLoaded {
                request_id,
                branch_name,
                commits,
            } => {
                if !self.requests.is_latest_branch_commits(request_id) {
                    self.state.push_log(format!(
                        "Ignored stale branch commits response for {branch_name}"
                    ));
                    return Ok(());
                }
                self.state.push_log(format!(
                    "Loaded {} commits for branch {branch_name}",
                    commits.len()
                ));
                self.state.data_cache.saved_commits = Some(std::mem::replace(
                    &mut self.state.data_cache.commits,
                    commits,
                ));
                self.state.components.show_branch_commits();
                if self.state.ui_state.active_panel == Panel::Branches {
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
                    && self.state.selected_branch().map(|b| b.name.as_str())
                        == Some(branch_name.as_str())
                {
                    self.state.data_cache.current_diff =
                        Some((format!("Main View · Branch Graph · {branch_name}"), graph));
                    self.state.components.main_view_scroll_to(0);
                }
            }
            FrontendEvent::Error { message, .. } => {
                self.state.push_log(format!("Error: {message}"))
            }
            FrontendEvent::ActionSucceeded { message, .. } => {
                self.state.push_log(format!("OK: {message}"))
            }
            FrontendEvent::CommitMessageLoaded { message, .. } => {
                // Store the commit message for amend operation
                self.state.data_cache.pending_commit_message = Some(message);
            }
            FrontendEvent::ActionFailed { message, .. } => {
                self.state.push_log(format!("FAILED: {message}"))
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::{AppEvent, GitEvent, ModalEvent};
    use crate::app::Panel;
    use tokio::sync::mpsc;

    fn create_test_app() -> App {
        let (cmd_tx, _cmd_rx) = mpsc::channel(100);
        let (_event_tx, event_rx) = mpsc::channel(100);
        App::new(cmd_tx, event_rx)
    }

    #[test]
    fn test_app_process_event_git() {
        let mut app = create_test_app();
        app.process_event(AppEvent::Git(GitEvent::StageAll));

        // Verify BackendCommand was sent
        // (check cmd_tx channel has StageAll command)
    }

    #[test]
    fn test_app_process_event_modal() {
        let mut app = create_test_app();
        app.process_event(AppEvent::Modal(ModalEvent::ShowHelp));

        // Verify modal was set
        assert!(app.state.active_modal.is_some());
    }

    #[test]
    fn test_app_process_event_switch_panel() {
        let mut app = create_test_app();
        app.process_event(AppEvent::SwitchPanel(Panel::Branches));

        assert_eq!(app.state.ui_state.active_panel, Panel::Branches);
    }

    #[test]
    fn test_app_process_event_none() {
        let mut app = create_test_app();
        // Should not panic
        app.process_event(AppEvent::None);
    }
}

/// Helper function to shorten commit IDs for display
fn short_commit_id(id: &str) -> String {
    if id.len() > 7 {
        id[..7].to_string()
    } else {
        id.to_string()
    }
}

