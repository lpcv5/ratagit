use super::app::App;
use super::app_effects::AppEffects;
use crate::git::GitError;
use std::any::Any;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

impl AppEffects for App {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn process_background_refresh_tick(&mut self) {
        App::process_background_refresh_tick(self);
    }

    fn flush_pending_refresh(&mut self) -> color_eyre::Result<bool> {
        App::flush_pending_refresh(self)
    }

    fn flush_pending_diff_reload(&mut self) {
        App::flush_pending_diff_reload(self);
    }

    fn ensure_commits_loaded_for_active_panel(&mut self) {
        App::ensure_commits_loaded_for_active_panel(self);
    }

    fn reload_diff_now(&mut self) {
        App::reload_diff_now(self);
    }

    fn stage_paths_request(
        &self,
        paths: Vec<PathBuf>,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::stage_paths_request(self, paths)
    }

    fn toggle_stage_visual_selection(&mut self) -> color_eyre::Result<(usize, usize)> {
        App::toggle_stage_visual_selection(self)
    }

    fn prepare_commit_from_visual_selection(&mut self) -> color_eyre::Result<usize> {
        App::prepare_commit_from_visual_selection(self)
    }

    fn fetch_remote_request(&self) -> color_eyre::Result<Receiver<Result<String, GitError>>> {
        App::fetch_remote_request(self)
    }

    fn stage_file_request(
        &self,
        path: PathBuf,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::stage_file_request(self, path)
    }

    fn unstage_file_request(
        &self,
        path: PathBuf,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::unstage_file_request(self, path)
    }

    fn discard_paths_request(
        &self,
        paths: Vec<PathBuf>,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::discard_paths_request(self, paths)
    }

    fn commit_request(
        &self,
        message: String,
    ) -> color_eyre::Result<Receiver<Result<String, GitError>>> {
        App::commit_request(self, message)
    }

    fn create_branch_request(
        &self,
        name: String,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::create_branch_request(self, name)
    }

    fn checkout_branch_request(
        &self,
        name: String,
        auto_stash: bool,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::checkout_branch_request(self, name, auto_stash)
    }

    fn delete_branch_request(
        &self,
        name: String,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::delete_branch_request(self, name)
    }

    fn stash_push_request(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> color_eyre::Result<Receiver<Result<usize, GitError>>> {
        App::stash_push_request(self, paths, message)
    }

    fn stash_apply_request(
        &self,
        index: usize,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::stash_apply_request(self, index)
    }

    fn stash_pop_request(
        &self,
        index: usize,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::stash_pop_request(self, index)
    }

    fn stash_drop_request(
        &self,
        index: usize,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>> {
        App::stash_drop_request(self, index)
    }

    fn git_log_graph_request(
        &self,
        branch: Option<String>,
    ) -> color_eyre::Result<Receiver<Result<Vec<String>, GitError>>> {
        App::git_log_graph_request(self, branch)
    }
}
