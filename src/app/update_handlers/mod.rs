mod branch;
mod commit;
mod files;
mod navigation;
mod quit;
mod revision;
mod search;
mod staging;
mod stash;

pub(crate) use branch::handle_branch_message;
pub(crate) use commit::handle_commit_message;
pub(crate) use files::handle_files_message;
pub(crate) use navigation::handle_navigation_message;
pub(crate) use quit::handle_quit;
pub(crate) use revision::handle_revision_message;
pub(crate) use search::handle_search_message;
pub(crate) use staging::handle_staging_message;
pub(crate) use stash::handle_stash_message;
