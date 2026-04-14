mod branch_graph;
mod branches;
mod commit_diff;
mod commit_files;
mod commits;
mod diff;
mod repo;
mod stash;
mod status;
mod working_tree;

pub use branch_graph::get_branch_graph;
pub use branches::{get_branches, BranchEntry};
pub use commit_diff::get_commit_diff;
pub use commit_files::get_commit_files;
pub use commits::{
    amend_commit, amend_commit_with_files, commit, get_commit_message, get_commits,
    get_commits_for_branch, reset_hard, reset_mixed, reset_soft, CommitEntry,
};
pub use diff::get_diff;
pub use repo::GitRepo;
pub use stash::{get_stashes, stash_files, StashEntry};
pub use status::{get_status_files, StatusEntry};
pub use working_tree::{
    discard_files, ignore_files, rename_file, stage_all, stage_file, unstage_file,
};
