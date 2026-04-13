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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn create_test_repo() -> (tempfile::TempDir, GitRepo) {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path();

        let repo = git2::Repository::init(path).expect("Failed to init repo");

        {
            let mut config = repo.config().expect("Failed to get config");
            config.set_str("user.name", "Test User").expect("Failed to set user.name");
            config.set_str("user.email", "test@example.com").expect("Failed to set user.email");
        }

        {
            let sig = repo.signature().expect("Failed to create signature");
            let tree_id = {
                let mut index = repo.index().expect("Failed to get index");
                index.write_tree().expect("Failed to write tree")
            };
            let tree = repo.find_tree(tree_id).expect("Failed to find tree");
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .expect("Failed to create initial commit");
        }

        (temp_dir, GitRepo { repo })
    }

    fn add_commit(repo: &GitRepo, temp_dir: &tempfile::TempDir, message: &str) {
        let filename = format!("file_{}.txt", message.replace(' ', "_"));
        fs::write(temp_dir.path().join(&filename), "content").expect("Failed to write file");

        let mut index = repo.repo.index().expect("Failed to get index");
        index.add_path(Path::new(&filename)).expect("Failed to add file");
        index.write().expect("Failed to write index");

        let sig = repo.repo.signature().expect("Failed to create signature");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
        let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();

        repo.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .expect("Failed to commit");

        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    #[test]
    fn test_get_commits_single() {
        let (_temp_dir, repo) = create_test_repo();
        let commits = get_commits(&repo, 10).expect("Failed to get commits");

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "Initial commit");
        assert_eq!(commits[0].author, "Test User <test@example.com>");
        assert_eq!(commits[0].short_id.len(), 8);
    }

    #[test]
    fn test_get_commits_multiple() {
        let (temp_dir, repo) = create_test_repo();

        add_commit(&repo, &temp_dir, "Second commit");
        add_commit(&repo, &temp_dir, "Third commit");

        let commits = get_commits(&repo, 10).expect("Failed to get commits");

        assert_eq!(commits.len(), 3);
        // Verify all commits are present
        let summaries: Vec<&str> = commits.iter().map(|c| c.summary.as_str()).collect();
        assert!(summaries.contains(&"Initial commit"));
        assert!(summaries.contains(&"Second commit"));
        assert!(summaries.contains(&"Third commit"));
        // Most recent should be first (Third commit)
        assert_eq!(commits[0].summary, "Third commit");
    }

    #[test]
    fn test_get_commits_with_limit() {
        let (temp_dir, repo) = create_test_repo();

        add_commit(&repo, &temp_dir, "Second commit");
        add_commit(&repo, &temp_dir, "Third commit");
        add_commit(&repo, &temp_dir, "Fourth commit");

        let commits = get_commits(&repo, 2).expect("Failed to get commits");

        // Should only get the 2 most recent commits
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "Fourth commit");
    }

    #[test]
    fn test_get_commits_for_branch() {
        let (temp_dir, repo) = create_test_repo();

        add_commit(&repo, &temp_dir, "Main commit");

        // Create a branch and add commits to it
        let head = repo.repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        repo.repo.branch("feature", &commit, false).expect("Failed to create branch");
        repo.repo.set_head("refs/heads/feature").expect("Failed to set HEAD");

        add_commit(&repo, &temp_dir, "Feature commit");

        let commits = get_commits_for_branch(&repo, "feature", 10).expect("Failed to get commits");

        assert_eq!(commits.len(), 3);
        // Most recent commit should be first
        assert_eq!(commits[0].summary, "Feature commit");
        // Verify all commits are present
        let summaries: Vec<&str> = commits.iter().map(|c| c.summary.as_str()).collect();
        assert!(summaries.contains(&"Feature commit"));
        assert!(summaries.contains(&"Main commit"));
        assert!(summaries.contains(&"Initial commit"));
    }
}
