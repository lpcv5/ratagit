use crate::git::{DiffLine, GitRepository};
use crate::ui::widgets::file_tree::FileTreeNodeStatus;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DiffTarget {
    None,
    Branch {
        name: String,
    },
    File {
        path: PathBuf,
        status: FileTreeNodeStatus,
    },
    Directory {
        path: PathBuf,
    },
    Commit {
        oid: String,
        path: Option<PathBuf>,
    },
    Stash {
        index: usize,
        path: Option<PathBuf>,
    },
}

#[allow(dead_code)]
pub fn load_diff(repo: &dyn GitRepository, target: DiffTarget) -> Vec<DiffLine> {
    match target {
        DiffTarget::None => Vec::new(),
        DiffTarget::Branch { name } => repo.branch_log(&name, 100).unwrap_or_default(),
        DiffTarget::File { path, status } => load_file_diff(repo, &path, &status),
        DiffTarget::Directory { path } => repo.diff_directory(&path).unwrap_or_default(),
        DiffTarget::Commit { oid, path } => repo
            .commit_diff_scoped(&oid, path.as_deref())
            .unwrap_or_default(),
        DiffTarget::Stash { index, path } => {
            repo.stash_diff(index, path.as_deref()).unwrap_or_default()
        }
    }
}

#[allow(dead_code)]
fn load_file_diff(
    repo: &dyn GitRepository,
    path: &std::path::Path,
    status: &FileTreeNodeStatus,
) -> Vec<DiffLine> {
    match status {
        FileTreeNodeStatus::Staged(_) => repo.diff_staged(path).unwrap_or_default(),
        FileTreeNodeStatus::Untracked => repo.diff_untracked(path).unwrap_or_default(),
        FileTreeNodeStatus::Unstaged(_) => repo.diff_unstaged(path).unwrap_or_default(),
        FileTreeNodeStatus::Directory => Vec::new(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{DiffLine, DiffLineKind, GitError, GitRepository};
    use pretty_assertions::assert_eq;
    use std::path::Path;

    struct TestRepo;

    impl GitRepository for TestRepo {
        fn status(&self) -> Result<crate::git::GitStatus, GitError> {
            Ok(crate::git::GitStatus::default())
        }

        fn stage(&self, _: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn stage_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
            Ok(())
        }

        fn unstage(&self, _: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn unstage_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
            Ok(())
        }

        fn discard_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
            Ok(())
        }

        fn diff_unstaged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![diff_line("unstaged")])
        }

        fn diff_staged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![diff_line("staged")])
        }

        fn diff_untracked(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![diff_line("untracked")])
        }

        fn branches(&self) -> Result<Vec<crate::git::BranchInfo>, GitError> {
            Ok(vec![])
        }

        fn create_branch(&self, _: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn checkout_branch(&self, _: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn delete_branch(&self, _: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn commits(&self, _: usize) -> Result<Vec<crate::git::CommitInfo>, GitError> {
            Ok(vec![])
        }

        fn commit_files(&self, _: &str) -> Result<Vec<crate::git::FileEntry>, GitError> {
            Ok(vec![])
        }

        fn commit_diff_scoped(&self, _: &str, _: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![diff_line("commit")])
        }

        fn commit(&self, _: &str) -> Result<String, GitError> {
            Ok(String::new())
        }

        fn stashes(&self) -> Result<Vec<crate::git::StashInfo>, GitError> {
            Ok(vec![])
        }

        fn stash_files(&self, _: usize) -> Result<Vec<crate::git::FileEntry>, GitError> {
            Ok(vec![])
        }

        fn stash_diff(&self, _: usize, _: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![diff_line("stash")])
        }

        fn stash_push_paths(&self, _: &[PathBuf], _: &str) -> Result<usize, GitError> {
            Ok(0)
        }

        fn stash_apply(&self, _: usize) -> Result<(), GitError> {
            Ok(())
        }

        fn stash_pop(&self, _: usize) -> Result<(), GitError> {
            Ok(())
        }

        fn stash_drop(&self, _: usize) -> Result<(), GitError> {
            Ok(())
        }

        fn fetch_default_async(
            &self,
        ) -> Result<std::sync::mpsc::Receiver<Result<String, GitError>>, GitError> {
            let (tx, rx) = std::sync::mpsc::channel();
            drop(tx);
            Ok(rx)
        }
    }

    fn diff_line(content: &str) -> DiffLine {
        DiffLine {
            kind: DiffLineKind::Added,
            content: content.to_string(),
        }
    }

    fn diff_snapshot(lines: Vec<DiffLine>) -> Vec<(DiffLineKind, String)> {
        lines
            .into_iter()
            .map(|line| (line.kind, line.content))
            .collect()
    }

    #[test]
    fn load_diff_none_returns_empty_result() {
        let repo = TestRepo;

        assert_eq!(
            diff_snapshot(load_diff(&repo, DiffTarget::None)),
            Vec::new()
        );
    }

    #[test]
    fn load_diff_file_staged_returns_staged_patch() {
        let repo = TestRepo;

        let diff = load_diff(
            &repo,
            DiffTarget::File {
                path: "foo.txt".into(),
                status: FileTreeNodeStatus::Staged(crate::git::FileStatus::Modified),
            },
        );

        assert_eq!(
            diff_snapshot(diff),
            vec![(DiffLineKind::Added, "staged".to_string())]
        );
    }

    #[test]
    fn load_diff_file_unstaged_returns_unstaged_patch() {
        let repo = TestRepo;

        let diff = load_diff(
            &repo,
            DiffTarget::File {
                path: "foo.txt".into(),
                status: FileTreeNodeStatus::Unstaged(crate::git::FileStatus::Modified),
            },
        );

        assert_eq!(
            diff_snapshot(diff),
            vec![(DiffLineKind::Added, "unstaged".to_string())]
        );
    }

    #[test]
    fn load_diff_file_untracked_returns_untracked_patch() {
        let repo = TestRepo;

        let diff = load_diff(
            &repo,
            DiffTarget::File {
                path: "foo.txt".into(),
                status: FileTreeNodeStatus::Untracked,
            },
        );

        assert_eq!(
            diff_snapshot(diff),
            vec![(DiffLineKind::Added, "untracked".to_string())]
        );
    }

    #[test]
    fn load_diff_file_directory_returns_empty_result() {
        let repo = TestRepo;

        let diff = load_diff(
            &repo,
            DiffTarget::File {
                path: "dir".into(),
                status: FileTreeNodeStatus::Directory,
            },
        );

        assert_eq!(diff_snapshot(diff), Vec::new());
    }

    #[test]
    fn load_diff_commit_scope_returns_commit_patch() {
        let repo = TestRepo;

        let diff = load_diff(
            &repo,
            DiffTarget::Commit {
                oid: "abc123".to_string(),
                path: None,
            },
        );

        assert_eq!(
            diff_snapshot(diff),
            vec![(DiffLineKind::Added, "commit".to_string())]
        );
    }

    #[test]
    fn load_diff_stash_scope_returns_stash_patch() {
        let repo = TestRepo;

        let diff = load_diff(
            &repo,
            DiffTarget::Stash {
                index: 0,
                path: None,
            },
        );

        assert_eq!(
            diff_snapshot(diff),
            vec![(DiffLineKind::Added, "stash".to_string())]
        );
    }
}
