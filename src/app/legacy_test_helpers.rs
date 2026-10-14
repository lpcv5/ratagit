//! Synchronous compatibility helpers for pre-existing unit tests.
//! Production reducers cannot access these because they only receive AppState.
use super::{refresh, revision_tree};
use super::{App, RefreshKind};
use color_eyre::Result;
use std::path::PathBuf;

impl App {
    fn apply_refresh(&mut self, kind: RefreshKind) -> Result<()> {
        self.state.git.status = self.runtime.repo.status()?;
        let new_dirs = refresh::collect_all_dirs(&self.state.git.status);
        for d in new_dirs {
            self.state.ui.files.expanded_dirs.insert(d);
        }
        self.rebuild_tree();

        if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full) {
            self.state.ui.branches.items = self.runtime.repo.branches().unwrap_or_default();
            self.state.ui.stash.items = self.runtime.repo.stashes().unwrap_or_default();
            if self.state.ui.stash.tree_mode.active {
                if let Some(index) = self.state.ui.stash.tree_mode.selected_source {
                    if self.state.ui.stash.items.iter().any(|s| s.index == index) {
                        self.state.ui.stash.tree_mode.files =
                            self.runtime.repo.stash_files(index).unwrap_or_default();
                        revision_tree::rebuild_tree_nodes(
                            &self.state.ui.stash.tree_mode.files,
                            &self.state.ui.stash.tree_mode.expanded_dirs,
                            &mut self.state.ui.stash.tree_mode.nodes,
                            &mut self.state.ui.stash.panel.list_state,
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
        self.state.ui.commits.items = self
            .runtime
            .repo
            .commits(self.state.tasks.commits_requested_limit)
            .unwrap_or_default();
        self.state.ui.commits.dirty = false;
        if self.state.ui.commits.tree_mode.active {
            if let Some(ref oid) = self.state.ui.commits.tree_mode.selected_source {
                if self.state.ui.commits.items.iter().any(|c| c.oid == *oid) {
                    self.state.ui.commits.tree_mode.files =
                        self.runtime.repo.commit_files(oid).unwrap_or_default();
                    revision_tree::rebuild_tree_nodes(
                        &self.state.ui.commits.tree_mode.files,
                        &self.state.ui.commits.tree_mode.expanded_dirs,
                        &mut self.state.ui.commits.tree_mode.nodes,
                        &mut self.state.ui.commits.panel.list_state,
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
        self.state.ui.files.visual_mode = false;
        self.state.ui.files.visual_anchor = None;
        Ok(targets.len())
    }
}
