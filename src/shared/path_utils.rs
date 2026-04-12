use crate::backend::DiffTarget;
use std::collections::HashSet;

/// Normalize a path by removing trailing slashes
pub fn normalize_path(path: &str) -> String {
    path.trim_end_matches('/').to_string()
}

/// Deduplicate diff targets, removing children when parent directory is selected
/// Takes a slice and returns a filtered vector
pub fn dedupe_targets_parent_first(targets: &[DiffTarget]) -> Vec<DiffTarget> {
    let selected_dirs: Vec<String> = targets
        .iter()
        .filter(|target| target.is_dir)
        .map(|target| normalize_path(&target.path))
        .collect();
    let mut seen = HashSet::new();

    targets
        .iter()
        .filter(|target| {
            let normalized = normalize_path(&target.path);
            let parent_selected = selected_dirs.iter().any(|dir| {
                dir != &normalized
                    && (normalized == *dir || normalized.starts_with(format!("{dir}/").as_str()))
            });
            if parent_selected {
                return false;
            }

            let unique = format!("{}:{}", normalized, target.is_dir);
            seen.insert(unique)
        })
        .cloned()
        .collect()
}

/// Generate a label for a diff target (used in UI)
pub fn diff_target_label(target: &DiffTarget) -> String {
    if target.is_dir {
        format!("{}/ (directory)", target.path)
    } else {
        target.path.clone()
    }
}

/// Generate a pathspec for git diff operations
pub fn diff_target_pathspec(target: &DiffTarget) -> String {
    if target.is_dir && !target.path.ends_with('/') {
        format!("{}/", target.path)
    } else {
        target.path.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("foo/bar"), "foo/bar");
        assert_eq!(normalize_path("foo/bar/"), "foo/bar");
        assert_eq!(normalize_path("foo/bar///"), "foo/bar");
    }

    #[test]
    fn test_dedupe_targets_parent_first() {
        let targets = vec![
            DiffTarget {
                path: "src".to_string(),
                is_dir: true,
            },
            DiffTarget {
                path: "src/main.rs".to_string(),
                is_dir: false,
            },
            DiffTarget {
                path: "src/lib.rs".to_string(),
                is_dir: false,
            },
        ];

        let result = dedupe_targets_parent_first(&targets);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src");
    }

    #[test]
    fn test_diff_target_label() {
        let dir_target = DiffTarget {
            path: "src".to_string(),
            is_dir: true,
        };
        assert_eq!(diff_target_label(&dir_target), "src/ (directory)");

        let file_target = DiffTarget {
            path: "main.rs".to_string(),
            is_dir: false,
        };
        assert_eq!(diff_target_label(&file_target), "main.rs");
    }

    #[test]
    fn test_diff_target_pathspec() {
        let dir_target = DiffTarget {
            path: "src".to_string(),
            is_dir: true,
        };
        assert_eq!(diff_target_pathspec(&dir_target), "src/");

        let file_target = DiffTarget {
            path: "main.rs".to_string(),
            is_dir: false,
        };
        assert_eq!(diff_target_pathspec(&file_target), "main.rs");
    }
}
