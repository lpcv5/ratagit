use anyhow::Result;
use git2::{StatusOptions, StatusShow};

use super::repo::GitRepo;

#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub is_staged: bool,
    pub is_unstaged: bool,
    pub is_untracked: bool,
}

pub fn get_status_files(repo: &GitRepo) -> Result<Vec<StatusEntry>> {
    let mut options = StatusOptions::new();
    options.include_untracked(true);
    options.include_ignored(false);
    options.include_unmodified(false);
    options.show(StatusShow::IndexAndWorkdir);

    let statuses = repo.repo.statuses(Some(&mut options))?;
    let mut entries = Vec::new();

    for entry in statuses.iter() {
        let Some(path) = entry.path() else {
            continue;
        };

        let status = entry.status();

        let is_untracked = status.is_wt_new();
        let is_unstaged = status.is_wt_new()
            || status.is_wt_modified()
            || status.is_wt_deleted()
            || status.is_wt_renamed()
            || status.is_wt_typechange();
        let is_staged = status.is_index_new()
            || status.is_index_modified()
            || status.is_index_deleted()
            || status.is_index_renamed()
            || status.is_index_typechange();

        entries.push(StatusEntry {
            path: path.to_string(),
            is_staged,
            is_unstaged,
            is_untracked,
        });
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}
