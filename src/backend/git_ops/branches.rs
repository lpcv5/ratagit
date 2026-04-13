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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_get_branches_single() {
        let (_temp_dir, repo) = create_test_repo();
        let branches = get_branches(&repo).expect("Failed to get branches");

        assert_eq!(branches.len(), 1);
        // Git creates either "main" or "master" depending on config
        assert!(branches[0].name == "main" || branches[0].name == "master");
        assert!(branches[0].is_head);
        assert_eq!(branches[0].upstream, None);
    }

    #[test]
    fn test_get_branches_multiple() {
        let (_temp_dir, repo) = create_test_repo();

        // Create additional branches
        let head = repo.repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        repo.repo.branch("feature-a", &commit, false).expect("Failed to create branch");
        repo.repo.branch("feature-b", &commit, false).expect("Failed to create branch");

        let branches = get_branches(&repo).expect("Failed to get branches");

        assert_eq!(branches.len(), 3);
        // Should be sorted alphabetically
        assert_eq!(branches[0].name, "feature-a");
        assert_eq!(branches[1].name, "feature-b");
    }

    #[test]
    fn test_get_branches_detects_head() {
        let (_temp_dir, repo) = create_test_repo();

        // Create and checkout a new branch
        let head = repo.repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        let branch = repo.repo.branch("new-branch", &commit, false).expect("Failed to create branch");

        repo.repo.set_head(branch.get().name().unwrap()).expect("Failed to set HEAD");

        let branches = get_branches(&repo).expect("Failed to get branches");

        // Find the new-branch
        let new_branch = branches.iter().find(|b| b.name == "new-branch").expect("Branch not found");
        assert!(new_branch.is_head);

        // Original branch should not be HEAD
        let original = branches.iter().find(|b| b.name != "new-branch").expect("Original branch not found");
        assert!(!original.is_head);
    }
}
