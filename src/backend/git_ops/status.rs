use anyhow::Result;
use git2::{StatusOptions, StatusShow};

use super::repo::GitRepo;

#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub is_staged: bool,
    pub is_unstaged: bool,
    pub is_untracked: bool,
}

pub fn get_status_files(repo: &GitRepo) -> Result<Vec<StatusEntry>> {
    let mut options = StatusOptions::new();
    options.include_untracked(true);
    options.include_ignored(false);
    options.include_unmodified(false);
    options.show(StatusShow::IndexAndWorkdir);
    options.recurse_untracked_dirs(true); // 递归列出未跟踪目录中的文件

    let statuses = repo.repo.statuses(Some(&mut options))?;
    let mut entries = Vec::new();

    for entry in statuses.iter() {
        let Some(path) = entry.path() else {
            continue;
        };

        let status = entry.status();

        let is_untracked = status.is_wt_new();
        let is_unstaged = status.is_wt_new()
            || status.is_wt_modified()
            || status.is_wt_deleted()
            || status.is_wt_renamed()
            || status.is_wt_typechange();
        let is_staged = status.is_index_new()
            || status.is_index_modified()
            || status.is_index_deleted()
            || status.is_index_renamed()
            || status.is_index_typechange();

        entries.push(StatusEntry {
            path: path.to_string(),
            is_staged,
            is_unstaged,
            is_untracked,
        });
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
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

        // Configure user
        {
            let mut config = repo.config().expect("Failed to get config");
            config
                .set_str("user.name", "Test User")
                .expect("Failed to set user.name");
            config
                .set_str("user.email", "test@example.com")
                .expect("Failed to set user.email");
        }

        // Create initial commit
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
    fn test_get_status_files_empty() {
        let (_temp_dir, repo) = create_test_repo();
        let entries = get_status_files(&repo).expect("Failed to get status");
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_get_status_files_untracked() {
        let (temp_dir, repo) = create_test_repo();

        // Create untracked file
        fs::write(temp_dir.path().join("new.txt"), "content").expect("Failed to write file");

        let entries = get_status_files(&repo).expect("Failed to get status");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "new.txt");
        assert!(entries[0].is_untracked);
        assert!(entries[0].is_unstaged);
        assert!(!entries[0].is_staged);
    }

    #[test]
    fn test_get_status_files_staged() {
        let (temp_dir, repo) = create_test_repo();

        // Create and stage file
        fs::write(temp_dir.path().join("staged.txt"), "content").expect("Failed to write file");
        let mut index = repo.repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("staged.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let entries = get_status_files(&repo).expect("Failed to get status");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "staged.txt");
        assert!(entries[0].is_staged);
        assert!(!entries[0].is_untracked);
    }

    #[test]
    fn test_get_status_files_modified() {
        let (temp_dir, repo) = create_test_repo();

        // Create, stage, and commit file
        fs::write(temp_dir.path().join("file.txt"), "original").expect("Failed to write file");
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

        // Modify file
        fs::write(temp_dir.path().join("file.txt"), "modified").expect("Failed to write file");

        let entries = get_status_files(&repo).expect("Failed to get status");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "file.txt");
        assert!(entries[0].is_unstaged);
        assert!(!entries[0].is_staged);
        assert!(!entries[0].is_untracked);
    }
}
