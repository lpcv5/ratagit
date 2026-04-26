use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use git2::{
    BranchType, Diff, DiffDelta, DiffFormat, DiffOptions, ErrorCode, Object, Oid, Repository,
    Status, StatusEntry, StatusOptions, StatusShow, Tree,
};
use ratagit_core::{
    BranchDeleteMode, BranchEntry, COMMITS_PAGE_SIZE, CommitEntry, CommitHashStatus, FileEntry,
    RepoSnapshot, ResetMode, StashEntry,
};

use crate::cli::GitCli;
use crate::untracked_diff::format_untracked_diffs;
use crate::{GitBackend, GitError, validate_repo_relative_path};

pub struct HybridGitBackend {
    repo: Repository,
    workdir: PathBuf,
    cli: GitCli,
    last_files: Vec<FileEntry>,
}

impl HybridGitBackend {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let repo = Repository::discover(path.as_ref())?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::new("bare git repositories are not supported"))?
            .to_path_buf();
        let cli = GitCli::new(workdir.clone());
        Ok(Self {
            repo,
            workdir,
            cli,
            last_files: Vec::new(),
        })
    }

    fn collect_stashes(&mut self) -> Result<Vec<StashEntry>, GitError> {
        let mut stashes = Vec::new();
        let result = self.repo.stash_foreach(|index, message, _oid| {
            stashes.push(StashEntry {
                id: format!("stash@{{{index}}}"),
                summary: message.to_string(),
            });
            true
        });

        match result {
            Ok(()) => Ok(stashes),
            Err(error) if error.code() == ErrorCode::NotFound => Ok(stashes),
            Err(error) => Err(error.into()),
        }
    }

    fn head_tree(&self) -> Result<Option<Tree<'_>>, GitError> {
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(error) if is_missing_head_error(&error) => {
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let commit = match head.peel_to_commit() {
            Ok(commit) => commit,
            Err(error) if is_missing_head_error(&error) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        Ok(Some(commit.tree()?))
    }

    fn head_object(&self) -> Result<Option<Object<'_>>, GitError> {
        match self.repo.revparse_single("HEAD") {
            Ok(object) => Ok(Some(object)),
            Err(error) if is_missing_head_error(&error) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

impl fmt::Debug for HybridGitBackend {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HybridGitBackend")
            .field("workdir", &self.workdir)
            .finish_non_exhaustive()
    }
}

impl GitBackend for HybridGitBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        let started = Instant::now();
        let (current_branch, detached_head) = current_head(&self.repo)?;
        trace_step("head", started);
        let started = Instant::now();
        let files = collect_files(&self.cli, &self.repo)?;
        trace_step("status", started);
        let status_summary = summarize_files(&files);
        let started = Instant::now();
        let commits = collect_commits_page(
            &self.repo,
            &current_branch,
            detached_head,
            0,
            COMMITS_PAGE_SIZE,
        )?;
        trace_step("commits", started);
        let started = Instant::now();
        let branches = collect_branches(&self.repo, &current_branch, detached_head)?;
        trace_step("branches", started);
        let started = Instant::now();
        let stashes = self.collect_stashes()?;
        trace_step("stashes", started);
        self.last_files = files.clone();

        Ok(RepoSnapshot {
            status_summary,
            current_branch,
            detached_head,
            files,
            commits,
            branches,
            stashes,
        })
    }

    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CommitEntry>, GitError> {
        let (current_branch, detached_head) = current_head(&self.repo)?;
        collect_commits_page(&self.repo, &current_branch, detached_head, offset, limit)
    }

    fn files_details_diff(&mut self, paths: &[String]) -> Result<String, GitError> {
        if paths.is_empty() {
            return Ok(String::new());
        }

        let selected_paths = selected_pathspecs(paths)?;
        let mut unstaged_options = diff_options(&selected_paths);
        let started = Instant::now();
        let mut unstaged = format_diff(
            &self
                .repo
                .diff_index_to_workdir(None, Some(&mut unstaged_options))?,
        )?;
        trace_step("unstaged_diff", started);
        let files = if self.last_files.is_empty() {
            collect_files(&self.cli, &self.repo)?
        } else {
            self.last_files.clone()
        };
        let started = Instant::now();
        let untracked = format_untracked_diffs(&self.workdir, files, &selected_paths)?;
        trace_step("untracked_diff", started);
        if !untracked.is_empty() {
            if !unstaged.trim().is_empty() {
                unstaged.push('\n');
            }
            unstaged.push_str(&untracked);
        }

        let head_tree = self.head_tree()?;
        let mut staged_options = diff_options(&selected_paths);
        let started = Instant::now();
        let staged = format_diff(&self.repo.diff_tree_to_index(
            head_tree.as_ref(),
            None,
            Some(&mut staged_options),
        )?)?;
        trace_step("staged_diff", started);

        let mut sections = Vec::new();
        if !unstaged.trim().is_empty() {
            sections.push("### unstaged".to_string());
            sections.push(unstaged.trim_end().to_string());
        }
        if !staged.trim().is_empty() {
            if !sections.is_empty() {
                sections.push(String::new());
            }
            sections.push("### staged".to_string());
            sections.push(staged.trim_end().to_string());
        }

        Ok(sections.join("\n"))
    }

    fn branch_details_log(&mut self, branch: &str, max_count: usize) -> Result<String, GitError> {
        self.cli.branch_details_log(branch, max_count)
    }

    fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, GitError> {
        self.cli.commit_details_diff(commit_id)
    }

    fn stage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.stage_files(&[path.to_string()])
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.unstage_files(&[path.to_string()])
    }

    fn stage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut index = self.repo.index()?;
        for path in paths {
            let relative_path = validate_repo_relative_path(path)?;
            if self.workdir.join(relative_path).exists() {
                index.add_path(relative_path)?;
            } else {
                index.remove_path(relative_path)?;
            }
        }
        index.write()?;
        Ok(())
    }

    fn unstage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let validated = paths
            .iter()
            .map(|path| {
                validate_repo_relative_path(path)?;
                Ok(path.as_str())
            })
            .collect::<Result<Vec<_>, GitError>>()?;

        if let Some(head) = self.head_object()? {
            self.repo
                .reset_default(Some(&head), validated.iter().copied())?;
            return Ok(());
        }

        let mut index = self.repo.index()?;
        for path in validated {
            index.remove_path(Path::new(path))?;
        }
        index.write()?;
        Ok(())
    }

    fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.cli.create_commit(message)
    }

    fn create_branch(&mut self, name: &str, start_point: &str) -> Result<(), GitError> {
        self.cli.create_branch(name, start_point)
    }

    fn checkout_branch(&mut self, name: &str, auto_stash: bool) -> Result<(), GitError> {
        self.cli.checkout_branch(name, auto_stash)
    }

    fn delete_branch(
        &mut self,
        name: &str,
        mode: BranchDeleteMode,
        force: bool,
    ) -> Result<(), GitError> {
        self.cli.delete_branch(name, mode, force)
    }

    fn rebase_branch(
        &mut self,
        target: &str,
        interactive: bool,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        self.cli.rebase_branch(target, interactive, auto_stash)
    }

    fn squash_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.cli.squash_commits(commit_ids)
    }

    fn fixup_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.cli.fixup_commits(commit_ids)
    }

    fn reword_commit(&mut self, commit_id: &str, message: &str) -> Result<(), GitError> {
        self.cli.reword_commit(commit_id, message)
    }

    fn delete_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.cli.delete_commits(commit_ids)
    }

    fn checkout_commit_detached(
        &mut self,
        commit_id: &str,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        self.cli.checkout_commit_detached(commit_id, auto_stash)
    }

    fn stash_push(&mut self, message: &str) -> Result<(), GitError> {
        self.cli.stash_push(message)
    }

    fn stash_files(&mut self, message: &str, paths: &[String]) -> Result<(), GitError> {
        self.cli.stash_files(message, paths)
    }

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError> {
        self.cli.stash_pop(stash_id)
    }

    fn reset(&mut self, mode: ResetMode) -> Result<(), GitError> {
        self.cli.reset(mode)
    }

    fn nuke(&mut self) -> Result<(), GitError> {
        self.cli.nuke()
    }

    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.cli.discard_files(paths)
    }
}

