use anyhow::Result;
use git2::BranchType;

use super::repo::GitRepo;

#[derive(Debug, Clone)]
pub struct BranchEntry {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
}

pub fn get_branches(repo: &GitRepo) -> Result<Vec<BranchEntry>> {
    let mut branches = Vec::new();

    for branch_result in repo.repo.branches(Some(BranchType::Local))? {
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
