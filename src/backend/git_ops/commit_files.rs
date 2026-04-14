use anyhow::Result;
use git2::DiffOptions;

use super::commits::CommitEntry;
use super::repo::GitRepo;
use crate::components::core::GitFileStatus;

/// 获取某个 commit 修改的文件列表及其状态
pub fn get_commit_files(
    repo: &GitRepo,
    commit: &CommitEntry,
) -> Result<Vec<(String, GitFileStatus)>> {
    let commit_obj = repo.repo.find_commit(git2::Oid::from_str(&commit.id)?)?;
    let tree = commit_obj.tree()?;

    // 获取父提交的树
    let parent_tree = if commit_obj.parent_count() > 0 {
        let parent = commit_obj.parent(0)?;
        Some(parent.tree()?)
    } else {
        // 初始提交，比较空树
        None
    };

    let mut options = DiffOptions::new();
    let diff = match parent_tree {
        Some(ref parent) => {
            repo.repo
                .diff_tree_to_tree(Some(parent), Some(&tree), Some(&mut options))?
        }
        None => repo
            .repo
            .diff_tree_to_tree(None, Some(&tree), Some(&mut options))?,
    };

    let mut files = Vec::new();

    diff.foreach(
        &mut |delta, _progress| {
            let path = delta
                .new_file()
                .path()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();

            let status = match delta.status() {
                git2::Delta::Added => GitFileStatus::Added,
                git2::Delta::Deleted => GitFileStatus::Deleted,
                git2::Delta::Modified => GitFileStatus::Modified,
                git2::Delta::Renamed => GitFileStatus::Renamed,
                _ => GitFileStatus::Unmodified,
            };

            files.push((path, status));
            true
        },
        None,
        None,
        None,
    )?;

    // 按路径排序
    files.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(files)
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
    fn test_get_commit_files() {
        let (temp_dir, repo) = create_test_repo();

        // Create and commit files
        fs::write(temp_dir.path().join("file1.txt"), "content1").expect("Failed to write file");
        fs::write(temp_dir.path().join("file2.txt"), "content2").expect("Failed to write file");

        let commit_entry = {
            let mut index = repo.repo.index().expect("Failed to get index");
            index
                .add_path(Path::new("file1.txt"))
                .expect("Failed to add file");
            index
                .add_path(Path::new("file2.txt"))
                .expect("Failed to add file");
            index.write().expect("Failed to write index");

            let sig = repo.repo.signature().expect("Failed to create signature");
            let tree_id = index.write_tree().expect("Failed to write tree");
            let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
            let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
            let oid = repo
                .repo
                .commit(Some("HEAD"), &sig, &sig, "Add files", &tree, &[&parent])
                .expect("Failed to commit");

            CommitEntry {
                short_id: oid.to_string()[..8].to_string(),
                id: oid.to_string(),
                summary: "Add files".to_string(),
                body: None,
                author: "Test User <test@example.com>".to_string(),
                timestamp: 0,
            }
        };

        let files = get_commit_files(&repo, &commit_entry).expect("Failed to get commit files");

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "file1.txt");
        assert!(matches!(files[0].1, GitFileStatus::Added));
        assert_eq!(files[1].0, "file2.txt");
        assert!(matches!(files[1].1, GitFileStatus::Added));
    }

    #[test]
    fn test_get_commit_files_invalid_id() {
        let (_temp_dir, repo) = create_test_repo();

        let invalid_commit = CommitEntry {
            short_id: "invalid0".to_string(),
            id: "invalid0000000000000000000000000000000000000000".to_string(),
            summary: "Invalid".to_string(),
            body: None,
            author: "Test".to_string(),
            timestamp: 0,
        };

        let result = get_commit_files(&repo, &invalid_commit);
        assert!(result.is_err());
    }
}