fn current_head(repo: &Repository) -> Result<(String, bool), GitError> {
    match repo.head() {
        Ok(head) if head.is_branch() => {
            Ok((head.shorthand().unwrap_or("unknown").to_string(), false))
        }
        Ok(head) => {
            let name = head
                .target()
                .map(short_oid)
                .unwrap_or_else(|| "HEAD".to_string());
            Ok((name, true))
        }
        Err(error) if is_missing_head_error(&error) => {
            let branch =
                unborn_branch_from_head_file(repo).unwrap_or_else(|| "unknown".to_string());
            Ok((branch, false))
        }
        Err(error) => Err(error.into()),
    }
}

fn is_missing_head_error(error: &git2::Error) -> bool {
    matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch)
        || (error.message().contains("refs/heads") && error.message().contains("not found"))
}

fn collect_files(cli: &GitCli, repo: &Repository) -> Result<Vec<FileEntry>, GitError> {
    match cli.status_files() {
        Ok(mut files) => {
            sort_files(&mut files);
            Ok(files)
        }
        Err(error) => {
            tracing::debug!(
                target: "ratagit.git",
                error = %error,
                "git cli status failed; falling back to git2 status"
            );
            collect_files_with_git2(repo)
        }
    }
}

fn collect_files_with_git2(repo: &Repository) -> Result<Vec<FileEntry>, GitError> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true)
        .show(StatusShow::IndexAndWorkdir);
    let statuses = match repo.statuses(Some(&mut options)) {
        Ok(statuses) => statuses,
        Err(error) if is_missing_head_error(&error) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut files = statuses
        .iter()
        .filter_map(|entry| {
            if entry.status().contains(Status::IGNORED) {
                return None;
            }
            let path = status_entry_path(&entry)?;
            Some(file_entry_from_status(path, entry.status()))
        })
        .collect::<Vec<_>>();
    sort_files(&mut files);
    Ok(files)
}

