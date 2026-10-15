//! Adapt revision decisions to shared task scheduling and cross-feature updates.
use super::diff::DiffRefreshSource;
use super::revisions::{RevisionAction, RevisionCancel, RevisionRead, RevisionUpdate};
use super::{AppState, BackgroundLoad, BackgroundPayload, SidePanel};
use crate::background::{TaskKey, TaskPriority};
use crate::flux::command_output::CommandOutput;
use tracing::debug;

impl AppState {
    pub(super) fn start_branches_load(&mut self, output: &mut CommandOutput) -> bool {
        let key = TaskKey::Branches;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }

        self.queue_background(BackgroundLoad::Branches, priority, output);
        true
    }

    pub(super) fn start_stashes_load(&mut self, output: &mut CommandOutput) -> bool {
        let key = TaskKey::Stashes;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }

        self.queue_background(BackgroundLoad::Stashes, priority, output);
        true
    }

    pub(super) fn start_commits_load(&mut self, limit: usize, output: &mut CommandOutput) -> bool {
        let key = TaskKey::Commits;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }
        self.reduce_revisions(RevisionAction::TrackCommitsLoad(limit));
        let fast = self.revisions.view().commits.items.is_empty();
        self.queue_background(BackgroundLoad::Commits { limit, fast }, priority, output);
        true
    }

    pub(super) fn reduce_revisions(&mut self, action: RevisionAction) -> RevisionUpdate {
        self.revisions.update(action, self.ui.active_panel)
    }

    fn apply_revision_update(&mut self, update: RevisionUpdate, output: &mut CommandOutput) {
        if let Some(cancel) = update.cancel {
            self.cancel_background_task(
                &match cancel {
                    RevisionCancel::CommitFiles => TaskKey::CommitFiles,
                    RevisionCancel::StashFiles => TaskKey::StashFiles,
                },
                output,
            );
        }
        if let Some(read) = update.read {
            let load = match read {
                RevisionRead::Commits { limit } => {
                    self.start_commits_load(limit, output);
                    return;
                }
                RevisionRead::BranchCommits { branch, limit } => {
                    BackgroundLoad::BranchCommits { branch, limit }
                }
                RevisionRead::CommitFiles { oid } => BackgroundLoad::CommitFiles { oid },
                RevisionRead::StashFiles { index } => BackgroundLoad::StashFiles { index },
            };
            self.queue_background(load, TaskPriority::High, output);
        }
        if let Some((message, success)) = update.log {
            self.push_log(message, success);
        }
    }

    pub fn open_selected_branch_commits(&mut self, limit: usize, output: &mut CommandOutput) {
        let update = self.reduce_revisions(RevisionAction::OpenBranchCommits { limit });
        self.apply_revision_update(update, output);
    }

    pub fn close_branch_commits_subview(&mut self, output: &mut CommandOutput) {
        let update = self.reduce_revisions(RevisionAction::CloseBranchCommits);
        self.apply_revision_update(update, output);
    }

    pub fn commit_open_tree_or_toggle_dir(&mut self, output: &mut CommandOutput) {
        let update = self.reduce_revisions(RevisionAction::OpenCommitTree);
        self.apply_revision_update(update, output);
    }

    pub fn stash_open_tree_or_toggle_dir(&mut self, output: &mut CommandOutput) {
        let update = self.reduce_revisions(RevisionAction::OpenStashTree);
        self.apply_revision_update(update, output);
    }

    pub fn commit_close_tree(&mut self, output: &mut CommandOutput) {
        let update = self.reduce_revisions(RevisionAction::CloseCommitTree);
        self.apply_revision_update(update, output);
    }

    pub fn stash_close_tree(&mut self, output: &mut CommandOutput) {
        let update = self.reduce_revisions(RevisionAction::CloseStashTree);
        self.apply_revision_update(update, output);
    }

    pub fn recompute_commit_highlight(&mut self) {
        self.reduce_revisions(RevisionAction::RecomputeHighlight);
    }

    /// Task generation acceptance happens before feature source acceptance.
    pub(super) fn apply_revision_payload(&mut self, payload: &BackgroundPayload) -> bool {
        match payload {
            BackgroundPayload::Commits(items) => debug!(
                event = "commits_load_finished",
                count = items.len(),
                requested_limit = self.revisions.commits_requested_limit(),
                "commits load finished"
            ),
            BackgroundPayload::CommitsFast(items) => debug!(
                event = "commits_load_finished",
                mode = "fast",
                count = items.len(),
                requested_limit = self.revisions.commits_requested_limit(),
                "commits fast load finished"
            ),
            _ => {}
        }
        let (action, source) = match payload {
            BackgroundPayload::Branches(items) => (
                RevisionAction::ReplaceBranches(items.clone()),
                Some(DiffRefreshSource::Branches),
            ),
            BackgroundPayload::Stashes(items) => (
                RevisionAction::ReplaceStashes(items.clone()),
                Some(DiffRefreshSource::Stashes),
            ),
            BackgroundPayload::Commits(items) => (
                RevisionAction::ReplaceCommits {
                    items: items.clone(),
                    fast: false,
                },
                Some(DiffRefreshSource::Commits),
            ),
            BackgroundPayload::CommitsFast(items) => (
                RevisionAction::ReplaceCommits {
                    items: items.clone(),
                    fast: true,
                },
                Some(DiffRefreshSource::Commits),
            ),
            BackgroundPayload::BranchCommits { branch, items } => (
                RevisionAction::BranchCommitsLoaded {
                    branch: branch.clone(),
                    items: items.clone(),
                },
                Some(DiffRefreshSource::Commits),
            ),
            BackgroundPayload::CommitFiles { oid, files } => (
                RevisionAction::CommitFilesLoaded {
                    oid: oid.clone(),
                    files: files.clone(),
                },
                None,
            ),
            BackgroundPayload::StashFiles { index, files } => (
                RevisionAction::StashFilesLoaded {
                    index: *index,
                    files: files.clone(),
                },
                None,
            ),
            _ => unreachable!("only revision completions reach the revision coordinator"),
        };
        let update = self.reduce_revisions(action);
        if !update.accepted {
            return false;
        }
        if let Some(source) = source {
            self.diff.source_refreshed(source);
        }
        self.ui.dirty.mark_all();
        source.is_none_or(|source| self.should_schedule_diff_for_refresh(source))
    }

    pub(super) fn maybe_schedule_commits_extend(&mut self) {
        let loading = self.has_background_task(&TaskKey::Commits);
        let previous_limit = self.revisions.commits_requested_limit();
        self.reduce_revisions(RevisionAction::ExtendCommits { loading });
        if self.revisions.commits_requested_limit() != previous_limit {
            let state = &self.revisions.view().commits;
            debug!(
                event = "commits_extend_requested",
                selected = state.panel.selected(),
                current = state.items.len(),
                next_limit = self.revisions.commits_requested_limit(),
                "scheduled commits extension"
            );
        }
    }

    pub fn ensure_commits_loaded_for_active_panel(&mut self, output: &mut CommandOutput) {
        if self.ui.active_panel == SidePanel::Commits && self.revisions.view().commits.dirty {
            self.start_commits_load(self.revisions.commits_requested_limit(), output);
        }
    }
}
