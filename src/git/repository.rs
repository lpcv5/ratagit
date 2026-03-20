use std::path::PathBuf;
use std::process::Command;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSyncState {
    Main,
    RemoteSynced,
    LocalOnly,
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
    pub sync_state: CommitSyncState,
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
    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError>;

    /// Documentation comment in English.
    fn stashes(&self) -> Result<Vec<StashInfo>, GitError>;

    /// Documentation comment in English.
    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError>;

    /// Documentation comment in English.
    fn stash_diff(&self, index: usize, path: Option<&std::path::Path>) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn stash_push_paths(&self, paths: &[PathBuf], message: &str) -> Result<usize, GitError>;

    /// Documentation comment in English.
    fn stash_apply(&self, index: usize) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn stash_pop(&self, index: usize) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn stash_drop(&self, index: usize) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn commit_diff_scoped(&self, oid: &str, path: Option<&std::path::Path>) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn commit(&self, message: &str) -> Result<String, GitError>;

    /// Documentation comment in English.
    fn create_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn checkout_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn delete_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn fetch_default(&self) -> Result<String, GitError>;
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

    fn repo_root(&self) -> Result<PathBuf, GitError> {
        if let Some(workdir) = self.repo.workdir() {
            return Ok(workdir.to_path_buf());
        }

        self.repo
            .path()
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or(GitError::InvalidState)
    }

    fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.repo_root()?)
            .output()
            .map_err(|e| GitError::Git2(format!("failed to run git {:?}: {}", args, e)))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(GitError::Git2(detail))
    }

    fn run_git_owned(&self, args: &[String]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.repo_root()?)
            .output()
            .map_err(|e| GitError::Git2(format!("failed to run git {:?}: {}", args, e)))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(GitError::Git2(detail))
    }

    fn resolve_ref_oid(&self, refname: &str) -> Option<git2::Oid> {
        self.repo
            .find_reference(refname)
            .ok()
            .and_then(|r| r.target())
    }

    fn upstream_oid(&self) -> Option<git2::Oid> {
        let head = self.repo.head().ok()?;
        let local_name = head.shorthand()?;
        let local = self.repo.find_branch(local_name, git2::BranchType::Local).ok()?;
        let upstream = local.upstream().ok()?;
        upstream.get().target()
    }

    fn classify_commit_sync(
        &self,
        oid: git2::Oid,
        main_tip: Option<git2::Oid>,
        upstream_tip: Option<git2::Oid>,
    ) -> CommitSyncState {
        if let Some(main_tip) = main_tip {
            if main_tip == oid || self.repo.graph_descendant_of(main_tip, oid).unwrap_or(false) {
                return CommitSyncState::Main;
            }
        }
        if let Some(upstream_tip) = upstream_tip {
            if upstream_tip == oid || self
                .repo
                .graph_descendant_of(upstream_tip, oid)
                .unwrap_or(false)
            {
                return CommitSyncState::RemoteSynced;
            }
        }
        CommitSyncState::LocalOnly
    }
}

