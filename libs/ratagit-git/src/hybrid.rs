use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use git2::{
    BranchType, DiffDelta, ErrorCode, Object, Oid, Repository, Status, StatusEntry, StatusOptions,
    StatusShow,
};
use ratagit_core::{
    BranchDeleteMode, BranchEntry, COMMITS_PAGE_SIZE, CommitEntry, CommitHashStatus,
    FileDiffTarget, FileEntry, FilesSnapshot, RepoSnapshot, ResetMode, StashEntry, StatusMode,
};

use crate::cli::GitCli;
use crate::untracked_diff::format_untracked_diffs;
use crate::{GitBackend, GitError, validate_repo_relative_path};

pub struct HybridGitBackend {
    repo: Repository,
    workdir: PathBuf,
    cli: GitCli,
}

impl HybridGitBackend {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let repo = Repository::discover(path.as_ref())?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| GitError::new("bare git repositories are not supported"))?
            .to_path_buf();
        let cli = GitCli::new(workdir.clone());
        Ok(Self { repo, workdir, cli })
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
        let files_snapshot = self.refresh_files()?;
        let commits = self.refresh_commits()?;
        let branches = self.refresh_branches()?;
        let stashes = self.refresh_stashes()?;

        Ok(RepoSnapshot {
            status_summary: files_snapshot.status_summary,
            current_branch: files_snapshot.current_branch,
            detached_head: files_snapshot.detached_head,
            files: files_snapshot.files,
            commits,
            branches,
            stashes,
        })
    }

    fn refresh_files(&mut self) -> Result<FilesSnapshot, GitError> {
        let started = Instant::now();
        let (current_branch, detached_head) = current_head(&self.repo)?;
        trace_step("head", started);
        let started = Instant::now();
        let index_entry_count = self.repo.index()?.len();
        trace_index_step(started, index_entry_count);
        let status_mode = status_mode_for_index_entry_count(index_entry_count);
        let (files, status_truncated, status_scan_skipped, status_summary) =
            if status_mode == StatusMode::HugeRepoMetadataOnly {
                trace_status_skipped(index_entry_count, status_mode);
                (
                    Vec::new(),
                    false,
                    true,
                    format!("status scan skipped: {index_entry_count} indexed files"),
                )
            } else {
                let started = Instant::now();
                let status = collect_files(&self.cli, &self.repo, status_mode)?;
                trace_status_step(
                    started,
                    index_entry_count,
                    status.files.len(),
                    status.truncated,
                    status_mode,
                );
                let status_summary = summarize_files(&status.files);
                (status.files, status.truncated, false, status_summary)
            };
        let large_repo_mode = status_mode != StatusMode::Full;

        Ok(FilesSnapshot {
            status_summary,
            current_branch,
            detached_head,
            files,
            index_entry_count,
            large_repo_mode,
            status_truncated,
            status_scan_skipped,
            untracked_scan_skipped: status_mode != StatusMode::Full,
        })
    }

    fn refresh_commits(&mut self) -> Result<Vec<CommitEntry>, GitError> {
        let started = Instant::now();
        let (current_branch, detached_head) = current_head(&self.repo)?;
        trace_step("head", started);
        let started = Instant::now();
        let commits = collect_commits_page(
            &self.cli,
            &self.repo,
            &current_branch,
            detached_head,
            0,
            COMMITS_PAGE_SIZE,
        )?;
        trace_step("commits", started);
        Ok(commits)
    }

    fn refresh_branches(&mut self) -> Result<Vec<BranchEntry>, GitError> {
        let started = Instant::now();
        let (current_branch, detached_head) = current_head(&self.repo)?;
        trace_step("head", started);
        let started = Instant::now();
        let branches = collect_branches(&self.repo, &current_branch, detached_head)?;
        trace_step("branches", started);
        Ok(branches)
    }

    fn refresh_stashes(&mut self) -> Result<Vec<StashEntry>, GitError> {
        let started = Instant::now();
        let stashes = self.collect_stashes()?;
        trace_step("stashes", started);
        Ok(stashes)
    }

    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CommitEntry>, GitError> {
        let (current_branch, detached_head) = current_head(&self.repo)?;
        collect_commits_page(
            &self.cli,
            &self.repo,
            &current_branch,
            detached_head,
            offset,
            limit,
        )
    }

    fn files_details_diff(&mut self, targets: &[FileDiffTarget]) -> Result<String, GitError> {
        if targets.is_empty() {
            return Ok(String::new());
        }

        let selected_paths = selected_pathspecs(targets.iter().map(|target| &target.path))?;
        let started = Instant::now();
        let mut unstaged = self.cli.files_unstaged_diff(&selected_paths)?;
        trace_diff_step("unstaged_diff", started, selected_paths.len());
        let started = Instant::now();
        let untracked = format_untracked_diffs(&self.workdir, targets)?;
        trace_diff_step("untracked_diff", started, selected_paths.len());
        if !untracked.is_empty() {
            if !unstaged.trim().is_empty() {
                unstaged.push('\n');
            }
            unstaged.push_str(&untracked);
        }

        let started = Instant::now();
        let staged = self.cli.files_staged_diff(&selected_paths)?;
        trace_diff_step("staged_diff", started, selected_paths.len());

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

    fn commit_files(
        &mut self,
        commit_id: &str,
    ) -> Result<Vec<ratagit_core::CommitFileEntry>, GitError> {
        self.cli.commit_files(commit_id)
    }

    fn commit_file_diff(
        &mut self,
        target: &ratagit_core::CommitFileDiffTarget,
    ) -> Result<String, GitError> {
        self.cli.commit_file_diff(target)
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectedFiles {
    files: Vec<FileEntry>,
    truncated: bool,
}

fn collect_files(
    cli: &GitCli,
    repo: &Repository,
    mode: StatusMode,
) -> Result<CollectedFiles, GitError> {
    match cli.status_files(mode) {
        Ok(mut status) => {
            let started = Instant::now();
            sort_files(&mut status.files);
            trace_files_sort_step(started, status.files.len());
            if status.output_truncated || status.entries_truncated {
                tracing::warn!(
                    target: "ratagit.git",
                    output_truncated = status.output_truncated,
                    entries_truncated = status.entries_truncated,
                    result_count = status.files.len(),
                    "git status result truncated"
                );
            }
            Ok(CollectedFiles {
                files: status.files,
                truncated: status.output_truncated || status.entries_truncated,
            })
        }
        Err(error) => {
            tracing::warn!(
                target: "ratagit.git",
                error = %error,
                "git cli status failed; falling back to git2 status"
            );
            let started = Instant::now();
            let files = collect_files_with_git2(repo, mode)?;
            tracing::debug!(
                target: "ratagit.git",
                mode = ?mode,
                elapsed_ms = started.elapsed().as_millis(),
                result_count = files.len(),
                "git2 status fallback completed"
            );
            Ok(CollectedFiles {
                files,
                truncated: false,
            })
        }
    }
}

fn collect_files_with_git2(
    repo: &Repository,
    mode: StatusMode,
) -> Result<Vec<FileEntry>, GitError> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(mode == StatusMode::Full)
        .recurse_untracked_dirs(mode == StatusMode::Full)
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
    cli: &GitCli,
    repo: &Repository,
    current_branch: &str,
    detached_head: bool,
    offset: usize,
    limit: usize,
) -> Result<Vec<CommitEntry>, GitError> {
    if head_is_missing(repo)? {
        return Ok(Vec::new());
    }
    match collect_commits_page_with_cli(cli, repo, current_branch, detached_head, offset, limit) {
        Ok(commits) => return Ok(commits),
        Err(error) => {
            tracing::warn!(
                target: "ratagit.git",
                error = %error,
                offset,
                limit,
                "git cli commit page failed; falling back to git2 revwalk"
            );
        }
    }
    collect_commits_page_with_git2(repo, current_branch, detached_head, offset, limit)
}

fn collect_commits_page_with_cli(
    cli: &GitCli,
    repo: &Repository,
    current_branch: &str,
    detached_head: bool,
    offset: usize,
    limit: usize,
) -> Result<Vec<CommitEntry>, GitError> {
    let output = cli.commit_log_page(offset, limit)?;
    let parsed = parse_commit_log_page(&output)?;
    let oids = parsed.iter().map(|entry| entry.oid).collect::<Vec<_>>();
    let statuses = classify_commit_hash_statuses(repo, current_branch, detached_head, &oids)?;
    Ok(parsed
        .into_iter()
        .map(|entry| CommitEntry {
            id: entry.id,
            full_id: entry.full_id,
            summary: entry.summary,
            message: entry.message,
            author_name: entry.author_name,
            graph: "●".to_string(),
            hash_status: statuses
                .get(&entry.oid)
                .copied()
                .unwrap_or(CommitHashStatus::Unpushed),
            is_merge: entry.is_merge,
        })
        .collect())
}

fn collect_commits_page_with_git2(
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

#[derive(Debug, PartialEq, Eq)]
struct ParsedCommitLogEntry {
    oid: Oid,
    id: String,
    full_id: String,
    summary: String,
    message: String,
    author_name: String,
    is_merge: bool,
}

fn parse_commit_log_page(output: &[u8]) -> Result<Vec<ParsedCommitLogEntry>, GitError> {
    let mut entries = Vec::new();
    for record in output.split(|byte| *byte == 0x1e) {
        if record.iter().all(|byte| byte.is_ascii_whitespace()) {
            continue;
        }
        let fields = record.split(|byte| *byte == 0).collect::<Vec<_>>();
        if fields.len() < 5 {
            return Err(GitError::new("invalid git log record"));
        }
        let full_id = utf8_field(fields[0], "full commit id")?.trim();
        let oid = Oid::from_str(full_id)
            .map_err(|error| GitError::new(format!("invalid commit id from git log: {error}")))?;
        let id = utf8_field(fields[1], "short commit id")?.trim();
        let parents = utf8_field(fields[2], "commit parents")?.trim();
        let author_name = utf8_field(fields[3], "commit author")?.trim();
        let message = utf8_field(fields[4], "commit message")?
            .trim_end()
            .to_string();
        let summary = message.lines().next().unwrap_or("").trim().to_string();
        if summary.is_empty() {
            continue;
        }
        entries.push(ParsedCommitLogEntry {
            oid,
            id: if id.is_empty() {
                short_oid(oid)
            } else {
                id.to_string()
            },
            full_id: oid.to_string(),
            summary,
            message,
            author_name: if author_name.is_empty() {
                "unknown".to_string()
            } else {
                author_name.to_string()
            },
            is_merge: parents.split_whitespace().count() > 1,
        });
    }
    Ok(entries)
}

fn utf8_field<'a>(field: &'a [u8], label: &str) -> Result<&'a str, GitError> {
    std::str::from_utf8(field)
        .map_err(|error| GitError::new(format!("invalid utf-8 {label} from git log: {error}")))
}

