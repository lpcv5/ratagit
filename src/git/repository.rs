use std::path::PathBuf;
use thiserror::Error;

/// Documentation comment in English.
#[derive(Debug, Clone, Error)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git2(String),

    #[error("Invalid repository state")]
    InvalidState,
}

// Comment in English.
impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError::Git2(err.to_string())
    }
}

/// Documentation comment in English.
#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
}

/// Documentation comment in English.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub status: FileStatus,
}

/// Documentation comment in English.
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    pub unstaged: Vec<FileEntry>,
    pub staged: Vec<FileEntry>,
    pub untracked: Vec<FileEntry>,
}

/// Documentation comment in English.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    Header,
}

/// Documentation comment in English.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

/// Branch info
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
}

/// Commit info for log display
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub short_hash: String,
    pub oid: String,
    pub message: String,
    pub author: String,
    pub time: String,
    pub parent_count: usize,
}

/// Stash entry
#[derive(Debug, Clone)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
}

/// Documentation comment in English.
/// Documentation comment in English.
pub trait GitRepository {
    /// Documentation comment in English.
    fn status(&self) -> Result<GitStatus, GitError>;

    /// Documentation comment in English.
    fn stage(&self, path: &std::path::Path) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn branches(&self) -> Result<Vec<BranchInfo>, GitError>;

    /// Documentation comment in English.
    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError>;

    /// Documentation comment in English.
    fn stashes(&self) -> Result<Vec<StashInfo>, GitError>;

    /// Documentation comment in English.
    fn commit_diff(&self, oid: &str) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn commit(&self, message: &str) -> Result<String, GitError>;

    /// Documentation comment in English.
    fn create_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn checkout_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn delete_branch(&self, name: &str) -> Result<(), GitError>;
}

/// Documentation comment in English.
pub struct Git2Repository {
    repo: git2::Repository,
}

impl Git2Repository {
    /// Documentation comment in English.
    pub fn discover() -> Result<Self, GitError> {
        let repo = git2::Repository::discover(".")?;
        Ok(Self { repo })
    }

    /// Documentation comment in English.
    #[cfg(test)]
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, GitError> {
        let repo = git2::Repository::open(path)?;
        Ok(Self { repo })
    }

    /// Documentation comment in English.
    fn convert_status(status: git2::Status) -> FileStatus {
        if status.is_index_new() || status.is_wt_new() {
            FileStatus::New
        } else if status.is_index_modified() || status.is_wt_modified() {
            FileStatus::Modified
        } else if status.is_index_deleted() || status.is_wt_deleted() {
            FileStatus::Deleted
        } else if status.is_index_renamed() || status.is_wt_renamed() {
            FileStatus::Renamed
        } else if status.is_index_typechange() || status.is_wt_typechange() {
            FileStatus::TypeChange
        } else {
            FileStatus::Modified // default
        }
    }

    fn signature(&self) -> Result<git2::Signature<'_>, GitError> {
        match self.repo.signature() {
            Ok(sig) => Ok(sig),
            Err(_) => {
                Ok(git2::Signature::now("ratagit", "ratagit@localhost")
                    .map_err(GitError::from)?)
            }
        }
    }
}

impl GitRepository for Git2Repository {
    fn status(&self) -> Result<GitStatus, GitError> {
        let mut git_status = GitStatus::default();

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;

        for entry in statuses.iter() {
            let path = PathBuf::from(entry.path().unwrap_or(""));
            let status = entry.status();

            let file_entry = FileEntry {
                path,
                status: Self::convert_status(status),
            };

            // Comment in English.
            if status.is_wt_new() {
                git_status.untracked.push(file_entry);
            } else if status.is_index_new()
                || status.is_index_modified()
                || status.is_index_deleted()
                || status.is_index_renamed()
            {
                git_status.staged.push(file_entry);
            } else {
                git_status.unstaged.push(file_entry);
            }
        }

        Ok(git_status)
    }