fn collect_commits_page(
    repo: &Repository,
    current_branch: &str,
    detached_head: bool,
    offset: usize,
    limit: usize,
) -> Result<Vec<CommitEntry>, GitError> {
    let mut revwalk = repo.revwalk()?;
    match revwalk.push_head() {
        Ok(()) => {}
        Err(error) if is_missing_head_error(&error) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    }

    let main_oid = repo.refname_to_id("refs/heads/main").ok();
    let upstream_oid = if detached_head {
        None
    } else {
        upstream_oid(repo, current_branch)?
    };
    let mut commits = Vec::new();
    for oid in revwalk.skip(offset).take(limit) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let summary = commit.summary().unwrap_or("").trim();
        if summary.is_empty() {
            continue;
        }
        let message = commit.message().unwrap_or("").trim_end().to_string();
        let author_name = commit
            .author()
            .name()
            .unwrap_or("unknown")
            .trim()
            .to_string();
        commits.push(CommitEntry {
            id: short_oid(oid),
            full_id: oid.to_string(),
            summary: summary.to_string(),
            message,
            author_name,
            graph: "●".to_string(),
            hash_status: commit_hash_status(repo, oid, main_oid, upstream_oid)?,
            is_merge: commit.parent_count() > 1,
        });
    }
    Ok(commits)
}