fn classify_commit_hash_statuses(
    repo: &Repository,
    current_branch: &str,
    detached_head: bool,
    oids: &[Oid],
) -> Result<HashMap<Oid, CommitHashStatus>, GitError> {
    let mut statuses = oids
        .iter()
        .copied()
        .map(|oid| (oid, CommitHashStatus::Unpushed))
        .collect::<HashMap<_, _>>();
    if statuses.is_empty() {
        return Ok(statuses);
    }
    if let Ok(main_oid) = repo.refname_to_id("refs/heads/main") {
        mark_reachable_commits(
            repo,
            main_oid,
            &mut statuses,
            CommitHashStatus::MergedToMain,
        )?;
    }
    if !detached_head && let Some(upstream_oid) = upstream_oid(repo, current_branch)? {
        mark_reachable_commits(repo, upstream_oid, &mut statuses, CommitHashStatus::Pushed)?;
    }
    Ok(statuses)
}

fn mark_reachable_commits(
    repo: &Repository,
    tip: Oid,
    statuses: &mut HashMap<Oid, CommitHashStatus>,
    status: CommitHashStatus,
) -> Result<(), GitError> {
    let mut remaining = statuses
        .iter()
        .filter_map(|(oid, current_status)| {
            (*current_status == CommitHashStatus::Unpushed).then_some(*oid)
        })
        .collect::<HashSet<_>>();
    if remaining.is_empty() {
        return Ok(());
    }
    let mut revwalk = repo.revwalk()?;
    revwalk.push(tip)?;
    for oid in revwalk {
        let oid = oid?;
        if remaining.remove(&oid) {
            statuses.insert(oid, status);
            if remaining.is_empty() {
                break;
            }
        }
    }
    Ok(())
}

