use anyhow::{Context, Result};
use git2::{BranchType, DiffFormat, DiffOptions, Repository, Sort, StatusOptions, StatusShow};

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    pub fn discover() -> Result<Self> {
        let repo =
            Repository::discover(".").context("Not a git repository (or any parent directory)")?;
        repo.workdir()
            .context("Repository has no working directory")?;

        Ok(Self { repo })
    }

    pub fn get_status_files(&self) -> Result<Vec<StatusEntry>> {
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        options.include_ignored(false);
        options.include_unmodified(false);
        options.show(StatusShow::IndexAndWorkdir);
        options.recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut options))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let Some(path) = entry.path() else {
                continue;
            };

            let status = entry.status();
            entries.push(StatusEntry {
                path: path.to_string(),
                is_staged: status.is_index_new()
                    || status.is_index_modified()
                    || status.is_index_deleted()
                    || status.is_index_renamed()
                    || status.is_index_typechange(),
                is_unstaged: status.is_wt_new()
                    || status.is_wt_modified()
                    || status.is_wt_deleted()
                    || status.is_wt_renamed()
                    || status.is_wt_typechange(),
            });
        }

        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(entries)
    }

    pub fn get_branches(&self) -> Result<Vec<BranchEntry>> {
        let mut branches = Vec::new();

        for branch_result in self.repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch_result?;
            let name = branch.name()?.unwrap_or("(invalid utf-8)").to_string();
            let upstream = branch
                .upstream()
                .ok()
                .and_then(|upstream| upstream.name().ok().flatten().map(str::to_string));

            branches.push(BranchEntry {
                name,
                is_head: branch.is_head(),
                upstream,
            });
        }

        branches.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(branches)
    }

    pub fn get_commits(&self, limit: usize) -> Result<Vec<CommitEntry>> {
        let mut walk = self.repo.revwalk()?;
        walk.set_sorting(Sort::TIME)?;
        walk.push_head()?;

        let mut commits = Vec::new();
        for oid_result in walk.take(limit) {
            let oid = oid_result?;
            let commit = self.repo.find_commit(oid)?;
            let summary = commit.summary().unwrap_or("(no summary)").to_string();
            let body = commit.body().map(str::trim).filter(|body| !body.is_empty());
            let author = commit.author();
            let author_name = author.name().unwrap_or("Unknown");
            let author_email = author.email().unwrap_or("unknown@example.com");
            let id = oid.to_string();

            commits.push(CommitEntry {
                short_id: short_oid(&id),
                id,
                summary,
                body: body.map(str::to_string),
                author: format!("{author_name} <{author_email}>"),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(commits)
    }

    pub fn get_stashes(&mut self) -> Result<Vec<StashEntry>> {
        let mut stashes = Vec::new();

        self.repo.stash_foreach(|index, message, oid| {
            stashes.push(StashEntry {
                index,
                id: short_oid(&oid.to_string()),
                message: message.to_string(),
            });
            true
        })?;

        Ok(stashes)
    }

    pub fn get_diff(&self, file_path: &str) -> Result<String> {
        let head_tree = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.peel_to_tree().ok());

        let mut options = DiffOptions::new();
        options.include_untracked(true);
        options.recurse_untracked_dirs(true);
        options.pathspec(file_path);

        let diff = self
            .repo
            .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut options))?;

        let mut diff_text = String::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = line.origin();
            let content = String::from_utf8_lossy(line.content());

            match origin {
                '+' | '-' | ' ' => {
                    diff_text.push(origin);
                    diff_text.push_str(&content);
                }
                _ => diff_text.push_str(&content),
            }

            true
        })?;

        if diff_text.trim().is_empty() {
            Ok(format!("No patch output available for {file_path}."))
        } else {
            Ok(diff_text)
        }
    }
}

fn short_oid(oid: &str) -> String {
    oid.chars().take(8).collect()
}

#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub is_staged: bool,
    pub is_unstaged: bool,
}

#[derive(Debug, Clone)]
pub struct BranchEntry {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub short_id: String,
    pub id: String,
    pub summary: String,
    pub body: Option<String>,
    pub author: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub id: String,
    pub message: String,
}
