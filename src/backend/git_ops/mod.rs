mod branches;
mod commit_files;
mod commits;
mod diff;
mod repo;
mod stash;
mod status;
mod working_tree;

pub use branches::{get_branches, BranchEntry};
pub use commit_files::get_commit_files;
pub use commits::{get_commits, CommitEntry};
pub use diff::get_diff;
pub use repo::GitRepo;
pub use stash::{get_stashes, StashEntry};
pub use status::{get_status_files, StatusEntry};
pub use working_tree::{stage_file, unstage_file};
