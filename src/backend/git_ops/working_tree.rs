use anyhow::Result;

use super::repo::GitRepo;

pub fn stage_file(repo: &GitRepo, file_path: &str) -> Result<()> {
    let mut index = repo.repo.index()?;
    index.add_path(file_path.as_ref())?;
    Ok(index.write()?)
}

pub fn unstage_file(repo: &GitRepo, file_path: &str) -> Result<()> {
    // 从 index 中移除文件
    let mut index = repo.repo.index()?;
    index.remove_path(file_path.as_ref())?;
    Ok(index.write()?)
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

    #[test]
    fn test_stage_file_success() {
        let (temp_dir, repo) = create_test_repo();

        // Create untracked file
        fs::write(temp_dir.path().join("new.txt"), "content").expect("Failed to write file");

        // Stage the file
        stage_file(&repo, "new.txt").expect("Failed to stage file");

        // Verify file is staged
        let statuses = repo.repo.statuses(None).expect("Failed to get statuses");
        let entry = statuses.iter().find(|e| e.path() == Some("new.txt")).expect("File not found");
        assert!(entry.status().is_index_new());
    }

    #[test]
    fn test_stage_file_invalid_path() {
        let (_temp_dir, repo) = create_test_repo();

        // Try to stage non-existent file
        let result = stage_file(&repo, "nonexistent.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_unstage_file_success() {
        let (temp_dir, repo) = create_test_repo();

        // Create and stage file
        fs::write(temp_dir.path().join("staged.txt"), "content").expect("Failed to write file");
        let mut index = repo.repo.index().expect("Failed to get index");
        index.add_path(Path::new("staged.txt")).expect("Failed to add file");
        index.write().expect("Failed to write index");

        // Unstage the file
        unstage_file(&repo, "staged.txt").expect("Failed to unstage file");

        // Verify file is no longer staged
        let statuses = repo.repo.statuses(None).expect("Failed to get statuses");
        let entry = statuses.iter().find(|e| e.path() == Some("staged.txt")).expect("File not found");
        assert!(!entry.status().is_index_new());
        assert!(entry.status().is_wt_new()); // Should still be untracked
    }

    #[test]
    fn test_unstage_file_not_staged() {
        let (temp_dir, repo) = create_test_repo();

        // Create untracked file (not staged)
        fs::write(temp_dir.path().join("untracked.txt"), "content").expect("Failed to write file");

        // Unstaging an untracked file succeeds (no-op in git2)
        let result = unstage_file(&repo, "untracked.txt");
        assert!(result.is_ok());
    }
}
