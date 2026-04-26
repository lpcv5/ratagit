use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use ratagit_core::FileEntry;

use crate::{GitError, validate_repo_relative_path};

const MAX_UNTRACKED_DIFF_FILES: usize = 25;
const MAX_UNTRACKED_DIFF_BYTES: usize = 256 * 1024;

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
    let mut emitted_files = 0usize;
    let mut emitted_bytes = 0usize;
    for file in files
        .into_iter()
        .filter(|entry| entry.untracked && path_matches_any(&entry.path, &selected))
    {
        if !seen.insert(file.path.clone()) {
            continue;
        }
        if emitted_files >= MAX_UNTRACKED_DIFF_FILES {
            patches.push(format!(
                "untracked diff truncated: showing first {MAX_UNTRACKED_DIFF_FILES} files"
            ));
            break;
        }
        let remaining_bytes = MAX_UNTRACKED_DIFF_BYTES.saturating_sub(emitted_bytes);
        if remaining_bytes == 0 {
            patches.push(format!(
                "untracked diff truncated: reached {MAX_UNTRACKED_DIFF_BYTES} byte limit"
            ));
            break;
        }
        let patch = format_untracked_file_diff(workdir, &file.path, remaining_bytes)?;
        emitted_bytes = emitted_bytes.saturating_add(patch.len());
        emitted_files += 1;
        let truncated = patch.contains("untracked diff omitted:");
        patches.push(patch);
        if truncated {
            break;
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

fn format_untracked_file_diff(
    workdir: &Path,
    path: &str,
    max_bytes: usize,
) -> Result<String, GitError> {
    let relative_path = validate_repo_relative_path(path)?;
    let target = workdir.join(relative_path);
    let len = fs::metadata(&target)
        .map_err(|err| GitError::new(format!("failed to stat untracked file {path}: {err}")))?
        .len() as usize;
    if len > max_bytes {
        return Ok(format!(
            "diff --git a/{0} b/{0}\nnew file mode 100644\nuntracked diff omitted: file exceeds remaining {1} byte limit",
            path, max_bytes
        ));
    }
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
    use std::fs::{create_dir_all, remove_dir_all, write};
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn large_untracked_file_diff_is_omitted() {
        let root = std::env::temp_dir().join(format!(
            "ratagit-untracked-diff-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        create_dir_all(&root).expect("temp dir should be creatable");
        write(
            root.join("huge.txt"),
            vec![b'x'; MAX_UNTRACKED_DIFF_BYTES + 1],
        )
        .expect("huge file should be writable");

        let diff = format_untracked_diffs(
            &root,
            vec![FileEntry {
                path: "huge.txt".to_string(),
                staged: false,
                untracked: true,
            }],
            &["huge.txt".to_string()],
        )
        .expect("diff should be generated");

        assert!(diff.contains("untracked diff omitted"));
        let _ = remove_dir_all(root);
    }
}
