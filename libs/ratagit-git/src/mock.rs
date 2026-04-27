use std::collections::BTreeMap;

use ratagit_core::{
    BranchDeleteMode, BranchEntry, COMMITS_PAGE_SIZE, CommitEntry, CommitFileDiffTarget,
    CommitFileEntry, CommitFileStatus, CommitHashStatus, FileDiffTarget, FilesSnapshot,
    RepoSnapshot, ResetMode, StashEntry,
};

use crate::{
    GitBackendHistoryRewrite, GitBackendRead, GitBackendWrite, GitError, resequence_stashes,
};

#[derive(Debug, Clone)]
pub struct MockGitBackend {
    snapshot: RepoSnapshot,
    operations: Vec<String>,
    commit_sequence: u64,
    index_entry_count: usize,
    large_repo_mode: bool,
    status_truncated: bool,
    status_scan_skipped: bool,
    untracked_scan_skipped: bool,
    commit_diff_overrides: BTreeMap<String, String>,
}

impl MockGitBackend {
    pub fn new(snapshot: RepoSnapshot) -> Self {
        let index_entry_count = snapshot.files.len();
        Self {
            snapshot,
            operations: Vec::new(),
            commit_sequence: 1,
            index_entry_count,
            large_repo_mode: false,
            status_truncated: false,
            status_scan_skipped: false,
            untracked_scan_skipped: false,
            commit_diff_overrides: BTreeMap::new(),
        }
    }

    pub fn with_status_metadata(
        snapshot: RepoSnapshot,
        index_entry_count: usize,
        large_repo_mode: bool,
        status_truncated: bool,
        untracked_scan_skipped: bool,
    ) -> Self {
        Self {
            snapshot,
            operations: Vec::new(),
            commit_sequence: 1,
            index_entry_count,
            large_repo_mode,
            status_truncated,
            status_scan_skipped: false,
            untracked_scan_skipped,
            commit_diff_overrides: BTreeMap::new(),
        }
    }

    pub fn with_huge_repo_status_metadata(
        snapshot: RepoSnapshot,
        index_entry_count: usize,
    ) -> Self {
        Self {
            snapshot,
            operations: Vec::new(),
            commit_sequence: 1,
            index_entry_count,
            large_repo_mode: true,
            status_truncated: false,
            status_scan_skipped: true,
            untracked_scan_skipped: true,
            commit_diff_overrides: BTreeMap::new(),
        }
    }

    pub fn with_commit_diff_overrides(
        snapshot: RepoSnapshot,
        commit_diff_overrides: BTreeMap<String, String>,
    ) -> Self {
        let index_entry_count = snapshot.files.len();
        Self {
            snapshot,
            operations: Vec::new(),
            commit_sequence: 1,
            index_entry_count,
            large_repo_mode: false,
            status_truncated: false,
            status_scan_skipped: false,
            untracked_scan_skipped: false,
            commit_diff_overrides,
        }
    }

    pub fn operations(&self) -> &[String] {
        &self.operations
    }

    pub fn snapshot(&self) -> &RepoSnapshot {
        &self.snapshot
    }

