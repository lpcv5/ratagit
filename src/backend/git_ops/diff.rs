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
