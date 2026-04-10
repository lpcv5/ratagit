use crate::flux::stores::StateAccess;
use crate::git::GitError;
use std::any::Any;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

/// Trait abstracting all App operations needed by the effect runner.
///
/// This decouples `effects.rs` from the concrete `App` type, enabling
/// the effect runtime to work through a `dyn AppEffects` reference
/// rather than holding `Rc<Mutex<App>>` directly.
///
/// All Git repository request methods return `Receiver<Result<T, GitError>>` —
/// these are spawned in background threads by the concrete implementation.
pub trait AppEffects: StateAccess {
    // --- Downcasting support for tests ---

    #[allow(dead_code)]
    fn as_any(&self) -> &dyn Any;
    #[allow(dead_code)]
    fn as_any_mut(&mut self) -> &mut dyn Any;
    // --- State queries used to make decisions in effects ---

    // --- Orchestration methods that combine multiple state changes ---

    fn process_background_refresh_tick(&mut self);
    fn flush_pending_refresh(&mut self) -> color_eyre::Result<bool>;
    fn flush_pending_diff_reload(&mut self);
    fn ensure_commits_loaded_for_active_panel(&mut self);
    fn reload_diff_now(&mut self);
    fn open_selected_branch_commits(&mut self, limit: usize) -> color_eyre::Result<()>;
    fn stage_paths_request(
        &self,
        paths: Vec<PathBuf>,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn stash_open_tree_or_toggle_dir(&mut self) -> color_eyre::Result<()>;
    fn commit_open_tree_or_toggle_dir(&mut self) -> color_eyre::Result<()>;
    fn toggle_stage_visual_selection(&mut self) -> color_eyre::Result<(usize, usize)>;
    fn prepare_commit_from_visual_selection(&mut self) -> color_eyre::Result<usize>;

    // --- Repository async request launchers ---

    fn fetch_remote_request(&self) -> color_eyre::Result<Receiver<Result<String, GitError>>>;
    fn stage_file_request(
        &self,
        path: PathBuf,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn unstage_file_request(
        &self,
        path: PathBuf,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn discard_paths_request(
        &self,
        paths: Vec<PathBuf>,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn commit_request(
        &self,
        message: String,
    ) -> color_eyre::Result<Receiver<Result<String, GitError>>>;
    fn create_branch_request(
        &self,
        name: String,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn checkout_branch_request(
        &self,
        name: String,
        auto_stash: bool,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn delete_branch_request(
        &self,
        name: String,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn stash_push_request(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> color_eyre::Result<Receiver<Result<usize, GitError>>>;
    fn stash_apply_request(
        &self,
        index: usize,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn stash_pop_request(&self, index: usize)
        -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn stash_drop_request(
        &self,
        index: usize,
    ) -> color_eyre::Result<Receiver<Result<(), GitError>>>;
    fn git_log_graph_request(
        &self,
        branch: Option<String>,
    ) -> color_eyre::Result<Receiver<Result<Vec<String>, GitError>>>;
}
