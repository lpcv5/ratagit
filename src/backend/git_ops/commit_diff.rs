use anyhow::Result;
use git2::{DiffFormat, DiffOptions, Oid};

use super::repo::GitRepo;

pub fn get_commit_diff(
    repo: &GitRepo,
    commit_id: &str,
    path: &str,
    is_dir: bool,
) -> Result<String> {
    let commit = repo.repo.find_commit(Oid::from_str(commit_id)?)?;
    let commit_tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let mut options = DiffOptions::new();
    let diff = match parent_tree {
        Some(ref parent) => {
            repo.repo
                .diff_tree_to_tree(Some(parent), Some(&commit_tree), Some(&mut options))?
        }
        None => repo
            .repo
            .diff_tree_to_tree(None, Some(&commit_tree), Some(&mut options))?,
    };

    let mut matched = false;
    let mut diff_text = String::new();
    let prefix = if is_dir {
        Some(if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        })
    } else {
        None
    };

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        let old_path = delta
            .old_file()
            .path()
            .and_then(|p| p.to_str())
            .unwrap_or_default();
        let new_path = delta
            .new_file()
            .path()
            .and_then(|p| p.to_str())
            .unwrap_or_default();

        let include = if let Some(prefix) = &prefix {
            path_matches_dir(old_path, path, prefix) || path_matches_dir(new_path, path, prefix)
        } else {
            old_path == path || new_path == path
        };

        if !include {
            return true;
        }

        matched = true;

        let origin = line.origin();
        let content = String::from_utf8_lossy(line.content());
        match origin {
            '+' | '-' | ' ' => {
                diff_text.push(origin);
                diff_text.push_str(&content);
            }
            _ => diff_text.push_str(&content),
        }
        true
    })?;

    if !matched || diff_text.trim().is_empty() {
        let scope = if is_dir { "directory" } else { "file" };
        return Ok(format!(
            "No patch output available for {scope} `{path}` in commit {commit_id}."
        ));
    }

    Ok(diff_text)
}

fn path_matches_dir(candidate: &str, dir: &str, prefix: &str) -> bool {
    candidate == dir || candidate.starts_with(prefix)
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
    fn test_get_commit_diff_file() {
        let (temp_dir, repo) = create_test_repo();

        // Create and commit a file
        fs::write(temp_dir.path().join("test.txt"), "content\n").expect("Failed to write file");

        let commit_id = {
            let mut index = repo.repo.index().expect("Failed to get index");
            index
                .add_path(Path::new("test.txt"))
                .expect("Failed to add file");
            index.write().expect("Failed to write index");

            let sig = repo.repo.signature().expect("Failed to create signature");
            let tree_id = index.write_tree().expect("Failed to write tree");
            let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
            let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
            repo.repo
                .commit(Some("HEAD"), &sig, &sig, "Add test.txt", &tree, &[&parent])
                .expect("Failed to commit")
                .to_string()
        };

        let diff = get_commit_diff(&repo, &commit_id, "test.txt", false)
            .expect("Failed to get commit diff");

        assert!(diff.contains("content") || diff.contains("test.txt"));
    }

    #[test]
    fn test_get_commit_diff_directory() {
        let (temp_dir, repo) = create_test_repo();

        // Create directory with files
        fs::create_dir(temp_dir.path().join("src")).expect("Failed to create dir");
        fs::write(temp_dir.path().join("src/file1.txt"), "content1\n")
            .expect("Failed to write file");
        fs::write(temp_dir.path().join("src/file2.txt"), "content2\n")
            .expect("Failed to write file");

        let commit_id = {
            let mut index = repo.repo.index().expect("Failed to get index");
            index
                .add_path(Path::new("src/file1.txt"))
                .expect("Failed to add file");
            index
                .add_path(Path::new("src/file2.txt"))
                .expect("Failed to add file");
            index.write().expect("Failed to write index");

            let sig = repo.repo.signature().expect("Failed to create signature");
            let tree_id = index.write_tree().expect("Failed to write tree");
            let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
            let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
            repo.repo
                .commit(Some("HEAD"), &sig, &sig, "Add src files", &tree, &[&parent])
                .expect("Failed to commit")
                .to_string()
        };

        let diff =
            get_commit_diff(&repo, &commit_id, "src", true).expect("Failed to get commit diff");

        // Should contain diffs for files in the directory
        assert!(diff.contains("content1") || diff.contains("content2") || diff.contains("src"));
    }
}
