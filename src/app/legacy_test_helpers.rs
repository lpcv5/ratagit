//! Synchronous compatibility helpers for pre-existing unit tests.
//! Production reducers cannot access these because they only receive AppState.
use super::revision_tree;
use super::{App, RefreshKind};
use color_eyre::Result;
use std::path::PathBuf;

impl App {
    fn apply_refresh(&mut self, kind: RefreshKind) -> Result<()> {
        let status = self.runtime.repo.status()?;
        self.apply_status_refresh(status);

        if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full) {
            self.state.revisions.branches.items =
                self.runtime.repo.branches().unwrap_or_default().into();
            self.state.revisions.stash.items =
                self.runtime.repo.stashes().unwrap_or_default().into();
            if self.state.revisions.stash.tree_mode.active {
                if let Some(index) = self.state.revisions.stash.tree_mode.selected_source {
                    if self
                        .state
                        .revisions
                        .stash
                        .items
                        .iter()
                        .any(|s| s.index == index)
                    {
                        self.state.revisions.stash.tree_mode.files = self
                            .runtime
                            .repo
                            .stash_files(index)
                            .unwrap_or_default()
                            .into();
                        let stash = &mut self.state.revisions.stash;
                        revision_tree::rebuild_tree_nodes(
                            &stash.tree_mode.files,
                            &stash.tree_mode.expanded_dirs,
                            &mut stash.tree_mode.nodes,
                            &mut stash.panel,
                        );
                    } else {
                        self.stash_close_tree();
                    }
                } else {
                    self.stash_close_tree();
                }
            }
        }

        if matches!(kind, RefreshKind::Full) {
            self.reload_commits_now();
        }

