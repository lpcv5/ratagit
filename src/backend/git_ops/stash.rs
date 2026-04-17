use anyhow::Result;

use super::repo::GitRepo;

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    #[allow(dead_code)] // Used in tests
    pub id: String,
    pub message: String,
}

pub fn get_stashes(repo: &mut GitRepo) -> Result<Vec<StashEntry>> {
    let mut stashes = Vec::new();

    repo.repo.stash_foreach(|index, message, oid| {
        stashes.push(StashEntry {
            index,
            id: short_oid(&oid.to_string()),
            message: message.to_string(),
        });
        true
    })?;

    Ok(stashes)
}

fn short_oid(oid: &str) -> String {
    oid.chars().take(8).collect()
}

pub fn stash_files(repo: &GitRepo, paths: &[String], message: Option<&str>) -> Result<()> {
    let msg = message.unwrap_or("WIP on files");

    // Use git stash push with pathspec
    // git2 doesn't have direct support for partial stash, so we use command
    use std::process::Command;

    let repo_path = repo.repo.path().parent().unwrap_or(repo.repo.path());
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("stash").arg("push");

    // Add -u to include untracked files
    cmd.arg("-u");

    cmd.arg("-m").arg(msg);

    // Add -- separator once, then all paths
    cmd.arg("--");
    for path in paths {
        cmd.arg(path);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to stash files: {}",
            String::from_utf8_lossy(&output.stderr)
        );
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
    fn test_get_stashes_empty() {
        let (_temp_dir, mut repo) = create_test_repo();
        let stashes = get_stashes(&mut repo).expect("Failed to get stashes");
        assert_eq!(stashes.len(), 0);
    }

    #[test]
    fn test_get_stashes_with_entries() {
        let (temp_dir, mut repo) = create_test_repo();

        // Create a file and commit it
        fs::write(temp_dir.path().join("file.txt"), "original").expect("Failed to write file");
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

        // Modify the file
        fs::write(temp_dir.path().join("file.txt"), "modified").expect("Failed to write file");

        // Create a stash
        let sig = repo.repo.signature().expect("Failed to create signature");
        repo.repo
            .stash_save(&sig, "My stash message", None)
            .expect("Failed to create stash");

        let stashes = get_stashes(&mut repo).expect("Failed to get stashes");

        assert_eq!(stashes.len(), 1);
        assert_eq!(stashes[0].index, 0);
        assert_eq!(stashes[0].id.len(), 8); // short_oid returns 8 chars
        assert!(stashes[0].message.contains("My stash message"));
    }
}
