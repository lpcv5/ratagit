use anyhow::Result;
use git2::Sort;

use super::repo::GitRepo;

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub short_id: String,
    pub id: String,
    pub summary: String,
    pub body: Option<String>,
    #[allow(dead_code)] // Used in tests
    pub author: String,
    #[allow(dead_code)] // Reserved for future use
    pub timestamp: i64,
}

pub fn get_commits(repo: &GitRepo, limit: usize) -> Result<Vec<CommitEntry>> {
    let mut walk = repo.repo.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push_head()?;
    collect_commits(repo, &mut walk, limit)
}

pub fn get_commits_for_branch(
    repo: &GitRepo,
    branch_name: &str,
    limit: usize,
) -> Result<Vec<CommitEntry>> {
    let branch = repo
        .repo
        .find_branch(branch_name, git2::BranchType::Local)?;
    let commit = branch.get().peel_to_commit()?;
    let mut walk = repo.repo.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push(commit.id())?;
    collect_commits(repo, &mut walk, limit)
}

fn collect_commits(
    repo: &GitRepo,
    walk: &mut git2::Revwalk,
    limit: usize,
) -> Result<Vec<CommitEntry>> {
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

pub fn get_commit_message(repo: &GitRepo, commit_id: &str) -> Result<String> {
    let oid = git2::Oid::from_str(commit_id)?;
    let commit = repo.repo.find_commit(oid)?;
    Ok(commit.message().unwrap_or("").to_string())
}

/// Create a new commit with staged files
pub fn commit(repo: &GitRepo, message: &str) -> Result<()> {
    let mut index = repo.repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.repo.find_tree(tree_id)?;
    let sig = repo.repo.signature()?;

    // Get parent commit (HEAD)
    let parent_commit = match repo.repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(_) => None, // Initial commit
    };

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    repo.repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(())
}

pub fn amend_commit(repo: &GitRepo, message: &str) -> Result<()> {
    let head = repo.repo.head()?;
    let commit = head.peel_to_commit()?;

    // Use commit.amend() to properly amend the commit
    commit.amend(
        Some("HEAD"),
        None, // author - None means keep original
        None, // committer - None means use current signature
        None, // message_encoding - None means UTF-8
        Some(message),
        None, // tree - None means keep original tree
    )?;

    Ok(())
}

pub fn amend_commit_with_files(
    repo: &GitRepo,
    commit_id: &str,
    message: &str,
    paths: &[String],
) -> Result<()> {
    // Stage the specified files
    let mut index = repo.repo.index()?;
    for path in paths {
        index.add_path(std::path::Path::new(path))?;
    }
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.repo.find_tree(tree_id)?;

    // Get the target commit
    let oid = git2::Oid::from_str(commit_id)?;
    let commit = repo.repo.find_commit(oid)?;

    // Check if this is HEAD
    let head = repo.repo.head()?;
    let head_commit = head.peel_to_commit()?;

    if commit.id() == head_commit.id() {
        // Amending HEAD - straightforward
        commit.amend(Some("HEAD"), None, None, None, Some(message), Some(&tree))?;
    } else {
        // Amending a non-HEAD commit: rewrite the target commit and rebase descendants
        amend_non_head_commit(repo, &commit, message, &tree, &head_commit)?;
    }

    Ok(())
}

/// Rewrite a non-HEAD commit with a new tree, then rebase all descendant commits on top.
fn amend_non_head_commit(
    repo: &GitRepo,
    target: &git2::Commit,
    message: &str,
    new_tree: &git2::Tree,
    head_commit: &git2::Commit,
) -> Result<()> {
    // Collect commits from target (exclusive) up to HEAD (inclusive), oldest first
    let mut descendants: Vec<git2::Commit> = Vec::new();
    let mut current = head_commit.clone();
    loop {
        if current.id() == target.id() {
            break;
        }
        descendants.push(current.clone());
        if current.parent_count() == 0 {
            anyhow::bail!("Target commit not found in ancestry of HEAD");
        }
        current = current.parent(0)?;
    }
    descendants.reverse(); // oldest descendant first

    // Create the amended target commit (no ref update yet)
    let amended_oid = target.amend(
        None, // don't update any ref
        None,
        None,
        None,
        Some(message),
        Some(new_tree),
    )?;
    let mut new_parent_oid = amended_oid;

    // Replay each descendant commit on top of the amended commit
    let sig = repo.repo.signature()?;
    for desc in &descendants {
        let new_parent = repo.repo.find_commit(new_parent_oid)?;
        let desc_tree = desc.tree()?;
        new_parent_oid = repo.repo.commit(
            None, // no ref update yet
            &desc.author(),
            &sig,
            desc.message().unwrap_or(""),
            &desc_tree,
            &[&new_parent],
        )?;
    }

    // Update HEAD to point to the new tip
    let new_head = repo.repo.find_object(new_parent_oid, None)?;
    repo.repo.reset(&new_head, git2::ResetType::Soft, None)?;

    Ok(())
}