    fn merge_selected_commits_into_parents(
        &mut self,
        commit_ids: &[String],
        keep_messages: bool,
    ) -> Result<(), GitError> {
        if commit_ids.is_empty() {
            return Err(GitError::new("no commits selected"));
        }
        if self
            .snapshot
            .commits
            .iter()
            .any(|commit| commit_ids.iter().any(|id| commit_matches(commit, id)) && commit.is_merge)
        {
            return Err(GitError::new("merge commits are not supported"));
        }
        if self.snapshot.commits.iter().any(|commit| {
            commit_ids.iter().any(|id| commit_matches(commit, id))
                && commit.hash_status != CommitHashStatus::Unpushed
        }) {
            return Err(GitError::new("commit is not private"));
        }
        let target_indexes = self
            .snapshot
            .commits
            .iter()
            .enumerate()
            .filter_map(|(index, commit)| {
                commit_ids
                    .iter()
                    .any(|id| commit_matches(commit, id))
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        if target_indexes.is_empty() {
            return Err(GitError::new("commit not found"));
        }
        if target_indexes
            .iter()
            .any(|index| index.saturating_add(2) >= self.snapshot.commits.len())
        {
            return Err(GitError::new("cannot squash or fixup into root commit"));
        }
        let mut removed_any = false;
        let mut index = 0;
        while index < self.snapshot.commits.len() {
            if !commit_ids
                .iter()
                .any(|id| commit_matches(&self.snapshot.commits[index], id))
            {
                index += 1;
                continue;
            }
            let removed = self.snapshot.commits.remove(index);
            let Some(parent) = self.snapshot.commits.get_mut(index) else {
                return Err(GitError::new("cannot rewrite root commit"));
            };
            if keep_messages {
                parent.summary = format!("{} + {}", parent.summary, removed.summary);
                parent.message = format!(
                    "{}\n\n{}",
                    parent.message.trim_end(),
                    removed.message.trim_end()
                );
            }
            removed_any = true;
        }
        if removed_any {
            Ok(())
        } else {
            Err(GitError::new("commit not found"))
        }
    }
}

fn commit_matches(commit: &CommitEntry, id: &str) -> bool {
    commit.full_id == id || commit.id == id
}

impl GitBackendRead for MockGitBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        self.operations.push("refresh".to_string());
        let mut snapshot = self.snapshot.clone();
        snapshot.commits.truncate(COMMITS_PAGE_SIZE);
        Ok(snapshot)
    }

    fn refresh_files(&mut self) -> Result<FilesSnapshot, GitError> {
        self.operations.push("refresh-files".to_string());
        Ok(FilesSnapshot {
            status_summary: self.snapshot.status_summary.clone(),
            current_branch: self.snapshot.current_branch.clone(),
            detached_head: self.snapshot.detached_head,
            files: self.snapshot.files.clone(),
            index_entry_count: self.index_entry_count,
            large_repo_mode: self.large_repo_mode,
            status_truncated: self.status_truncated,
            status_scan_skipped: self.status_scan_skipped,
            untracked_scan_skipped: self.untracked_scan_skipped,
        })
    }

    fn refresh_branches(&mut self) -> Result<Vec<BranchEntry>, GitError> {
        self.operations.push("refresh-branches".to_string());
        Ok(self.snapshot.branches.clone())
    }

    fn refresh_commits(&mut self) -> Result<Vec<CommitEntry>, GitError> {
        self.operations.push("refresh-commits".to_string());
        Ok(self
            .snapshot
            .commits
            .iter()
            .take(COMMITS_PAGE_SIZE)
            .cloned()
            .collect())
    }

    fn branch_commits(&mut self, branch: &str) -> Result<Vec<CommitEntry>, GitError> {
        self.operations.push(format!("branch-commits:{branch}"));
        Ok(self
            .snapshot
            .commits
            .iter()
            .take(COMMITS_PAGE_SIZE)
            .cloned()
            .collect())
    }

