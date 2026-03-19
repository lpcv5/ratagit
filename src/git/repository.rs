use std::path::PathBuf;
use thiserror::Error;

/// Git 操作错误
#[derive(Debug, Clone, Error)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git2(String),

    #[error("Repository not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid repository state")]
    InvalidState,
}

// 实现从 git2::Error 转换
impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError::Git2(err.to_string())
    }
}

/// 文件状态
#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
}

/// 文件条目
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub status: FileStatus,
}

/// Git 仓库状态
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    pub unstaged: Vec<FileEntry>,
    pub staged: Vec<FileEntry>,
    pub untracked: Vec<FileEntry>,
}

/// Diff 行类型
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    Header,
}

/// Diff 行
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

/// Git 仓库 trait（抽象 git2/gix）
/// Phase 1: 暂不要求 Send + Sync，Phase 2 再引入异步
pub trait GitRepository {
    /// 获取仓库状态
    fn status(&self) -> Result<GitStatus, GitError>;

    /// Stage 文件
    fn stage(&self, path: &PathBuf) -> Result<(), GitError>;

    /// Unstage 文件
    fn unstage(&self, path: &PathBuf) -> Result<(), GitError>;

    /// 获取工作区文件 diff（unstaged，已跟踪）
    fn diff_unstaged(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError>;

    /// 获取暂存区文件 diff（staged）
    fn diff_staged(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError>;

    /// 获取未跟踪文件内容（全部作为新增行）
    fn diff_untracked(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError>;

    /// 获取仓库根目录
    fn workdir(&self) -> Option<PathBuf>;

    /// 获取本地分支列表
    fn branches(&self) -> Result<Vec<BranchInfo>, GitError>;

    /// 获取当前分支的 commit 历史
    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError>;

    /// 获取 stash 列表
    fn stashes(&self) -> Result<Vec<StashInfo>, GitError>;
}

/// git2 实现
pub struct Git2Repository {
    repo: git2::Repository,
}

impl Git2Repository {
    /// 在当前目录或父目录中查找 Git 仓库
    pub fn discover() -> Result<Self, GitError> {
        let repo = git2::Repository::discover(".")?;
        Ok(Self { repo })
    }

    /// 从路径创建
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, GitError> {
        let repo = git2::Repository::open(path)?;
        Ok(Self { repo })
    }

    /// 将 git2::Status 转换为 FileStatus
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
            FileStatus::Modified // 默认
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

            // 分类文件
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

    fn stage(&self, path: &PathBuf) -> Result<(), GitError> {
        let mut index = self.repo.index()?;
        index.add_path(path)?;
        index.write()?;
        Ok(())
    }

    fn unstage(&self, path: &PathBuf) -> Result<(), GitError> {
        // 获取 HEAD commit
        let head = self.repo.head()?.target().ok_or(GitError::InvalidState)?;
        let commit_obj = self.repo.find_object(head, Some(git2::ObjectType::Commit))?;

        // 使用 reset_default 从 HEAD 恢复文件到 index
        let path_str = path.to_str().ok_or(GitError::InvalidState)?;
        self.repo.reset_default(Some(&commit_obj), &[path_str])?;

        Ok(())
    }
    fn diff_unstaged(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError> {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""));

        let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_staged(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError> {
        let head_tree = self.repo.head().ok()
            .and_then(|h| h.peel_to_tree().ok());

        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""));

        let diff = self.repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_untracked(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError> {
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

    fn workdir(&self) -> Option<PathBuf> {
        self.repo.workdir().map(|p| p.to_path_buf())
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
mod tests {
    use super::*;

    #[test]
    fn test_discover_repo() {
        // 当前目录应该是一个 Git 仓库（ratagit 项目本身）
        let result = Git2Repository::discover();
        assert!(result.is_ok());
    }
}
