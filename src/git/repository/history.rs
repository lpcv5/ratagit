//! Commit queries and per-worker reachability caches.

use super::{Git2RepoInner, Git2Repository};
use crate::git::{
    CommitInfo, CommitSyncState, DiffLine, DiffLineKind, FileEntry, FileStatus, GitError,
};
use std::collections::HashSet;

mod graph;
use graph::{build_commit_graph_lines, graph_charset_from_env, GraphCommitRow};
use std::sync::mpsc::Receiver;

#[derive(Default)]
pub(super) struct ReachabilityCache {
    default_tip: Option<git2::Oid>,
    upstream_tip: Option<git2::Oid>,
    limit: usize,
    default_set: HashSet<git2::Oid>,
    upstream_set: HashSet<git2::Oid>,
}

impl Git2RepoInner {
    fn resolve_ref_oid(&self, refname: &str) -> Option<git2::Oid> {
        let reference = self.repo.find_reference(refname).ok()?;
        reference
            .resolve()
            .ok()
            .and_then(|resolved| resolved.target())
            .or_else(|| reference.target())
    }

    fn upstream_oid(&self) -> Option<git2::Oid> {
        let head = self.repo.head().ok()?;
        let local_name = head.shorthand()?;
        let local = self
            .repo
            .find_branch(local_name, git2::BranchType::Local)
            .ok()?;
        let upstream = local.upstream().ok()?;
        upstream.get().target()
    }

    fn default_branch_oid(&self) -> Option<git2::Oid> {
        self.resolve_ref_oid("refs/remotes/origin/HEAD")
            .or_else(|| self.resolve_ref_oid("refs/heads/main"))
            .or_else(|| self.resolve_ref_oid("refs/heads/master"))
            .or_else(|| self.resolve_ref_oid("refs/remotes/origin/main"))
            .or_else(|| self.resolve_ref_oid("refs/remotes/origin/master"))
    }

    fn collect_commits_from_revwalk(
        &self,
        revwalk: &mut git2::Revwalk<'_>,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let default_tip = self.default_branch_oid();
        let upstream_tip = self.upstream_oid();
        let reachability_limit = limit * 4;

        let needs_recompute = {
            let cache = self.reachability_cache.borrow();
            cache.default_tip != default_tip
                || cache.upstream_tip != upstream_tip
                || cache.limit < reachability_limit
        };
        if needs_recompute {
            let default_set = self.reachable_oids_from(default_tip, reachability_limit);
            let upstream_set = self.reachable_oids_from(upstream_tip, reachability_limit);
            let mut cache = self.reachability_cache.borrow_mut();
            cache.default_tip = default_tip;
            cache.upstream_tip = upstream_tip;
            cache.limit = reachability_limit;
            cache.default_set = default_set;
            cache.upstream_set = upstream_set;
        }

        let cache = self.reachability_cache.borrow();
        let default_oids = &cache.default_set;
        let upstream_oids = &cache.upstream_set;

        let mut graph_rows = Vec::new();
        let mut entries = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.summary().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("").to_string();
            let mut parents = Vec::with_capacity(commit.parent_count());
            for parent_index in 0..commit.parent_count() {
                if let Ok(parent_oid) = commit.parent_id(parent_index) {
                    parents.push(parent_oid);
                }
            }
            let time = {
                let t = commit.time().seconds();
                chrono::DateTime::from_timestamp(t, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            };
            let sync_state = if default_oids.contains(&oid) {
                CommitSyncState::DefaultBranch
            } else if upstream_oids.contains(&oid) {
                CommitSyncState::RemoteBranch
            } else {
                CommitSyncState::LocalOnly
            };
            let parent_oids: Vec<String> = parents.iter().map(|p| p.to_string()).collect();
            graph_rows.push(GraphCommitRow { oid, parents });
            entries.push(CommitInfo {
                oid: oid.to_string(),
                message,
                author,
                graph: Vec::new(),
                time,
                parent_count: commit.parent_count(),
                sync_state,
                parent_oids,
            });
        }

        let graph_lines = build_commit_graph_lines(&graph_rows, graph_charset_from_env());
        for (entry, graph) in entries.iter_mut().zip(graph_lines) {
            entry.graph = graph;
        }
        Ok(entries)
    }