    fn refresh_stashes(&mut self) -> Result<Vec<StashEntry>, GitError> {
        self.operations.push("refresh-stash".to_string());
        Ok(self.snapshot.stashes.clone())
    }

    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CommitEntry>, GitError> {
        self.operations
            .push(format!("commits-page:{offset}:{limit}"));
        Ok(self
            .snapshot
            .commits
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    fn files_details_diff(&mut self, targets: &[FileDiffTarget]) -> Result<String, GitError> {
        let paths = targets
            .iter()
            .map(|target| target.path.clone())
            .collect::<Vec<_>>();
        self.operations
            .push(format!("details-diff:{}", paths.join(",")));
        if paths.is_empty() {
            return Ok(String::new());
        }

        let mut unstaged = Vec::new();
        let mut staged = Vec::new();
        for path in paths {
            let Some(entry) = self.snapshot.files.iter().find(|entry| entry.path == *path) else {
                continue;
            };

            let hunk_body = if entry.untracked {
                format!("@@ -0,0 +1 @@\n+new file {}", entry.path)
            } else if entry.staged {
                format!(
                    "@@ -1 +1 @@\n-old staged {}\n+new staged {}",
                    entry.path, entry.path
                )
            } else {
                format!("@@ -1 +1 @@\n-old {}\n+new {}", entry.path, entry.path)
            };
            let block = format!("diff --git a/{0} b/{0}\n{1}", entry.path, hunk_body);
            if entry.staged {
                staged.push(block);
            } else {
                unstaged.push(block);
            }
        }

        let mut sections = Vec::new();
        if !unstaged.is_empty() {
            sections.push("### unstaged".to_string());
            sections.push(unstaged.join("\n"));
        }
        if !staged.is_empty() {
            if !sections.is_empty() {
                sections.push(String::new());
            }
            sections.push("### staged".to_string());
            sections.push(staged.join("\n"));
        }
        Ok(sections.join("\n"))
    }

    fn branch_details_log(&mut self, branch: &str, max_count: usize) -> Result<String, GitError> {
        self.operations
            .push(format!("branch-log:{branch}:{max_count}"));
        if !self
            .snapshot
            .branches
            .iter()
            .any(|entry| entry.name == branch)
        {
            return Err(GitError::new(format!("branch not found: {branch}")));
        }
        let commit = self
            .snapshot
            .commits
            .first()
            .map(|entry| (entry.id.as_str(), entry.summary.as_str()))
            .unwrap_or(("mock-0000", "no commits"));
        Ok(format!(
            "\u{1b}[33m*\u{1b}[m \u{1b}[33mcommit {}\u{1b}[m\nAuthor: ratagit-tests <ratagit-tests@example.com>\n\n    {} on {}",
            commit.0, commit.1, branch
        ))
    }

    fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, GitError> {
        self.operations.push(format!("commit-diff:{commit_id}"));
        let commit = self
            .snapshot
            .commits
            .iter()
            .find(|commit| commit_matches(commit, commit_id))
            .ok_or_else(|| GitError::new(format!("commit not found: {commit_id}")))?;
        if let Some(diff) = self
            .commit_diff_overrides
            .get(&commit.full_id)
            .or_else(|| self.commit_diff_overrides.get(&commit.id))
        {
            return Ok(diff.clone());
        }
        Ok(format!(
            "commit {}\nAuthor: ratagit-tests <ratagit-tests@example.com>\n\n    {}\n\ndiff --git a/commit.txt b/commit.txt\n@@ -1 +1 @@\n-old {}\n+new {}",
            commit.full_id, commit.summary, commit.id, commit.id
        ))
    }

    fn commit_files(&mut self, commit_id: &str) -> Result<Vec<CommitFileEntry>, GitError> {
        self.operations.push(format!("commit-files:{commit_id}"));
        self.snapshot
            .commits
            .iter()
            .find(|commit| commit_matches(commit, commit_id))
            .ok_or_else(|| GitError::new(format!("commit not found: {commit_id}")))?;
        Ok(vec![
            CommitFileEntry {
                path: "README.md".to_string(),
                old_path: None,
                status: CommitFileStatus::Modified,
            },
            CommitFileEntry {
                path: "src/lib.rs".to_string(),
                old_path: None,
                status: CommitFileStatus::Added,
            },
            CommitFileEntry {
                path: "src/new_name.rs".to_string(),
                old_path: Some("src/old_name.rs".to_string()),
                status: CommitFileStatus::Renamed,
            },
        ])
    }

    fn commit_file_diff(&mut self, target: &CommitFileDiffTarget) -> Result<String, GitError> {
        let path_list = target
            .paths
            .iter()
            .map(|path| path.path.as_str())
            .collect::<Vec<_>>()
            .join(",");
        self.operations.push(format!(
            "commit-file-diff:{}:{}",
            target.commit_id, path_list
        ));
        self.snapshot
            .commits
            .iter()
            .find(|commit| commit_matches(commit, &target.commit_id))
            .ok_or_else(|| GitError::new(format!("commit not found: {}", target.commit_id)))?;
        Ok(target
            .paths
            .iter()
            .map(|path| {
                let old_path = path.old_path.as_deref().unwrap_or(&path.path);
                format!(
                    "diff --git a/{old_path} b/{path}\n@@ -1 +1 @@\n-old {old_path}\n+new {path}",
                    path = path.path
                )
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

impl GitBackendWrite for MockGitBackend {
    fn stage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.operations.push(format!("stage:{path}"));
        let entry = self
            .snapshot
            .files
            .iter_mut()
            .find(|entry| entry.path == path)
            .ok_or_else(|| GitError::new(format!("file not found: {path}")))?;
        entry.staged = true;
        if entry.untracked {
            entry.status = CommitFileStatus::Added;
        }
        Ok(())
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.operations.push(format!("unstage:{path}"));
        let entry = self
            .snapshot
            .files
            .iter_mut()
            .find(|entry| entry.path == path)
            .ok_or_else(|| GitError::new(format!("file not found: {path}")))?;
        entry.staged = false;
        if entry.untracked {
            entry.status = CommitFileStatus::Unknown;
        }
        Ok(())
    }

    fn stage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("stage-files:{}", paths.join(",")));
        for path in paths {
            let entry = self
                .snapshot
                .files
                .iter_mut()
                .find(|entry| entry.path == *path)
                .ok_or_else(|| GitError::new(format!("file not found: {path}")))?;
            entry.staged = true;
            if entry.untracked {
                entry.status = CommitFileStatus::Added;
            }
        }
        Ok(())
    }

    fn unstage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("unstage-files:{}", paths.join(",")));
        for path in paths {
            let entry = self
                .snapshot
                .files
                .iter_mut()
                .find(|entry| entry.path == *path)
                .ok_or_else(|| GitError::new(format!("file not found: {path}")))?;
            entry.staged = false;
            if entry.untracked {
                entry.status = CommitFileStatus::Unknown;
            }
        }
        Ok(())
    }

    fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.operations.push(format!("commit:{message}"));
        if message.trim().is_empty() {
            return Err(GitError::new("commit message cannot be empty"));
        }
        let summary = message.lines().next().unwrap_or("").trim().to_string();
        self.snapshot.commits.insert(
            0,
            CommitEntry {
                id: format!("mock-{:04}", self.commit_sequence),
                full_id: format!("mock-{:04}", self.commit_sequence),
                summary,
                message: message.trim_end().to_string(),
                author_name: "ratagit-tests".to_string(),
                graph: "●".to_string(),
                hash_status: CommitHashStatus::Unpushed,
                is_merge: false,
            },
        );
        self.commit_sequence = self.commit_sequence.saturating_add(1);
        self.snapshot.files.retain(|entry| !entry.staged);
        Ok(())
    }

    fn create_branch(&mut self, name: &str, start_point: &str) -> Result<(), GitError> {
        self.operations
            .push(format!("create-branch:{name}:{start_point}"));
        if self
            .snapshot
            .branches
            .iter()
            .any(|branch| branch.name == name)
        {
            return Err(GitError::new(format!("branch already exists: {name}")));
        }
        self.snapshot.branches.push(BranchEntry {
            name: name.to_string(),
            is_current: false,
        });
        Ok(())
    }

    fn checkout_branch(&mut self, name: &str, auto_stash: bool) -> Result<(), GitError> {
        if auto_stash {
            self.operations.push("auto-stash-push".to_string());
        }
        self.operations.push(format!("checkout-branch:{name}"));
        if !self
            .snapshot
            .branches
            .iter()
            .any(|branch| branch.name == name)
        {
            return Err(GitError::new(format!("branch not found: {name}")));
        }
        for branch in &mut self.snapshot.branches {
            branch.is_current = branch.name == name;
        }
        self.snapshot.current_branch = name.to_string();
        self.snapshot.detached_head = false;
        if auto_stash {
            self.operations.push("auto-stash-pop".to_string());
        }
        Ok(())
    }

    fn delete_branch(
        &mut self,
        name: &str,
        mode: BranchDeleteMode,
        force: bool,
    ) -> Result<(), GitError> {
        match mode {
            BranchDeleteMode::Local => self.delete_local_branch(name, force),
            BranchDeleteMode::Remote => {
                self.operations.push(format!("delete-remote:origin/{name}"));
                Ok(())
            }
            BranchDeleteMode::Both => {
                self.delete_local_branch(name, force)?;
                self.operations.push(format!("delete-remote:origin/{name}"));
                Ok(())
            }
        }
    }

    fn checkout_commit_detached(
        &mut self,
        commit_id: &str,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        if auto_stash {
            self.operations.push("auto-stash-push".to_string());
        }
        self.operations
            .push(format!("checkout-detached:{commit_id}"));
        if !self
            .snapshot
            .commits
            .iter()
            .any(|commit| commit_matches(commit, commit_id))
        {
            return Err(GitError::new(format!("commit not found: {commit_id}")));
        }
        for branch in &mut self.snapshot.branches {
            branch.is_current = false;
        }
        self.snapshot.current_branch = commit_id.to_string();
        self.snapshot.detached_head = true;
        if auto_stash {
            self.operations.push("auto-stash-pop".to_string());
        }
        Ok(())
    }

    fn stash_push(&mut self, message: &str) -> Result<(), GitError> {
        self.operations.push(format!("stash-push:{message}"));
        let clean_message = clean_stash_message(message);
        self.snapshot.stashes.insert(
            0,
            StashEntry {
                id: "stash@{0}".to_string(),
                summary: clean_message,
            },
        );
        resequence_stashes(&mut self.snapshot.stashes);
        Ok(())
    }

    fn pull(&mut self) -> Result<(), GitError> {
        self.operations.push("pull".to_string());
        Ok(())
    }

    fn push(&mut self, force: bool) -> Result<(), GitError> {
        self.operations.push(if force {
            "force-push".to_string()
        } else {
            "push".to_string()
        });
        Ok(())
    }

    fn stash_files(&mut self, message: &str, paths: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("stash-files:{message}:{}", paths.join(",")));
        self.snapshot
            .files
            .retain(|entry| !paths.contains(&entry.path));
        self.snapshot.stashes.insert(
            0,
            StashEntry {
                id: "stash@{0}".to_string(),
                summary: clean_stash_message(message),
            },
        );
        resequence_stashes(&mut self.snapshot.stashes);
        Ok(())
    }

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError> {
        self.operations.push(format!("stash-pop:{stash_id}"));
        let index = self
            .snapshot
            .stashes
            .iter()
            .position(|entry| entry.id == stash_id)
            .ok_or_else(|| GitError::new(format!("stash not found: {stash_id}")))?;
        self.snapshot.stashes.remove(index);
        resequence_stashes(&mut self.snapshot.stashes);
        Ok(())
    }

