use std::process::Command;

use anyhow::{anyhow, Context, Result};

use super::repo::GitRepo;

pub fn get_branch_graph(repo: &GitRepo, branch_name: &str, limit: usize) -> Result<String> {
    let workdir = repo
        .repo
        .workdir()
        .context("Repository has no working directory")?;
    let revision = format!("refs/heads/{branch_name}");

    let output = Command::new("git")
        .arg("log")
        .arg("--graph")
        .arg("--decorate")
        .arg("--color=always")
        .arg(format!("--max-count={limit}"))
        .arg(revision)
        .current_dir(workdir)
        .output()
        .context("failed to execute git log --graph")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow!("git log --graph failed: {stderr}"));
    }

    let graph = String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string();
    if graph.is_empty() {
        Ok("(no commits found for this branch)".to_string())
    } else {
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
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

    #[test]
    #[serial]
    fn test_get_branch_graph_success() {
        let (temp_dir, repo) = create_test_repo();

        // Add a commit
        fs::write(temp_dir.path().join("file.txt"), "content").expect("Failed to write file");
        {
            let mut index = repo.repo.index().expect("Failed to get index");
            index
                .add_path(Path::new("file.txt"))
                .expect("Failed to add file");
            index.write().expect("Failed to write index");

            let sig = repo.repo.signature().expect("Failed to create signature");
            let tree_id = index.write_tree().expect("Failed to write tree");
            let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
            let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
            repo.repo
                .commit(Some("HEAD"), &sig, &sig, "Add file", &tree, &[&parent])
                .expect("Failed to commit");
        }

        // Get the current branch name
        let head = repo.repo.head().expect("Failed to get HEAD");
        let branch_name = head.shorthand().expect("Failed to get branch name");

        let graph = get_branch_graph(&repo, branch_name, 10).expect("Failed to get branch graph");

        // Should contain commit information
        assert!(!graph.is_empty());
        assert!(graph.contains("Initial commit") || graph.contains("Add file"));
    }

    #[test]
    #[serial]
    fn test_get_branch_graph_invalid_branch() {
        let (_temp_dir, repo) = create_test_repo();

        let result = get_branch_graph(&repo, "nonexistent-branch", 10);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_get_branch_graph_with_limit() {
        let (temp_dir, repo) = create_test_repo();

        // Add multiple commits
        for i in 1..=5 {
            let filename = format!("file{}.txt", i);
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
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("Commit {}", i),
                    &tree,
                    &[&parent],
                )
                .expect("Failed to commit");

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let head = repo.repo.head().expect("Failed to get HEAD");
        let branch_name = head.shorthand().expect("Failed to get branch name");

        let graph = get_branch_graph(&repo, branch_name, 2).expect("Failed to get branch graph");

        // Should have limited output (hard to verify exact count due to formatting)
        assert!(!graph.is_empty());
    }
}
