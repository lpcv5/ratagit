use ratagit_core::{BranchEntry, CommitEntry, RepoSnapshot, StashEntry};

use crate::{GitBackend, GitError, resequence_stashes};

#[derive(Debug, Clone)]
pub struct MockGitBackend {
    snapshot: RepoSnapshot,
    operations: Vec<String>,
    commit_sequence: u64,
}

impl MockGitBackend {
    pub fn new(snapshot: RepoSnapshot) -> Self {
        Self {
            snapshot,
            operations: Vec::new(),
            commit_sequence: 1,
        }
    }

    pub fn operations(&self) -> &[String] {
        &self.operations
    }

    pub fn snapshot(&self) -> &RepoSnapshot {
        &self.snapshot
    }
}

impl GitBackend for MockGitBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        self.operations.push("refresh".to_string());
        Ok(self.snapshot.clone())
    }

    fn files_details_diff(&mut self, paths: &[String]) -> Result<String, GitError> {
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

    fn stage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.operations.push(format!("stage:{path}"));
        let entry = self
            .snapshot
            .files
            .iter_mut()
            .find(|entry| entry.path == path)
            .ok_or_else(|| GitError::new(format!("file not found: {path}")))?;
        entry.staged = true;
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
                summary,
            },
        );
        self.commit_sequence = self.commit_sequence.saturating_add(1);
        self.snapshot.files.retain(|entry| !entry.staged);
        Ok(())
    }

    fn create_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.operations.push(format!("create-branch:{name}"));
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

    fn checkout_branch(&mut self, name: &str) -> Result<(), GitError> {
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

    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.operations
            .push(format!("discard-files:{}", paths.join(",")));
        self.snapshot
            .files
            .retain(|entry| !paths.contains(&entry.path));
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