pub fn reset_hard(repo: &GitRepo, target: &str) -> Result<()> {
    let obj = repo.repo.revparse_single(target)?;
    repo.repo.reset(&obj, git2::ResetType::Hard, None)?;
    Ok(())
}

pub fn reset_mixed(repo: &GitRepo, target: &str) -> Result<()> {
    let obj = repo.repo.revparse_single(target)?;
    repo.repo.reset(&obj, git2::ResetType::Mixed, None)?;
    Ok(())
}

pub fn reset_soft(repo: &GitRepo, target: &str) -> Result<()> {
    let obj = repo.repo.revparse_single(target)?;
    repo.repo.reset(&obj, git2::ResetType::Soft, None)?;
    Ok(())
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
            config
                .set_str("user.name", "Test User")
                .expect("Failed to set user.name");
            config
                .set_str("user.email", "test@example.com")
                .expect("Failed to set user.email");
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
        index
            .add_path(Path::new(&filename))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let sig = repo.repo.signature().expect("Failed to create signature");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
        let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();

        repo.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
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
        repo.repo
            .branch("feature", &commit, false)
            .expect("Failed to create branch");
        repo.repo
            .set_head("refs/heads/feature")
            .expect("Failed to set HEAD");

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

    #[test]
    fn test_amend_non_head_commit() {
        let (temp_dir, repo) = create_test_repo();

        // Create a history: Initial -> A -> B -> C (HEAD)
        add_commit(&repo, &temp_dir, "Commit A");
        add_commit(&repo, &temp_dir, "Commit B");
        add_commit(&repo, &temp_dir, "Commit C");

        let commits = get_commits(&repo, 10).expect("Failed to get commits");
        assert_eq!(commits.len(), 4);

        // Find commits by summary (order is not guaranteed by get_commits)
        let commit_b_id = commits
            .iter()
            .find(|c| c.summary == "Commit B")
            .expect("Commit B not found")
            .id
            .clone();
        let commit_a_id = commits
            .iter()
            .find(|c| c.summary == "Commit A")
            .expect("Commit A not found")
            .id
            .clone();
        let commit_c_id = commits
            .iter()
            .find(|c| c.summary == "Commit C")
            .expect("Commit C not found")
            .id
            .clone();

        // Create a new file to amend into commit B
        let new_file = "amended_file.txt";
        fs::write(temp_dir.path().join(new_file), "amended content").expect("Failed to write file");

        // Amend commit B with the new file
        amend_commit_with_files(
            &repo,
            &commit_b_id,
            "Commit B (amended)",
            &[new_file.to_string()],
        )
        .expect("Failed to amend commit");

        let new_commits = get_commits(&repo, 10).expect("Failed to get commits");
        assert_eq!(new_commits.len(), 4);

        let summaries: Vec<&str> = new_commits.iter().map(|c| c.summary.as_str()).collect();
        assert!(summaries.contains(&"Commit C"));
        assert!(summaries.contains(&"Commit B (amended)"));
        assert!(summaries.contains(&"Commit A"));
        assert!(summaries.contains(&"Initial commit"));
        assert!(!summaries.contains(&"Commit B")); // original B is gone

        // A is unchanged (it's before B in history)
        let new_a = new_commits
            .iter()
            .find(|c| c.summary == "Commit A")
            .unwrap();
        assert_eq!(new_a.id, commit_a_id);

        // B and C have new IDs (history rewritten)
        let new_b = new_commits
            .iter()
            .find(|c| c.summary == "Commit B (amended)")
            .unwrap();
        let new_c = new_commits
            .iter()
            .find(|c| c.summary == "Commit C")
            .unwrap();
        assert_ne!(new_b.id, commit_b_id);
        assert_ne!(new_c.id, commit_c_id);
    }
}