    fn collect_commits_fast_from_revwalk(
        &self,
        revwalk: &mut git2::Revwalk<'_>,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let mut entries = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.summary().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("").to_string();
            let mut parent_oids = Vec::with_capacity(commit.parent_count());
            for parent_index in 0..commit.parent_count() {
                if let Ok(parent_oid) = commit.parent_id(parent_index) {
                    parent_oids.push(parent_oid.to_string());
                }
            }
            entries.push(CommitInfo {
                oid: oid.to_string(),
                message,
                author,
                graph: Vec::new(),
                time: String::new(),
                parent_count: commit.parent_count(),
                sync_state: CommitSyncState::LocalOnly,
                parent_oids,
            });
        }
        Ok(entries)
    }

    /// Walk at most `limit` commits reachable from `tip` and return their OIDs as a HashSet.
    /// Returns an empty set if `tip` is None or if the walk fails.
    fn reachable_oids_from(
        &self,
        tip: Option<git2::Oid>,
        limit: usize,
    ) -> std::collections::HashSet<git2::Oid> {
        let tip = match tip {
            Some(t) => t,
            None => return std::collections::HashSet::new(),
        };
        let mut walk = match self.repo.revwalk() {
            Ok(w) => w,
            Err(_) => return std::collections::HashSet::new(),
        };
        if walk.push(tip).is_err() {
            return std::collections::HashSet::new();
        }
        let _ = walk.set_sorting(git2::Sort::TOPOLOGICAL);
        walk.take(limit).flatten().collect()
    }

    fn branch_log(&self, name: &str, limit: usize) -> Result<Vec<DiffLine>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        let local_ref = format!("refs/heads/{}", name);
        let Some(tip_oid) = self
            .resolve_ref_oid(&local_ref)
            .or_else(|| self.resolve_ref_oid(name))
        else {
            return Ok(Vec::new());
        };
        revwalk.push(tip_oid)?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

        let head_name = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let mut rows = Vec::new();
        let mut entries = Vec::new();
        for oid in revwalk.take(limit.max(1)) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let short_hash = format!("{:.7}", oid);
            let message = commit.summary().unwrap_or("").to_string();
            let mut parents = Vec::with_capacity(commit.parent_count());
            for parent_index in 0..commit.parent_count() {
                if let Ok(parent_oid) = commit.parent_id(parent_index) {
                    parents.push(parent_oid);
                }
            }
            let decoration = if oid == tip_oid {
                if head_name.as_deref() == Some(name) {
                    format!("(HEAD -> {})", name)
                } else {
                    format!("({})", name)
                }
            } else {
                String::new()
            };
            rows.push(GraphCommitRow { oid, parents });
            entries.push((short_hash, decoration, message));
        }

        let graph_lines = build_commit_graph_lines(&rows, graph_charset_from_env());
        let mut lines = Vec::new();
        for ((short_hash, decoration, message), graph) in entries.into_iter().zip(graph_lines) {
            let graph_text: String = graph.iter().map(|cell| cell.text.as_str()).collect();
            let mut line = String::new();
            line.push_str(&graph_text);
            line.push(' ');
            line.push_str(&short_hash);
            if !decoration.is_empty() {
                line.push(' ');
                line.push_str(&decoration);
            }
            if !message.is_empty() {
                line.push(' ');
                line.push_str(&message);
            }
            lines.push(DiffLine {
                kind: DiffLineKind::Context,
                content: line,
            });
        }
        Ok(lines)
    }

    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        self.collect_commits_from_revwalk(&mut revwalk, limit)
    }

    fn commits_fast(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        self.collect_commits_fast_from_revwalk(&mut revwalk, limit)
    }

    fn commits_for_branch(&self, name: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        let local_ref = format!("refs/heads/{}", name);
        if let Some(oid) = self
            .resolve_ref_oid(&local_ref)
            .or_else(|| self.resolve_ref_oid(name))
        {
            revwalk.push(oid)?;
        } else {
            return Ok(Vec::new());
        }
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        self.collect_commits_from_revwalk(&mut revwalk, limit)
    }

    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError> {
        let mut files = Vec::new();
        let oid = git2::Oid::from_str(oid)?;
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
        for delta in diff.deltas() {
            let status = match delta.status() {
                git2::Delta::Added => FileStatus::New,
                git2::Delta::Deleted => FileStatus::Deleted,
                git2::Delta::Renamed => FileStatus::Renamed,
                git2::Delta::Typechange => FileStatus::TypeChange,
                _ => FileStatus::Modified,
            };
            let path = match delta.status() {
                git2::Delta::Deleted => delta.old_file().path(),
                _ => delta.new_file().path().or_else(|| delta.old_file().path()),
            };
            if let Some(path) = path {
                files.push(FileEntry {
                    path: path.to_path_buf(),
                    status,
                });
            }
        }
        Ok(files)
    }
}

impl crate::git::HistoryRepository for Git2Repository {
    fn branch_log(&self, name: &str, limit: usize) -> Result<Vec<DiffLine>, GitError> {
        let name = name.to_string();
        self.spawn_repo_job_external("branch_log", move |inner| inner.branch_log(&name, limit))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn branch_log_async(
        &self,
        name: String,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job_external("branch_log", move |inner| inner.branch_log(&name, limit))
    }

    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.spawn_repo_job_external("commits", move |inner| inner.commits(limit))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commits_fast(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.spawn_repo_job_external("commits_fast", move |inner| inner.commits_fast(limit))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commits_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        self.spawn_repo_job_external("commits", move |inner| inner.commits(limit))
    }

    fn commits_fast_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        self.spawn_repo_job_external("commits_fast", move |inner| inner.commits_fast(limit))
    }

    fn commits_for_branch(&self, name: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let name = name.to_string();
        self.spawn_repo_job_external("commits_for_branch", move |inner| {
            inner.commits_for_branch(&name, limit)
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commits_for_branch_async(
        &self,
        name: &str,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let name = name.to_string();
        self.spawn_repo_job_external("commits_for_branch", move |inner| {
            inner.commits_for_branch(&name, limit)
        })
    }

    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError> {
        let oid = oid.to_string();
        self.spawn_repo_job("commit_files", move |inner| inner.commit_files(&oid))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }
}