    fn reset(&mut self, mode: ResetMode) -> Result<(), GitError> {
        self.operations
            .push(format!("reset:{}", reset_mode_name(mode)));
        match mode {
            ResetMode::Soft => {}
            ResetMode::Mixed => {
                for entry in &mut self.snapshot.files {
                    if !entry.untracked {
                        entry.staged = false;
                    }
                }
            }
            ResetMode::Hard => {
                self.snapshot.files.retain(|entry| entry.untracked);
            }
        }
        refresh_status_summary(&mut self.snapshot);
        Ok(())
    }

    fn nuke(&mut self) -> Result<(), GitError> {
        self.operations.push("nuke".to_string());
        self.snapshot.files.clear();
        refresh_status_summary(&mut self.snapshot);
        Ok(())
    }

    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("discard-files:{}", paths.join(",")));
        self.snapshot
            .files
            .retain(|entry| !paths.contains(&entry.path));
        Ok(())
    }
}

impl GitBackendHistoryRewrite for MockGitBackend {
    fn rebase_branch(
        &mut self,
        target: &str,
        interactive: bool,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        if auto_stash {
            self.operations.push("auto-stash-push".to_string());
        }
        let mode = if interactive { "interactive" } else { "simple" };
        self.operations.push(format!("rebase:{mode}:{target}"));
        if auto_stash {
            self.operations.push("auto-stash-pop".to_string());
        }
        Ok(())
    }