impl GitRepository for Git2Repository {
    fn status(&self) -> Result<GitStatus, GitError> {
        let mut git_status = GitStatus::default();

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .include_ignored(false)
            .update_index(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true)
            .include_unmodified(false)
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
        let main_tip = self
            .resolve_ref_oid("refs/heads/main")
            .or_else(|| self.resolve_ref_oid("refs/heads/master"));
        let upstream_tip = self.upstream_oid();

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
                sync_state: self.classify_commit_sync(oid, main_tip, upstream_tip),
            });
        }
        Ok(result)
    }

    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError> {
        let output = self.run_git(&["show", "--name-status", "--pretty=format:", oid])?;
        let mut files = Vec::new();
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            let status_str = parts.next().unwrap_or_default();
            let path_str = parts.next().unwrap_or_default();
            if path_str.is_empty() {
                continue;
            }
            let status = match status_str.chars().next().unwrap_or('M') {
                'A' => FileStatus::New,
                'D' => FileStatus::Deleted,
                'R' => FileStatus::Renamed,
                'T' => FileStatus::TypeChange,
                _ => FileStatus::Modified,
            };
            files.push(FileEntry {
                path: PathBuf::from(path_str),
                status,
            });
        }
        Ok(files)
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

    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError> {
        let spec = format!("stash@{{{}}}", index);
        let output = self.run_git(&["stash", "show", "--name-status", "--pretty=format:", &spec])?;
        let mut files = Vec::new();

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut parts = trimmed.split_whitespace();
            let status_str = parts.next().unwrap_or_default();
            let path_str = parts.next().unwrap_or_default();
            if path_str.is_empty() {
                continue;
            }

            let status = match status_str.chars().next().unwrap_or('M') {
                'A' => FileStatus::New,
                'D' => FileStatus::Deleted,
                'R' => FileStatus::Renamed,
                'T' => FileStatus::TypeChange,
                _ => FileStatus::Modified,
            };

            files.push(FileEntry {
                path: PathBuf::from(path_str),
                status,
            });
        }

        Ok(files)
    }

    fn stash_diff(&self, index: usize, path: Option<&std::path::Path>) -> Result<Vec<DiffLine>, GitError> {
        let spec = format!("stash@{{{}}}", index);
        let mut args = if path.is_some() {
            vec![
                "show".to_string(),
                spec,
            ]
        } else {
            vec![
                "stash".to_string(),
                "show".to_string(),
                "-p".to_string(),
                spec,
            ]
        };

        if let Some(path) = path {
            let Some(path_str) = path.to_str() else {
                return Err(GitError::Git2("stash path contains invalid unicode".to_string()));
            };
            args.push("--".to_string());
            args.push(path_str.to_string());
        }

        let patch = self.run_git_owned(&args)?;
        Ok(parse_patch_text(&patch))
    }

    fn stash_push_paths(&self, paths: &[PathBuf], message: &str) -> Result<usize, GitError> {
        if paths.is_empty() {
            return Err(GitError::Git2("no selected paths for stash".to_string()));
        }

        let before = self.stashes()?.len();
        let mut args = vec![
            "stash".to_string(),
            "push".to_string(),
            "-u".to_string(),
            "-m".to_string(),
            message.to_string(),
            "--".to_string(),
        ];
        for path in paths {
            let Some(path_str) = path.to_str() else {
                return Err(GitError::Git2("stash path contains invalid unicode".to_string()));
            };
            args.push(path_str.to_string());
        }

        self.run_git_owned(&args)?;
        let after = self.stashes()?;
        if after.len() <= before {
            return Err(GitError::Git2("no local changes in selected paths to stash".to_string()));
        }
        Ok(after[0].index)
    }

    fn stash_apply(&self, index: usize) -> Result<(), GitError> {
        let spec = format!("stash@{{{}}}", index);
        self.run_git(&["stash", "apply", &spec])?;
        Ok(())
    }

    fn stash_pop(&self, index: usize) -> Result<(), GitError> {
        let spec = format!("stash@{{{}}}", index);
        self.run_git(&["stash", "pop", &spec])?;
        Ok(())
    }

    fn stash_drop(&self, index: usize) -> Result<(), GitError> {
        let spec = format!("stash@{{{}}}", index);
        self.run_git(&["stash", "drop", &spec])?;
        Ok(())
    }

    fn commit_diff_scoped(&self, oid: &str, path: Option<&std::path::Path>) -> Result<Vec<DiffLine>, GitError> {
        let mut args = vec![
            "show".to_string(),
            "--pretty=format:".to_string(),
            oid.to_string(),
        ];
        if let Some(path) = path {
            let Some(path_str) = path.to_str() else {
                return Err(GitError::Git2("commit path contains invalid unicode".to_string()));
            };
            args.push("--".to_string());
            args.push(path_str.to_string());
        }
        let patch = self.run_git_owned(&args)?;
        Ok(parse_patch_text(&patch))
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

    fn fetch_default(&self) -> Result<String, GitError> {
        let upstream = self.run_git(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
        let remote = match upstream {
            Ok(name) => parse_remote_from_upstream(&name).unwrap_or_else(|| "origin".to_string()),
            Err(_) => "origin".to_string(),
        };

        self.run_git(&["fetch", "--prune", &remote])?;
        Ok(remote)
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

fn parse_patch_text(text: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let (kind, content) = if let Some(rest) = raw.strip_prefix('+') {
            (DiffLineKind::Added, rest.to_string())
        } else if let Some(rest) = raw.strip_prefix('-') {
            (DiffLineKind::Removed, rest.to_string())
        } else if raw.starts_with("diff --git")
            || raw.starts_with("index ")
            || raw.starts_with("@@")
            || raw.starts_with("--- ")
            || raw.starts_with("+++ ")
        {
            (DiffLineKind::Header, raw.to_string())
        } else {
            (DiffLineKind::Context, raw.to_string())
        };
        lines.push(DiffLine { kind, content });
    }
    lines
}

fn parse_remote_from_upstream(upstream: &str) -> Option<String> {
    upstream
        .split('/')
        .next()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
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
        assert_eq!(commits[0].sync_state, CommitSyncState::Main);

        let patch = repo
            .commit_diff_scoped(&oid, None)
            .expect("commit diff");
        assert!(!patch.is_empty());

        let files = repo.commit_files(&oid).expect("commit files");
        assert!(files.iter().any(|f| f.path == std::path::Path::new("new.txt")));

        let scoped = repo
            .commit_diff_scoped(&oid, Some(std::path::Path::new("new.txt")))
            .expect("commit scoped diff");
        assert!(!scoped.is_empty());
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

    #[test]
    fn test_stash_push_apply_drop() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v2\n");

        let created = repo
            .stash_push_paths(&[PathBuf::from("tracked.txt")], "wip")
            .expect("stash push");
        assert_eq!(created, 0);
        assert_eq!(repo.stashes().expect("stashes after push").len(), 1);

        repo.stash_apply(0).expect("stash apply");
        let status_after_apply = repo.status().expect("status after stash apply");
        assert!(status_after_apply
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));

        repo.stash_drop(0).expect("stash drop");
        assert!(repo.stashes().expect("stashes after drop").is_empty());
    }

    #[test]
    fn test_stash_pop_restores_and_removes_entry() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v3\n");

        repo
            .stash_push_paths(&[PathBuf::from("tracked.txt")], "wip pop")
            .expect("stash push");
        assert_eq!(repo.stashes().expect("stashes after push").len(), 1);

        repo.stash_pop(0).expect("stash pop");
        let status = repo.status().expect("status after stash pop");
        assert!(status
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));
        assert!(repo.stashes().expect("stashes after pop").is_empty());
    }

    #[test]
    fn test_stash_push_only_selected_paths() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "tracked changed\n");
        write_file(&dir.path().join("other.txt"), "other changed\n");

        repo
            .stash_push_paths(&[PathBuf::from("tracked.txt")], "partial")
            .expect("stash push selected path");

        let status = repo.status().expect("status after partial stash");
        assert!(!repo.stashes().expect("stashes after partial push").is_empty());
        assert!(
            status
                .unstaged
                .iter()
                .chain(status.untracked.iter())
                .any(|f| f.path == std::path::Path::new("other.txt"))
        );
    }

    #[test]
    fn test_stash_diff_for_selected_path() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v2\n");

        repo
            .stash_push_paths(&[PathBuf::from("tracked.txt")], "diff path")
            .expect("stash push for diff");

        let diff = repo
            .stash_diff(0, Some(Path::new("tracked.txt")))
            .expect("stash diff for path");
        assert!(!diff.is_empty());
        assert!(diff.iter().any(|l| matches!(l.kind, DiffLineKind::Header)));
    }

    #[test]
    fn test_parse_remote_from_upstream() {
        assert_eq!(
            parse_remote_from_upstream("origin/main").as_deref(),
            Some("origin")
        );
        assert_eq!(
            parse_remote_from_upstream("upstream/feature/x").as_deref(),
            Some("upstream")
        );
        assert!(parse_remote_from_upstream("").is_none());
    }
}
