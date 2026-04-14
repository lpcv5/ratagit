use anyhow::Result;
use ratatui::widgets::ListState;

use crate::backend::{BackendCommand, DiffTarget};
use crate::components::panels::CommitModeView;
use crate::components::Intent;
use crate::shared::path_utils::dedupe_targets_parent_first;

use super::ui_state::Panel;
use super::App;

const BRANCH_GRAPH_LIMIT: usize = 80;

impl App {
    pub(super) fn execute_intent(&mut self, intent: Intent) -> Result<()> {
        match intent {
            Intent::SelectNext => self.navigate_forward()?,
            Intent::SelectPrevious => self.navigate_backward()?,
            Intent::SwitchFocus(panel) => self.set_active_panel(panel)?,
            Intent::RefreshPanelDetail => self.update_main_view_for_active_panel()?,
            Intent::ScrollMainView(delta) => self.scroll_main_view_by(delta),
            Intent::ScrollLog(delta) => self.state.components.scroll_log_by(delta),
            Intent::ActivatePanel => self.activate_panel()?,
            Intent::ToggleStageFile => self.toggle_stage_selected_file()?,
            Intent::StageAll => self.stage_all()?,
            Intent::DiscardSelected => self.discard_selected()?,
            Intent::StashSelected => self.stash_selected()?,
            Intent::AmendCommit => self.amend_commit()?,
            Intent::ShowResetMenu => self.show_reset_menu()?,
            Intent::ExecuteResetOption(index) => self.execute_reset_option(index)?,
            Intent::CloseModal => self.close_modal(),
            Intent::SendCommand(cmd) => {
                let request_id = self.state.send_command(cmd)?;
                self.requests.track(request_id);
                // Close modal after sending command (for confirmation dialogs)
                self.close_modal();
            }
            Intent::None => {}
        }
        Ok(())
    }

