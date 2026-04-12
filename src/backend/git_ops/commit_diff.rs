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
