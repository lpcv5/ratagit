use anyhow::Result;
use git2::Sort;

use super::repo::GitRepo;

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub short_id: String,
    pub id: String,
    pub summary: String,
    pub body: Option<String>,
    pub author: String,
    pub timestamp: i64,
}

pub fn get_commits(repo: &GitRepo, limit: usize) -> Result<Vec<CommitEntry>> {
    let mut walk = repo.repo.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push_head()?;
    collect_commits(repo, &mut walk, limit)
}

pub fn get_commits_for_branch(repo: &GitRepo, branch_name: &str, limit: usize) -> Result<Vec<CommitEntry>> {
    let branch = repo.repo.find_branch(branch_name, git2::BranchType::Local)?;
    let commit = branch.get().peel_to_commit()?;
    let mut walk = repo.repo.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push(commit.id())?;
    collect_commits(repo, &mut walk, limit)
}

fn collect_commits(repo: &GitRepo, walk: &mut git2::Revwalk, limit: usize) -> Result<Vec<CommitEntry>> {
    let mut commits = Vec::new();
    for oid_result in walk.take(limit) {
        let oid = oid_result?;
        let commit = repo.repo.find_commit(oid)?;
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

fn short_oid(oid: &str) -> String {
    oid.chars().take(8).collect()
}