    fn squash_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("squash:{}", commit_ids.join(",")));
        self.merge_selected_commits_into_parents(commit_ids, true)
    }

    fn fixup_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("fixup:{}", commit_ids.join(",")));
        self.merge_selected_commits_into_parents(commit_ids, false)
    }

    fn reword_commit(&mut self, commit_id: &str, message: &str) -> Result<(), GitError> {
        self.operations
            .push(format!("reword:{commit_id}:{message}"));
        let commit = self
            .snapshot
            .commits
            .iter_mut()
            .find(|commit| commit_matches(commit, commit_id))
            .ok_or_else(|| GitError::new(format!("commit not found: {commit_id}")))?;
        if commit.hash_status != CommitHashStatus::Unpushed {
            return Err(GitError::new("commit is not private"));
        }
        if commit.is_merge {
            return Err(GitError::new("merge commits are not supported"));
        }
        commit.summary = message.lines().next().unwrap_or("").trim().to_string();
        commit.message = message.trim_end().to_string();
        Ok(())
    }

    fn delete_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("delete-commits:{}", commit_ids.join(",")));
        if self.snapshot.commits.iter().any(|commit| {
            commit_ids.iter().any(|id| commit_matches(commit, id))
                && commit.hash_status != CommitHashStatus::Unpushed
        }) {
            return Err(GitError::new("commit is not private"));
        }
        if self
            .snapshot
            .commits
            .iter()
            .any(|commit| commit_ids.iter().any(|id| commit_matches(commit, id)) && commit.is_merge)
        {
            return Err(GitError::new("merge commits are not supported"));
        }
        if self
            .snapshot
            .commits
            .last()
            .is_some_and(|commit| commit_ids.iter().any(|id| commit_matches(commit, id)))
        {
            return Err(GitError::new("cannot rewrite root commit"));
        }
        let before = self.snapshot.commits.len();
        self.snapshot
            .commits
            .retain(|commit| !commit_ids.iter().any(|id| commit_matches(commit, id)));
        if self.snapshot.commits.len() == before {
            return Err(GitError::new("commit not found"));
        }
        Ok(())
    }
}