    fn stage(&self, path: &std::path::Path) -> Result<(), GitError> {
        self.stage_paths(&[path.to_path_buf()])
    }

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut index = self.repo.index()?;
        let specs: Vec<&str> = paths
            .iter()
            .filter_map(|p| p.to_str())
            .collect();
        index.add_all(specs, git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError> {
        self.unstage_paths(&[path.to_path_buf()])
    }

    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let head = self.repo.head()?.target().ok_or(GitError::InvalidState)?;
        let commit_obj = self.repo.find_object(head, Some(git2::ObjectType::Commit))?;
        let specs: Vec<&str> = paths
            .iter()
            .filter_map(|p| p.to_str())
            .collect();
        self.repo.reset_default(Some(&commit_obj), specs)?;

        Ok(())
    }
    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""));

        let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let head_tree = self.repo.head().ok()
            .and_then(|h| h.peel_to_tree().ok());

        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""));

        let diff = self.repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let workdir = self.repo.workdir().ok_or(GitError::InvalidState)?;
        let full_path = workdir.join(path);
        let content = std::fs::read_to_string(&full_path)
            .unwrap_or_else(|_| String::from("<binary file>"));

        let header = format!("--- /dev/null\n+++ b/{}", path.display());
        let mut lines = vec![DiffLine { kind: DiffLineKind::Header, content: header }];
        for line in content.lines() {
            lines.push(DiffLine { kind: DiffLineKind::Added, content: line.to_string() });
        }
        Ok(lines)
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let head_name = self.repo.head().ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let mut result = Vec::new();
        for branch in self.repo.branches(Some(git2::BranchType::Local))? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                result.push(BranchInfo {
                    is_current: Some(name.to_string()) == head_name,
                    name: name.to_string(),
                });
            }
        }
        Ok(result)
    }

    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut result = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let short_hash = format!("{:.7}", oid);
            let message = commit.summary().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("").to_string();
            let time = {
                let t = commit.time().seconds();
                let dt = chrono::DateTime::from_timestamp(t, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M")
                    .to_string();
                dt
            };
            result.push(CommitInfo {
                short_hash,
                oid: oid.to_string(),
                message,
                author,
                time,
                parent_count: commit.parent_count(),
            });
        }
        Ok(result)
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        let mut result = Vec::new();
        // Walk stash refs: refs/stash is the tip, refs/stash@{N} for older entries
        let mut index = 0usize;
        loop {
            let refname = if index == 0 {
                "refs/stash".to_string()
            } else {
                format!("refs/stash@{{{}}}", index)
            };
            match self.repo.find_reference(&refname) {
                Ok(r) => {
                    let msg = r.peel_to_commit()
                        .ok()
                        .and_then(|c| c.summary().map(|s| s.to_string()))
                        .unwrap_or_else(|| format!("stash@{{{}}}", index));
                    result.push(StashInfo { index, message: msg });
                    index += 1;
                }
                Err(_) => break,
            }
        }
        Ok(result)
    }

    fn commit_diff(&self, oid: &str) -> Result<Vec<DiffLine>, GitError> {
        let oid = git2::Oid::from_str(oid).map_err(|e| GitError::Git2(e.to_string()))?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        Ok(parse_diff(&diff))
    }

    fn commit(&self, message: &str) -> Result<String, GitError> {
        let sig = self.signature()?;
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let commit_id = match self.repo.head() {
            Ok(head) => {
                let parent = head.peel_to_commit()?;
                self.repo
                    .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?
            }
            Err(_) => self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?,
        };

        Ok(commit_id.to_string())
    }

    fn create_branch(&self, name: &str) -> Result<(), GitError> {
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(name, &head, false)?;
        Ok(())
    }

    fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        let branch = self.repo.find_branch(name, git2::BranchType::Local)?;
        let obj = branch.into_reference().peel(git2::ObjectType::Commit)?;
        let mut opts = git2::build::CheckoutBuilder::new();
        opts.safe();
        self.repo.checkout_tree(&obj, Some(&mut opts))?;
        self.repo.set_head(&format!("refs/heads/{}", name))?;
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let mut branch = self.repo.find_branch(name, git2::BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }
}

fn parse_diff(diff: &git2::Diff) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let content = String::from_utf8_lossy(line.content()).trim_end_matches('\n').to_string();
        let kind = match line.origin() {
            '+' => DiffLineKind::Added,
            '-' => DiffLineKind::Removed,
            'H' | 'F' => DiffLineKind::Header,
            _ => DiffLineKind::Context,
        };
        lines.push(DiffLine { kind, content });
        true
    });
    lines
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).expect("write file");
    }

    fn init_repo_with_commit() -> (TempDir, Git2Repository) {
        let dir = TempDir::new().expect("tempdir");
        let repo = git2::Repository::init(dir.path()).expect("init repo");

        let file = dir.path().join("tracked.txt");
        write_file(&file, "v1\n");

        let mut index = repo.index().expect("index");
        index
            .add_path(Path::new("tracked.txt"))
            .expect("add tracked.txt");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");

        let sig = git2::Signature::now("tester", "tester@example.com").expect("signature");
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .expect("initial commit");

        let repo = Git2Repository::open(dir.path()).expect("open git2 repo");
        (dir, repo)
    }

    #[test]
    fn test_discover_repo() {
        // Comment in English.
        let result = Git2Repository::discover();
        assert!(result.is_ok());
    }

    #[test]
    fn test_stage_unstage_roundtrip() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v2\n");

        let status = repo.status().expect("status before stage");
        assert!(status
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));

        repo.stage(&PathBuf::from("tracked.txt")).expect("stage");
        let status = repo.status().expect("status after stage");
        assert!(status
            .staged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));

        repo.unstage(&PathBuf::from("tracked.txt")).expect("unstage");
        let status = repo.status().expect("status after unstage");
        assert!(status
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));
    }

    #[test]
    fn test_commit_happy_path() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("new.txt"), "hello\n");

        repo.stage(&PathBuf::from("new.txt")).expect("stage new");
        let oid = repo.commit("add new").expect("commit");
        assert!(!oid.is_empty());

        let commits = repo.commits(1).expect("commits");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "add new");
        assert_eq!(commits[0].oid, oid);

        let patch = repo.commit_diff(&oid).expect("commit diff");
        assert!(!patch.is_empty());
    }

    #[test]
    fn test_branch_create_checkout_delete() {
        let (_dir, repo) = init_repo_with_commit();
        repo.create_branch("feature/a").expect("create branch");
        assert!(repo
            .branches()
            .expect("branches")
            .iter()
            .any(|b| b.name == "feature/a"));

        repo.checkout_branch("feature/a").expect("checkout branch");
        assert!(repo
            .branches()
            .expect("branches after checkout")
            .iter()
            .any(|b| b.name == "feature/a" && b.is_current));

        repo.checkout_branch("master").or_else(|_| repo.checkout_branch("main")).expect("checkout default branch");
        repo.delete_branch("feature/a").expect("delete branch");
        assert!(!repo
            .branches()
            .expect("branches after delete")
            .iter()
            .any(|b| b.name == "feature/a"));
    }
}
