use anyhow::Result;
use std::fs;
use std::io::Write;

use super::repo::GitRepo;

pub fn stage_file(repo: &GitRepo, file_path: &str) -> Result<()> {
    let mut index = repo.repo.index()?;
    index.add_path(file_path.as_ref())?;
    Ok(index.write()?)
}

pub fn unstage_file(repo: &GitRepo, file_path: &str) -> Result<()> {
    // 使用 git reset HEAD <file> 的逻辑
    // 将 index 中的文件恢复到 HEAD 的状态
    let mut index = repo.repo.index()?;

    // Check if HEAD exists (not an unborn branch)
    let head_tree = repo
        .repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .and_then(|commit| commit.tree().ok());

    match head_tree {
        Some(tree) => {
            // Repository has commits, restore from HEAD
            match tree.get_path(file_path.as_ref()) {
                Ok(entry) => {
                    // 文件在 HEAD 中存在，将 index 恢复到 HEAD 的状态
                    index.add(&git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode: entry.filemode() as u32,
                        uid: 0,
                        gid: 0,
                        file_size: 0,
                        id: entry.id(),
                        flags: file_path.len() as u16,
                        flags_extended: 0,
                        path: file_path.as_bytes().to_vec(),
                    })?;
                }
                Err(_) => {
                    // 文件在 HEAD 中不存在（新文件），从 index 中移除
                    index.remove_path(file_path.as_ref())?;
                }
            }
        }
        None => {
            // No HEAD (unborn branch), just remove from index
            index.remove_path(file_path.as_ref())?;
        }
    }

    Ok(index.write()?)
}

pub fn stage_all(repo: &GitRepo) -> Result<()> {
    let mut index = repo.repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    Ok(index.write()?)
}

pub fn discard_files(repo: &GitRepo, paths: &[String]) -> Result<()> {
    let workdir = repo
        .repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No workdir"))?;

    for path in paths {
        let full_path = workdir.join(path);

        // Check if file is tracked by checking if it exists in HEAD
        let is_tracked = repo
            .repo
            .head()
            .ok()
            .and_then(|head| head.peel_to_tree().ok())
            .and_then(|tree| tree.get_path(std::path::Path::new(path)).ok())
            .is_some();

        if is_tracked {
            // Tracked file: use checkout to restore from HEAD
            repo.repo
                .checkout_head(Some(git2::build::CheckoutBuilder::new().path(path).force()))?;
        } else {
            // Untracked or staged-new file: remove from index and delete from working tree
            let mut index = repo.repo.index()?;
            // Remove from index if present (handles staged-new files)
            let _ = index.remove_path(std::path::Path::new(path));
            index.write()?;

            // Delete from working tree
            if full_path.exists() {
                if full_path.is_dir() {
                    std::fs::remove_dir_all(&full_path)?;
                } else {
                    std::fs::remove_file(&full_path)?;
                }
            }
        }
    }
    Ok(())
}

/// Append paths to .gitignore file
pub fn ignore_files(repo: &GitRepo, paths: &[String]) -> Result<()> {
    let workdir = repo
        .repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No workdir"))?;

    let gitignore_path = workdir.join(".gitignore");

    // Read existing .gitignore content
    let mut existing_content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    // Ensure file ends with newline if it has content
    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        existing_content.push('\n');
    }

    // Append new paths
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&gitignore_path)?;

    file.write_all(existing_content.as_bytes())?;
    for path in paths {
        writeln!(file, "{}", path)?;
    }

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

    #[test]
    fn test_stage_file_success() {
        let (temp_dir, repo) = create_test_repo();

        // Create untracked file
        fs::write(temp_dir.path().join("new.txt"), "content").expect("Failed to write file");

        // Stage the file
        stage_file(&repo, "new.txt").expect("Failed to stage file");

        // Verify file is staged
        let statuses = repo.repo.statuses(None).expect("Failed to get statuses");
        let entry = statuses
            .iter()
            .find(|e| e.path() == Some("new.txt"))
            .expect("File not found");
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
        index
            .add_path(Path::new("staged.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        // Unstage the file
        unstage_file(&repo, "staged.txt").expect("Failed to unstage file");

        // Verify file is no longer staged
        let statuses = repo.repo.statuses(None).expect("Failed to get statuses");
        let entry = statuses
            .iter()
            .find(|e| e.path() == Some("staged.txt"))
            .expect("File not found");
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

    #[test]
    fn test_ignore_files_creates_gitignore() {
        let (temp_dir, repo) = create_test_repo();

        // Ignore a file
        let paths = vec!["test.txt".to_string()];
        ignore_files(&repo, &paths).expect("Failed to ignore files");

        // Verify .gitignore was created
        let gitignore_path = temp_dir.path().join(".gitignore");
        assert!(gitignore_path.exists());

        // Verify content
        let content = fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");
        assert_eq!(content, "test.txt\n");
    }

    #[test]
    fn test_ignore_files_appends_to_existing() {
        let (temp_dir, repo) = create_test_repo();

        // Create existing .gitignore
        let gitignore_path = temp_dir.path().join(".gitignore");
        fs::write(&gitignore_path, "existing.txt\n").expect("Failed to write .gitignore");

        // Ignore more files
        let paths = vec!["new1.txt".to_string(), "new2.txt".to_string()];
        ignore_files(&repo, &paths).expect("Failed to ignore files");

        // Verify content
        let content = fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");
        assert_eq!(content, "existing.txt\nnew1.txt\nnew2.txt\n");
    }

    #[test]
    fn test_ignore_files_multiple() {
        let (temp_dir, repo) = create_test_repo();

        // Ignore multiple files at once
        let paths = vec![
            "file1.txt".to_string(),
            "file2.txt".to_string(),
            "dir/file3.txt".to_string(),
        ];
        ignore_files(&repo, &paths).expect("Failed to ignore files");

        // Verify content
        let gitignore_path = temp_dir.path().join(".gitignore");
        let content = fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");
        assert_eq!(content, "file1.txt\nfile2.txt\ndir/file3.txt\n");
    }
}