fn head_is_missing(repo: &Repository) -> Result<bool, GitError> {
    match repo.head() {
        Ok(_) => Ok(false),
        Err(error) if is_missing_head_error(&error) => Ok(true),
        Err(error) => Err(error.into()),
    }
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

pub(crate) const LARGE_REPO_INDEX_ENTRY_THRESHOLD: usize = 100_000;
pub(crate) const HUGE_REPO_INDEX_ENTRY_THRESHOLD: usize = 1_000_000;

fn status_mode_for_index_entry_count(index_entry_count: usize) -> StatusMode {
    if index_entry_count >= HUGE_REPO_INDEX_ENTRY_THRESHOLD {
        StatusMode::HugeRepoMetadataOnly
    } else if index_entry_count >= LARGE_REPO_INDEX_ENTRY_THRESHOLD {
        StatusMode::LargeRepoFast
    } else {
        StatusMode::Full
    }
}

fn trace_step(step: &'static str, started: Instant) {
    tracing::debug!(
        target: "ratagit.git",
        step,
        elapsed_ms = started.elapsed().as_millis(),
        "git backend step completed"
    );
}

fn trace_index_step(started: Instant, index_entry_count: usize) {
    tracing::debug!(
        target: "ratagit.git",
        step = "index",
        elapsed_ms = started.elapsed().as_millis(),
        index_entry_count,
        "git backend index count completed"
    );
}

fn trace_files_sort_step(started: Instant, result_count: usize) {
    tracing::debug!(
        target: "ratagit.git",
        step = "status_sort",
        elapsed_ms = started.elapsed().as_millis(),
        result_count,
        "git backend status sort completed"
    );
}

fn trace_status_step(
    started: Instant,
    index_entry_count: usize,
    result_count: usize,
    truncated: bool,
    mode: StatusMode,
) {
    tracing::debug!(
        target: "ratagit.git",
        command = "status",
        elapsed_ms = started.elapsed().as_millis(),
        index_entry_count,
        result_count,
        truncated,
        large_repo_mode = mode != StatusMode::Full,
        "git backend status completed"
    );
}

fn trace_status_skipped(index_entry_count: usize, mode: StatusMode) {
    tracing::warn!(
        target: "ratagit.git",
        command = "status",
        index_entry_count,
        large_repo_mode = mode != StatusMode::Full,
        "git backend status skipped"
    );
}

fn trace_diff_step(step: &'static str, started: Instant, path_count: usize) {
    tracing::debug!(
        target: "ratagit.git",
        step,
        command = "diff",
        elapsed_ms = started.elapsed().as_millis(),
        path_count,
        "git backend diff step completed"
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

fn selected_pathspecs<'a>(
    paths: impl IntoIterator<Item = &'a String>,
) -> Result<Vec<String>, GitError> {
    paths
        .into_iter()
        .map(|path| {
            validate_repo_relative_path(path)?;
            Ok(path.clone())
        })
        .collect()
}

fn short_oid(oid: Oid) -> String {
    oid.to_string().chars().take(7).collect()
}

#[cfg(test)]
mod tests;
