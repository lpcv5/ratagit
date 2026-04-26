use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use ratagit_core::FileEntry;

use crate::{GitError, validate_repo_relative_path};

pub(crate) fn format_untracked_diffs(
    workdir: &Path,
    files: Vec<FileEntry>,
    selected_paths: &[String],
) -> Result<String, GitError> {
    let selected = selected_paths
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    let mut patches = Vec::new();
    for file in files
        .into_iter()
        .filter(|entry| entry.untracked && path_matches_any(&entry.path, &selected))
    {
        if seen.insert(file.path.clone()) {
            patches.push(format_untracked_file_diff(workdir, &file.path)?);
        }
    }
    Ok(patches.join("\n"))
}

fn path_matches_any(path: &str, selected_paths: &[&str]) -> bool {
    selected_paths
        .iter()
        .any(|selected| path == *selected || path_is_under_directory(path, selected))
}

fn path_is_under_directory(path: &str, selected: &str) -> bool {
    let directory = selected.trim_end_matches('/');
    !directory.is_empty()
        && path.len() > directory.len()
        && path.starts_with(directory)
        && path.as_bytes().get(directory.len()) == Some(&b'/')
}

fn format_untracked_file_diff(workdir: &Path, path: &str) -> Result<String, GitError> {
    let relative_path = validate_repo_relative_path(path)?;
    let bytes = fs::read(workdir.join(relative_path))
        .map_err(|err| GitError::new(format!("failed to read untracked file {path}: {err}")))?;
    if bytes.contains(&0) {
        return Ok(format!(
            "diff --git a/{0} b/{0}\nnew file mode 100644\nBinary files /dev/null and b/{0} differ",
            path
        ));
    }

    let content = String::from_utf8_lossy(&bytes);
    let line_count = content.lines().count();
    let hunk_range = if line_count == 0 {
        "+0,0".to_string()
    } else if line_count == 1 {
        "+1".to_string()
    } else {
        format!("+1,{line_count}")
    };
    let mut patch = format!(
        "diff --git a/{0} b/{0}\nnew file mode 100644\n--- /dev/null\n+++ b/{0}\n@@ -0,0 {1} @@\n",
        path, hunk_range
    );

    for line in content.split_inclusive('\n') {
        let clean = trim_line_ending(line);
        patch.push('+');
        patch.push_str(clean);
        patch.push('\n');
    }

    if !content.is_empty() && !content.ends_with('\n') {
        patch.push_str("\\ No newline at end of file\n");
    }

    Ok(patch.trim_end().to_string())
}

fn trim_line_ending(line: &str) -> &str {
    let without_lf = line.strip_suffix('\n').unwrap_or(line);
    without_lf.strip_suffix('\r').unwrap_or(without_lf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_matching_supports_exact_files_and_directories() {
        assert!(path_matches_any("new.txt", &["new.txt"]));
        assert!(path_matches_any("nested/new.txt", &["nested"]));
        assert!(path_matches_any("nested/new.txt", &["nested/"]));
        assert!(!path_matches_any("nested-new.txt", &["nested"]));
    }

    #[test]
    fn trim_line_ending_handles_lf_crlf_and_plain_text() {
        assert_eq!(trim_line_ending("hello\n"), "hello");
        assert_eq!(trim_line_ending("hello\r\n"), "hello");
        assert_eq!(trim_line_ending("hello"), "hello");
    }
}