impl MockGitBackend {
    fn delete_local_branch(&mut self, name: &str, force: bool) -> Result<(), GitError> {
        let operation = if force {
            format!("force-delete-local:{name}")
        } else {
            format!("delete-local:{name}")
        };
        self.operations.push(operation);
        if self.snapshot.current_branch == name {
            return Err(GitError::new(format!(
                "cannot delete current branch: {name}"
            )));
        }
        let index = self
            .snapshot
            .branches
            .iter()
            .position(|branch| branch.name == name)
            .ok_or_else(|| GitError::new(format!("branch not found: {name}")))?;
        self.snapshot.branches.remove(index);
        Ok(())
    }
}

fn clean_stash_message(message: &str) -> String {
    if message.trim().is_empty() {
        "WIP".to_string()
    } else {
        message.to_string()
    }
}

fn reset_mode_name(mode: ResetMode) -> &'static str {
    match mode {
        ResetMode::Mixed => "mixed",
        ResetMode::Soft => "soft",
        ResetMode::Hard => "hard",
    }
}

fn refresh_status_summary(snapshot: &mut RepoSnapshot) {
    let staged = snapshot
        .files
        .iter()
        .filter(|entry| entry.staged && !entry.untracked)
        .count();
    let unstaged = snapshot
        .files
        .iter()
        .filter(|entry| !entry.staged || entry.untracked)
        .count();
    snapshot.status_summary = format!("staged: {staged}, unstaged: {unstaged}");
}