        Ok(())
    }

    pub fn flush_pending_refresh(&mut self) -> Result<bool> {
        let Some(kind) = self.state.refresh.pending_refresh.take() else {
            return Ok(false);
        };
        self.apply_refresh(kind)?;
        self.schedule_diff_reload();
        self.state.ui.dirty.mark_all();
        Ok(true)
    }

    pub fn stage_file(&mut self, path: PathBuf) -> Result<()> {
        self.runtime.repo.stage(&path)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    pub fn unstage_file(&mut self, path: PathBuf) -> Result<()> {
        self.runtime.repo.unstage(&path)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    pub fn discard_paths(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.runtime.repo.discard_paths(paths)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    pub(super) fn stage_paths_internal(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.runtime.repo.stage_paths(paths)?;
        Ok(())
    }

    pub(super) fn unstage_paths_internal(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.runtime.repo.unstage_paths(paths)?;
        Ok(())
    }

    pub fn commit(&mut self, message: &str) -> Result<String> {
        let oid = self.runtime.repo.commit(message)?;
        self.request_refresh(RefreshKind::Full);
        Ok(oid)
    }

    pub fn create_branch(&mut self, name: &str) -> Result<()> {
        self.runtime.repo.create_branch(name)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        self.runtime.repo.checkout_branch(name)?;
        self.request_refresh(RefreshKind::Full);
        Ok(())
    }

    pub fn delete_branch(&mut self, name: &str) -> Result<()> {
        self.runtime.repo.delete_branch(name)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn stash_apply(&mut self, index: usize) -> Result<()> {
        self.runtime.repo.stash_apply(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn stash_pop(&mut self, index: usize) -> Result<()> {
        self.runtime.repo.stash_pop(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn stash_drop(&mut self, index: usize) -> Result<()> {
        self.runtime.repo.stash_drop(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub(super) fn reload_commits_now(&mut self) {
        self.state.revisions.commits.items = self
            .runtime
            .repo
            .commits(self.state.revisions.commits_requested_limit())
            .unwrap_or_default()
            .into();
        self.state.revisions.commits.dirty = false;
        if self.state.revisions.commits.tree_mode.active {
            if let Some(ref oid) = self.state.revisions.commits.tree_mode.selected_source {
                if self
                    .state
                    .revisions
                    .commits
                    .items
                    .iter()
                    .any(|c| c.oid == *oid)
                {
                    self.state.revisions.commits.tree_mode.files = self
                        .runtime
                        .repo
                        .commit_files(oid)
                        .unwrap_or_default()
                        .into();
                    let commits = &mut self.state.revisions.commits;
                    revision_tree::rebuild_tree_nodes(
                        &commits.tree_mode.files,
                        &commits.tree_mode.expanded_dirs,
                        &mut commits.tree_mode.nodes,
                        &mut commits.panel,
                    );
                } else {
                    self.commit_close_tree();
                }
            } else {
                self.commit_close_tree();
            }
        }
    }
}

impl App {
    pub fn toggle_stage_visual_selection(&mut self) -> Result<(usize, usize)> {
        let selected = self.visual_selected_indices();
        if selected.is_empty() {
            return Ok((0, 0));
        }

        let (stage_paths, unstage_paths) = self.partition_toggle_targets(&selected);
        if !stage_paths.is_empty() {
            self.stage_paths_internal(&stage_paths)?;
        }
        if !unstage_paths.is_empty() {
            self.unstage_paths_internal(&unstage_paths)?;
        }
        self.request_refresh(RefreshKind::StatusOnly);
        Ok((stage_paths.len(), unstage_paths.len()))
    }

    pub fn prepare_commit_from_visual_selection(&mut self) -> Result<usize> {
        let selected = self.visual_selected_indices();
        let targets = self.collect_commit_targets(&selected);
        if targets.is_empty() {
            return Ok(0);
        }

        self.stage_paths_internal(&targets)?;
        self.request_refresh(RefreshKind::StatusOnly);
        self.state
            .reduce_files(super::files::FilesAction::ClearVisual);
        Ok(targets.len())
    }
}

// Direct transition fixtures use an explicit composition-owned output. This
// compatibility adapter is excluded from production; new runtime tests emit
// through ReduceCtx and the dispatcher just like the application.
impl App {
    pub fn request_refresh(&mut self, kind: RefreshKind) {
        self.state.request_refresh(kind, &mut self.initial_output)
    }
    pub fn schedule_diff_reload(&mut self) {
        self.state.schedule_diff_reload(&mut self.initial_output)
    }
    pub fn reload_diff_now(&mut self) {
        self.state.reload_diff_now(&mut self.initial_output)
    }
    pub fn process_background_refresh_tick(&mut self) {
        self.state
            .process_background_refresh_tick(&mut self.initial_output)
    }
    pub fn flush_pending_diff_reload(&mut self) {
        self.state
            .flush_pending_diff_reload(&mut self.initial_output)
    }
    pub fn apply_repository_action(&mut self, action: &crate::flux::action::DomainAction) {
        self.state
            .apply_repository_action(action, &mut self.initial_output)
    }
    pub fn apply_runtime_event(&mut self, event: &crate::flux::action::RuntimeEvent) {
        self.state
            .apply_runtime_event(event, &mut self.initial_output)
    }
    pub fn commit_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        self.state
            .commit_open_tree_or_toggle_dir(&mut self.initial_output)
    }
    pub fn commit_close_tree(&mut self) {
        self.state.commit_close_tree(&mut self.initial_output)
    }
    pub fn stash_close_tree(&mut self) {
        self.state.stash_close_tree(&mut self.initial_output)
    }
    pub fn start_commit_editor_guarded(&mut self) -> bool {
        self.state
            .start_commit_editor_guarded(&mut self.initial_output)
    }
    pub fn take_effects(&mut self) -> Vec<crate::flux::effects::EffectRequest> {
        self.initial_output
            .drain()
            .into_iter()
            .map(|command| {
                let super::Command::Effect(request) = command else {
                    panic!("direct fixture transitions only emit effects")
                };
                request
            })
            .collect()
    }
}