fn upstream_oid(repo: &Repository, current_branch: &str) -> Result<Option<Oid>, GitError> {
    let branch = match repo.find_branch(current_branch, BranchType::Local) {
        Ok(branch) => branch,
        Err(error) if is_missing_head_error(&error) || error.code() == ErrorCode::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    match branch.upstream() {
        Ok(upstream) => Ok(upstream.get().target()),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn commit_hash_status(
    repo: &Repository,
    oid: Oid,
    main_oid: Option<Oid>,
    upstream_oid: Option<Oid>,
) -> Result<CommitHashStatus, GitError> {
    if let Some(main_oid) = main_oid
        && commit_is_reachable_from(repo, oid, main_oid)?
    {
        return Ok(CommitHashStatus::MergedToMain);
    }
    if let Some(upstream_oid) = upstream_oid
        && commit_is_reachable_from(repo, oid, upstream_oid)?
    {
        return Ok(CommitHashStatus::Pushed);
    }
    Ok(CommitHashStatus::Unpushed)
}

fn commit_is_reachable_from(repo: &Repository, oid: Oid, tip: Oid) -> Result<bool, GitError> {
    if oid == tip {
        return Ok(true);
    }
    repo.graph_descendant_of(tip, oid).map_err(Into::into)
}

fn collect_branches(
    repo: &Repository,
    current_branch: &str,
    detached_head: bool,
) -> Result<Vec<BranchEntry>, GitError> {
    let mut branches = Vec::new();
    let branch_iter = match repo.branches(Some(BranchType::Local)) {
        Ok(branch_iter) => branch_iter,
        Err(error) if is_missing_head_error(&error) => return Ok(branches),
        Err(error) => return Err(error.into()),
    };
    for branch in branch_iter {
        let (branch, _branch_type) = match branch {
            Ok(branch) => branch,
            Err(error) if is_missing_head_error(&error) => {
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let name = match branch.name() {
            Ok(Some(name)) => name,
            Ok(None) => continue,
            Err(error) if is_missing_head_error(&error) => continue,
            Err(error) => return Err(error.into()),
        };
        branches.push(branch_entry(name, current_branch, detached_head));
    }
    branches.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(branches)
}

fn status_entry_path(entry: &StatusEntry<'_>) -> Option<String> {
    entry
        .index_to_workdir()
        .and_then(delta_path)
        .or_else(|| entry.head_to_index().and_then(delta_path))
}

fn delta_path(delta: DiffDelta<'_>) -> Option<String> {
    if let Some(path) = delta.new_file().path() {
        return path_to_repo_string(path);
    }
    delta.old_file().path().and_then(path_to_repo_string)
}

fn path_to_repo_string(path: &Path) -> Option<String> {
    Some(path.to_str()?.replace('\\', "/"))
}

fn file_entry_from_status(path: String, status: Status) -> FileEntry {
    let untracked = status.contains(Status::WT_NEW) && !status.contains(Status::INDEX_NEW);
    let staged = status.intersects(
        Status::INDEX_NEW
            | Status::INDEX_MODIFIED
            | Status::INDEX_DELETED
            | Status::INDEX_RENAMED
            | Status::INDEX_TYPECHANGE
            | Status::CONFLICTED,
    );
    FileEntry {
        path,
        staged,
        untracked,
    }
}

fn sort_files(files: &mut [FileEntry]) {
    files.sort_by(|left, right| left.path.cmp(&right.path));
}

fn summarize_files(files: &[FileEntry]) -> String {
    let staged = files.iter().filter(|entry| entry.staged).count();
    let unstaged = files.len().saturating_sub(staged);
    format!("staged: {staged}, unstaged: {unstaged}")
}

fn trace_step(step: &'static str, started: Instant) {
    tracing::debug!(
        target: "ratagit.git",
        step,
        elapsed_ms = started.elapsed().as_millis(),
        "git backend step completed"
    );
}

fn branch_name_from_reference_name(name: &str) -> Option<&str> {
    name.strip_prefix("refs/heads/")
}

fn unborn_branch_from_head_file(repo: &Repository) -> Option<String> {
    let head = std::fs::read_to_string(repo.path().join("HEAD")).ok()?;
    let refname = head.trim().strip_prefix("ref: ")?;
    branch_name_from_reference_name(refname).map(ToString::to_string)
}

fn branch_entry(name: &str, current_branch: &str, detached_head: bool) -> BranchEntry {
    BranchEntry {
        name: name.to_string(),
        is_current: !detached_head && name == current_branch,
    }
}

fn selected_pathspecs(paths: &[String]) -> Result<Vec<String>, GitError> {
    paths
        .iter()
        .map(|path| {
            validate_repo_relative_path(path)?;
            Ok(path.clone())
        })
        .collect()
}

fn diff_options(paths: &[String]) -> DiffOptions {
    let mut options = DiffOptions::new();
    options.disable_pathspec_match(true);
    for path in paths {
        options.pathspec(path.as_str());
    }
    options
}

fn format_diff(diff: &Diff<'_>) -> Result<String, GitError> {
    let mut bytes = Vec::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        bytes.extend_from_slice(line.content());
        true
    })?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn short_oid(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

#[cfg(test)]
mod tests;
