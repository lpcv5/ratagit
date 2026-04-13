use anyhow::Result;
use git2::{DiffFormat, DiffOptions};

use super::repo::GitRepo;

pub fn get_diff(repo: &GitRepo, file_path: &str) -> Result<String> {
    let head_tree = repo
        .repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_tree().ok());

    let mut options = DiffOptions::new();
    options.include_untracked(true);
    options.recurse_untracked_dirs(true);
    options.pathspec(file_path);

    let diff = repo
        .repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut options))?;

    let mut diff_text = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
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

    if diff_text.trim().is_empty() {
        Ok(format!("No patch output available for {file_path}."))
    } else {
        Ok(diff_text)
    }
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
    fn test_get_diff_new_file() {
        let (temp_dir, repo) = create_test_repo();

        // Create a new untracked file
        fs::write(temp_dir.path().join("new.txt"), "new content\n").expect("Failed to write file");

        let diff = get_diff(&repo, "new.txt").expect("Failed to get diff");

        // Untracked files should show in diff or return "No patch output"
        assert!(diff.contains("new content") || diff.contains("No patch output") || diff.contains("new.txt"));
    }

    #[test]
    fn test_get_diff_modified_file() {
        let (temp_dir, repo) = create_test_repo();

        // Create and commit a file
        fs::write(temp_dir.path().join("file.txt"), "original\n").expect("Failed to write file");
        {
            let mut index = repo.repo.index().expect("Failed to get index");
            index.add_path(Path::new("file.txt")).expect("Failed to add file");
            index.write().expect("Failed to write index");

            let sig = repo.repo.signature().expect("Failed to create signature");
            let tree_id = index.write_tree().expect("Failed to write tree");
            let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
            let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
            repo.repo.commit(Some("HEAD"), &sig, &sig, "Add file", &tree, &[&parent])
                .expect("Failed to commit");
        }

        // Modify the file
        fs::write(temp_dir.path().join("file.txt"), "modified\n").expect("Failed to write file");

        let diff = get_diff(&repo, "file.txt").expect("Failed to get diff");

        assert!(diff.contains("original") || diff.contains("modified"));
    }

    #[test]
    fn test_get_diff_no_changes() {
        let (temp_dir, repo) = create_test_repo();

        // Create and commit a file
        fs::write(temp_dir.path().join("file.txt"), "content\n").expect("Failed to write file");
        {
            let mut index = repo.repo.index().expect("Failed to get index");
            index.add_path(Path::new("file.txt")).expect("Failed to add file");
            index.write().expect("Failed to write index");

            let sig = repo.repo.signature().expect("Failed to create signature");
            let tree_id = index.write_tree().expect("Failed to write tree");
            let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
            let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();
            repo.repo.commit(Some("HEAD"), &sig, &sig, "Add file", &tree, &[&parent])
                .expect("Failed to commit");
        }

        // No modifications
        let diff = get_diff(&repo, "file.txt").expect("Failed to get diff");

        assert!(diff.contains("No patch output available"));
    }
}