    pub(super) fn update_main_view_for_active_panel(&mut self) -> Result<()> {
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

    pub(super) fn request_refresh_all(&mut self) {
        for cmd in [
            BackendCommand::RefreshStatus,
            BackendCommand::RefreshBranches,
            BackendCommand::RefreshCommits { limit: 30 },
            BackendCommand::RefreshStashes,
        ] {
            if let Ok(id) = self.state.send_command(cmd) {
                if id != 0 {
                    self.requests.track(id);
                }
            }
        }
    }

    pub(super) fn scroll_main_view_by(&mut self, delta: i16) {
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

    fn set_active_panel(&mut self, panel: Panel) -> Result<()> {
        if panel == Panel::Branches {
            if let Some(saved) = self.state.data_cache.saved_commits.take() {
                self.state.data_cache.commits = saved;
                self.state.sync_commit_list_state();
            }
        }
        if self.state.ui_state.active_panel != panel {
            self.state.ui_state.active_panel = panel;
            self.state
                .push_log(format!("Focus moved to {}", panel.title()));
        }
        self.update_main_view_for_active_panel()
    }

    fn activate_panel(&mut self) -> Result<()> {
        match self.state.ui_state.active_panel {
            Panel::Branches => {
                if let Some(branch) = self.state.selected_branch() {
                    let branch_name = branch.name.clone();
                    self.state.push_log(format!("Loading commits for branch {branch_name}..."));
                    let request_id = self.state.send_command(BackendCommand::GetBranchCommits {
                        branch_name,
                        limit: 50,
                    })?;
                    self.requests.set_latest_branch_commits(request_id);
                }
            }
            Panel::Commits => {
                if self.state.components.is_commit_list_multi_select_active() {
                    self.state.push_log(
                        "Commit list multi-select is active; Enter is disabled in this mode."
                            .to_string(),
                    );
                    return self.update_main_view_for_active_panel();
                }
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
                    self.requests.track(request_id);
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
                    self.state.components.file_list_panel.state_mut(),
                    self.state.data_cache.files.len(),
                    1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Branches => {
                cycle_selection(
                    self.state.components.branch_list_panel.state_mut(),
                    self.state.data_cache.branches.len(),
                    1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Commits => {
                cycle_selection(
                    self.state.components.commit_panel.state_mut(),
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
                    self.state.components.stash_list_panel.state_mut(),
                    self.state.data_cache.stashes.len(),
                    1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::MainView => self.scroll_main_view_by(1),
            Panel::Log => self.state.components.scroll_log_by(1),
        }
        Ok(())
    }

    fn navigate_backward(&mut self) -> Result<()> {
        match self.state.ui_state.active_panel {
            Panel::Files => {
                cycle_selection(
                    self.state.components.file_list_panel.state_mut(),
                    self.state.data_cache.files.len(),
                    -1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Branches => {
                cycle_selection(
                    self.state.components.branch_list_panel.state_mut(),
                    self.state.data_cache.branches.len(),
                    -1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::Commits => {
                cycle_selection(
                    self.state.components.commit_panel.state_mut(),
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
                    self.state.components.stash_list_panel.state_mut(),
                    self.state.data_cache.stashes.len(),
                    -1,
                );
                self.update_main_view_for_active_panel()?;
            }
            Panel::MainView => self.scroll_main_view_by(-1),
            Panel::Log => self.state.components.scroll_log_by(-1),
        }
        Ok(())
    }

    fn show_repo_overview(&mut self) {
        let current_branch = self
            .state
            .data_cache
            .branches
            .iter()
            .find(|b| b.is_head)
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "(detached)".to_string());
        let staged = self
            .state
            .data_cache
            .files
            .iter()
            .filter(|f| f.is_staged)
            .count();
        let unstaged = self
            .state
            .data_cache
            .files
            .iter()
            .filter(|f| f.is_unstaged)
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
            let targets = to_diff_targets(&targets);
            let deduped = dedupe_targets_parent_first(&targets);
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
            let targets = to_diff_targets(&targets);
            let deduped = dedupe_targets_parent_first(&targets);
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
                format!("Commit files tree for: {summary}\n\nMove the cursor to a file or folder to preview its diff."),
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
            .find(|e| e.path == pivot_path)
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
        self.requests.track(request_id);
        Ok(())
    }

    fn show_branch_detail(&mut self) -> Result<()> {
        if let Some((name, is_head, upstream)) = self
            .state
            .selected_branch()
            .map(|b| (b.name.clone(), b.is_head, b.upstream.clone()))
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
            self.state.selected_commit().map(|c| {
                (
                    c.short_id.clone(),
                    c.id.clone(),
                    c.author.clone(),
                    c.timestamp,
                    c.summary.clone(),
                    c.body.clone(),
                )
            })
        {
            let body = body
                .as_deref()
                .filter(|v| !v.is_empty())
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
            .map(|s| (s.index, s.id.clone(), s.message.clone()))
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

    fn send_latest_diff_command(&mut self, command: BackendCommand) -> Result<()> {
        let request_id = self.state.send_command(command)?;
        self.requests.set_latest_diff(request_id);
        Ok(())
    }

    fn send_latest_branch_graph_command(&mut self, branch_name: String) -> Result<()> {
        let request_id = self.state.send_command(BackendCommand::GetBranchGraph {
            branch_name,
            limit: BRANCH_GRAPH_LIMIT,
        })?;
        self.requests.set_latest_branch_graph(request_id);
        Ok(())
    }

    fn stage_all(&mut self) -> Result<()> {
        let request_id = self.state.send_command(BackendCommand::StageAll)?;
        if request_id != 0 {
            self.requests.track(request_id);
        }
        Ok(())
    }

    fn discard_selected(&mut self) -> Result<()> {
        let paths = if self.state.components.file_list_panel.is_multi_select_active() {
            self.state
                .components
                .file_list_panel
                .selected_tree_targets()
                .into_iter()
                .map(|(path, _)| path)
                .collect()
        } else {
            if let Some((path, _)) = self.state.components.file_list_panel.selected_tree_node() {
                vec![path]
            } else {
                return Ok(());
            }
        };

        if paths.is_empty() {
            return Ok(());
        }

        // Show confirmation modal
        use crate::components::ModalDialog;
        let modal = ModalDialog::confirmation(
            "Discard Changes".to_string(),
            format!("Discard changes to {} file(s)?\nThis cannot be undone.", paths.len()),
            Intent::SendCommand(BackendCommand::DiscardFiles { paths }),
        );
        self.state.active_modal = Some(modal);
        Ok(())
    }

    fn stash_selected(&mut self) -> Result<()> {
        let paths = if self.state.components.file_list_panel.is_multi_select_active() {
            self.state
                .components
                .file_list_panel
                .selected_tree_targets()
                .into_iter()
                .map(|(path, _)| path)
                .collect()
        } else {
            if let Some((path, _)) = self.state.components.file_list_panel.selected_tree_node() {
                vec![path]
            } else {
                return Ok(());
            }
        };

        if paths.is_empty() {
            return Ok(());
        }

        let request_id = self.state.send_command(BackendCommand::StashFiles {
            paths,
            message: None,
        })?;
        if request_id != 0 {
            self.requests.track(request_id);
        }
        Ok(())
    }

    fn amend_commit(&mut self) -> Result<()> {
        // Get selected files from Files panel
        let paths = self.state.components.file_list_panel.selected_tree_targets();

        if paths.is_empty() {
            self.state.push_log("No files selected for amend".to_string());
            return Ok(());
        }

        // Get selected commit from Commits panel
        let selected_commit = self.state.components.commit_panel.selected_commit(&self.state.data_cache.commits);

        if let Some(commit) = selected_commit {
            let commit_id = commit.id.clone();

            // Check if this is HEAD (first commit in the list)
            let is_head = self.state.data_cache.commits.first()
                .map(|c| c.id == commit_id)
                .unwrap_or(false);

            if !is_head {
                self.state.push_log("Can only amend HEAD commit. Please select the most recent commit.".to_string());
                return Ok(());
            }

            // For now, use a simple confirmation dialog
            use crate::components::ModalDialog;
            let message = format!(
                "Amend HEAD with {} selected file(s)?\n\nThis will add the selected files to the last commit.\nThe commit message will remain unchanged.\n\nPress 'y' to confirm, 'n' to cancel.",
                paths.len()
            );

            self.state.active_modal = Some(ModalDialog::confirmation(
                "Amend Commit".to_string(),
                message,
                Intent::SendCommand(crate::backend::BackendCommand::AmendCommitWithFiles {
                    commit_id,
                    message: format!("{}{}",
                        commit.summary,
                        commit.body.as_ref().map(|b| format!("\n\n{}", b)).unwrap_or_default()
                    ),
                    paths: paths.into_iter().map(|(path, _)| path).collect(),
                }),
            ));
        } else {
            self.state.push_log("No commit selected. Please select HEAD in the Commits panel.".to_string());
        }

        Ok(())
    }

    fn show_reset_menu(&mut self) -> Result<()> {
        use crate::components::ModalDialog;
        let options = vec![
            "Hard Reset (HEAD) - Discard all changes".to_string(),
            "Mixed Reset (HEAD) - Unstage all, keep changes".to_string(),
            "Soft Reset (HEAD) - Keep staged changes".to_string(),
            "Hard Reset (HEAD~1) - Undo last commit, discard changes".to_string(),
            "Soft Reset (HEAD~1) - Undo last commit, keep staged".to_string(),
            "Nuke Repository - Delete .git directory".to_string(),
        ];
        let modal = ModalDialog::selection("Reset Options".to_string(), options);
        self.state.active_modal = Some(modal);
        Ok(())
    }

    fn execute_reset_option(&mut self, index: usize) -> Result<()> {
        let (target, reset_type, needs_confirmation) = match index {
            0 => ("HEAD", "hard", true),
            1 => ("HEAD", "mixed", false),
            2 => ("HEAD", "soft", false),
            3 => ("HEAD~1", "hard", true),
            4 => ("HEAD~1", "soft", false),
            5 => {
                // Nuke repo - needs double confirmation
                use crate::components::ModalDialog;
                let modal = ModalDialog::confirmation(
                    "NUKE REPOSITORY".to_string(),
                    "Are you ABSOLUTELY SURE?\nThis will DELETE the .git directory.\nThis CANNOT be undone!".to_string(),
                    Intent::None, // TODO: Implement nuke
                );
                self.state.active_modal = Some(modal);
                return Ok(());
            }
            _ => return Ok(()),
        };

        let command = match reset_type {
            "hard" => BackendCommand::ResetHard { target: target.to_string() },
            "mixed" => BackendCommand::ResetMixed { target: target.to_string() },
            "soft" => BackendCommand::ResetSoft { target: target.to_string() },
            _ => return Ok(()),
        };

        if needs_confirmation {
            use crate::components::ModalDialog;
            let modal = ModalDialog::confirmation(
                "Confirm Reset".to_string(),
                format!("Reset {} to {}?\nThis will discard changes.", reset_type, target),
                Intent::SendCommand(command),
            );
            self.state.active_modal = Some(modal);
        } else {
            self.state.active_modal = None;
            let request_id = self.state.send_command(command)?;
            if request_id != 0 {
                self.requests.track(request_id);
            }
        }
        Ok(())
    }

    fn close_modal(&mut self) {
        self.state.active_modal = None;
    }
}

pub(super) fn cycle_selection(state: &mut ListState, len: usize, delta: i8) {
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

pub(super) fn short_commit_id(id: &str) -> String {
    id.chars().take(8).collect()
}

pub(super) fn to_diff_targets(targets: &[(String, bool)]) -> Vec<DiffTarget> {
    targets
        .iter()
        .map(|(path, is_dir)| DiffTarget {
            path: path.clone(),
            is_dir: *is_dir,
        })
        .collect()
}
