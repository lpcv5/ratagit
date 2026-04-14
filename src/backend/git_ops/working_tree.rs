use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;

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

/// Rename file using git mv (preserves history)
pub fn rename_file(repo: &GitRepo, old_path: &str, new_path: &str) -> Result<()> {
    let workdir = repo
        .repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("No workdir"))?;

    // Validate paths are relative and don't contain path traversal
    if new_path.contains("..") || new_path.starts_with('/') || new_path.starts_with('\\') {
        anyhow::bail!("Invalid path: must be relative without path traversal");
    }

    // Check for absolute paths (Windows: C:\, D:\, etc.)
    if new_path.len() >= 2 && new_path.chars().nth(1) == Some(':') {
        anyhow::bail!("Invalid path: absolute paths not allowed");
    }

    let old_full_path = workdir.join(old_path);
    let new_full_path = workdir.join(new_path);

    // Ensure new_full_path is actually under workdir (defense against path manipulation)
    if !new_full_path.starts_with(workdir) {
        anyhow::bail!("Invalid path: target must be within repository");
    }

    // Validate old path exists
    if !old_full_path.exists() {
        anyhow::bail!("File does not exist: {}", old_path);
    }

    // Validate new path doesn't exist
    if new_full_path.exists() {
        anyhow::bail!("Target already exists: {}", new_path);
    }

    // Validate new path parent directory exists
    if let Some(parent) = Path::new(new_path).parent() {
        if !parent.as_os_str().is_empty() {
            let parent_full = workdir.join(parent);
            if !parent_full.exists() {
                anyhow::bail!("Parent directory does not exist: {}", parent.display());
            }
        }
    }

    // Check if file is in index (staged or tracked)
    let mut index = repo.repo.index()?;
    let old_path_obj = Path::new(old_path);

    if let Some(entry) = index.get_path(old_path_obj, 0) {
        // File is in index: check if it's tracked in HEAD or staged-new
        let head_tree = repo.repo.head()?.peel_to_tree().ok();

        if let Some(tree) = head_tree {
            if let Ok(tree_entry) = tree.get_path(old_path_obj) {
                // File exists in HEAD: check for staged/unstaged changes
                let tree_oid = tree_entry.id();
                let index_oid = entry.id;

                // If index differs from HEAD, file has staged changes
                if tree_oid != index_oid {
                    anyhow::bail!("File has staged changes. Commit or unstage before renaming.");
                }

                // Check for unstaged changes by comparing working tree to index
                let workdir_oid = repo.repo.blob_path(&old_full_path)?;
                if workdir_oid != index_oid {
                    anyhow::bail!("File has unstaged changes. Commit or discard before renaming.");
                }

                // File is tracked and clean: use git mv
                index.remove_path(old_path_obj)?;
                fs::rename(&old_full_path, &new_full_path)?;
                index.add_path(Path::new(new_path))?;
                index.write()?;
            } else {
                // File is in index but not in HEAD: staged-new file
                anyhow::bail!(
                    "File is staged but not committed. Commit or unstage before renaming."
                );
            }
        } else {
            // No HEAD (initial commit scenario): file is staged-new
            anyhow::bail!("File is staged but not committed. Commit or unstage before renaming.");
        }
    } else {
        // Untracked file: just rename in filesystem
        fs::rename(&old_full_path, &new_full_path)?;
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

    #[test]
    fn test_rename_file_untracked() {
        let (temp_dir, repo) = create_test_repo();

        // Create untracked file
        let old_path = temp_dir.path().join("old.txt");
        fs::write(&old_path, "content").expect("Failed to write file");

        // Rename the file
        rename_file(&repo, "old.txt", "new.txt").expect("Failed to rename file");

        // Verify old file doesn't exist
        assert!(!old_path.exists());

        // Verify new file exists
        let new_path = temp_dir.path().join("new.txt");
        assert!(new_path.exists());
        assert_eq!(
            fs::read_to_string(&new_path).expect("Failed to read file"),
            "content"
        );
    }

    #[test]
    fn test_rename_file_tracked() {
        let (temp_dir, repo) = create_test_repo();

        // Create and commit a file
        let old_path = temp_dir.path().join("tracked.txt");
        fs::write(&old_path, "content").expect("Failed to write file");
        stage_file(&repo, "tracked.txt").expect("Failed to stage file");

        // Commit the file
        let sig = repo.repo.signature().expect("Failed to create signature");
        let tree_id = {
            let mut index = repo.repo.index().expect("Failed to get index");
            index.write_tree().expect("Failed to write tree")
        };
        let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
        let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
        repo.repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Add tracked.txt",
                &tree,
                &[&parent],
            )
            .expect("Failed to commit");

        // Rename the file
        rename_file(&repo, "tracked.txt", "renamed.txt").expect("Failed to rename file");

        // Verify old file doesn't exist
        assert!(!old_path.exists());

        // Verify new file exists
        let new_path = temp_dir.path().join("renamed.txt");
        assert!(new_path.exists());

        // Verify new file is in index
        let index = repo.repo.index().expect("Failed to get index");
        assert!(index.get_path(Path::new("renamed.txt"), 0).is_some());
        assert!(index.get_path(Path::new("tracked.txt"), 0).is_none());
    }

    #[test]
    fn test_rename_file_target_exists() {
        let (temp_dir, repo) = create_test_repo();

        // Create two files
        fs::write(temp_dir.path().join("file1.txt"), "content1").expect("Failed to write file");
        fs::write(temp_dir.path().join("file2.txt"), "content2").expect("Failed to write file");

        // Try to rename file1 to file2 (should fail)
        let result = rename_file(&repo, "file1.txt", "file2.txt");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Target already exists"));
    }

    #[test]
    fn test_rename_file_source_not_exists() {
        let (_temp_dir, repo) = create_test_repo();

        // Try to rename non-existent file
        let result = rename_file(&repo, "nonexistent.txt", "new.txt");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("File does not exist"));
    }

    #[test]
    fn test_rename_file_path_traversal() {
        let (temp_dir, repo) = create_test_repo();

        // Create a file
        fs::write(temp_dir.path().join("file.txt"), "content").expect("Failed to write file");

        // Try to rename with path traversal
        let result = rename_file(&repo, "file.txt", "../outside.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be relative"));
    }

    #[test]
    fn test_rename_file_absolute_path_unix() {
        let (temp_dir, repo) = create_test_repo();

        // Create a file
        fs::write(temp_dir.path().join("file.txt"), "content").expect("Failed to write file");

        // Try to rename with absolute path
        let result = rename_file(&repo, "file.txt", "/tmp/outside.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be relative"));
    }

    #[test]
    fn test_rename_file_absolute_path_windows() {
        let (temp_dir, repo) = create_test_repo();

        // Create a file
        fs::write(temp_dir.path().join("file.txt"), "content").expect("Failed to write file");

        // Try to rename with Windows absolute path
        let result = rename_file(&repo, "file.txt", "C:\\tmp\\outside.txt");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("absolute paths not allowed"));
    }

    #[test]
    fn test_rename_file_with_unstaged_changes() {
        let (temp_dir, repo) = create_test_repo();

        // Create and commit a file
        let file_path = temp_dir.path().join("tracked.txt");
        fs::write(&file_path, "original").expect("Failed to write file");
        stage_file(&repo, "tracked.txt").expect("Failed to stage file");

        // Commit the file
        let sig = repo.repo.signature().expect("Failed to create signature");
        let tree_id = {
            let mut index = repo.repo.index().expect("Failed to get index");
            index.write_tree().expect("Failed to write tree")
        };
        let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
        let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
        repo.repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Add tracked.txt",
                &tree,
                &[&parent],
            )
            .expect("Failed to commit");

        // Modify the file (unstaged changes)
        fs::write(&file_path, "modified").expect("Failed to write file");

        // Try to rename - should fail
        let result = rename_file(&repo, "tracked.txt", "renamed.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unstaged changes"));
    }

    #[test]
    fn test_rename_file_with_staged_changes() {
        let (temp_dir, repo) = create_test_repo();

        // Create and commit a file
        let file_path = temp_dir.path().join("tracked.txt");
        fs::write(&file_path, "original").expect("Failed to write file");
        stage_file(&repo, "tracked.txt").expect("Failed to stage file");

        // Commit the file
        let sig = repo.repo.signature().expect("Failed to create signature");
        let tree_id = {
            let mut index = repo.repo.index().expect("Failed to get index");
            index.write_tree().expect("Failed to write tree")
        };
        let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
        let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
        repo.repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Add tracked.txt",
                &tree,
                &[&parent],
            )
            .expect("Failed to commit");

        // Modify and stage the file
        fs::write(&file_path, "modified").expect("Failed to write file");
        stage_file(&repo, "tracked.txt").expect("Failed to stage file");

        // Try to rename - should fail
        let result = rename_file(&repo, "tracked.txt", "renamed.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("staged changes"));
    }

    #[test]
    fn test_rename_file_staged_new() {
        let (_temp_dir, repo) = create_test_repo();

        // Create and stage a new file (not committed)
        let file_path = repo.repo.workdir().unwrap().join("new_staged.txt");
        fs::write(&file_path, "content").expect("Failed to write file");
        stage_file(&repo, "new_staged.txt").expect("Failed to stage file");

        // Try to rename - should fail because file is staged but not committed
        let result = rename_file(&repo, "new_staged.txt", "renamed.txt");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("staged but not committed"));
    }
}
