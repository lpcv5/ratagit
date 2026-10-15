//! Branch operations and the primary-probe/external-fetch sequence.

use super::{Git2RepoInner, Git2Repository};
use crate::git::{BranchInfo, GitError};
use std::sync::mpsc::Receiver;

pub(super) fn parse_remote_from_upstream(upstream: &str) -> Option<String> {
    upstream
        .split('/')
        .next()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

impl Git2RepoInner {
    fn checkout_branch_in_repo(repo: &git2::Repository, name: &str) -> Result<(), GitError> {
        let local_ref = format!("refs/heads/{}", name);
        repo.find_reference(&local_ref)?;
        repo.set_head(&local_ref)?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.safe();
        repo.checkout_head(Some(&mut checkout))?;
        Ok(())
    }

    fn upstream_remote_name(&self) -> Option<String> {
        let head = self.repo.head().ok()?;
        let local_name = head.shorthand()?;
        let local = self
            .repo
            .find_branch(local_name, git2::BranchType::Local)
            .ok()?;
        let upstream = local.upstream().ok()?;
        let name = upstream.get().name()?;
        let trimmed = name.strip_prefix("refs/remotes/").unwrap_or(name);
        parse_remote_from_upstream(trimmed)
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let head_name = self
            .repo
            .head()
            .ok()
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

    fn create_branch(&self, name: &str) -> Result<(), GitError> {
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(name, &head, false)?;
        Ok(())
    }

    fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        Self::checkout_branch_in_repo(&self.repo, name)
    }

    fn checkout_branch_with_auto_stash(&mut self, name: &str) -> Result<(), GitError> {
        let has_stashed = {
            let mut opts = git2::StatusOptions::new();
            opts.include_untracked(true)
                .include_ignored(false)
                .include_unmodified(false);
            let statuses = self.repo.statuses(Some(&mut opts))?;
            let has_changes = !statuses.is_empty();
            drop(statuses);
            if !has_changes {
                false
            } else {
                let sig = self
                    .repo
                    .signature()
                    .or_else(|_| git2::Signature::now("ratagit", "ratagit@localhost"))?;
                let _ = self.repo.stash_save(
                    &sig,
                    "ratagit:auto-stash-before-switch",
                    Some(git2::StashFlags::INCLUDE_UNTRACKED),
                )?;
                true
            }
        };

        if let Err(err) = Self::checkout_branch_in_repo(&self.repo, name) {
            if has_stashed {
                let _ = self.repo.stash_apply(0, None);
                let _ = self.repo.stash_drop(0);
            }
            return Err(err);
        }

        if has_stashed {
            self.repo.stash_apply(0, None)?;
            self.repo.stash_drop(0)?;
        }
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let mut branch = self.repo.find_branch(name, git2::BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }
}

impl crate::git::ReferenceRepository for Git2Repository {
    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        self.spawn_repo_job("branches", |inner| inner.branches())?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn branches_async(&self) -> Result<Receiver<Result<Vec<BranchInfo>, GitError>>, GitError> {
        self.spawn_repo_job("branches", |inner| inner.branches())
    }

    fn create_branch(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("create_branch", move |inner| inner.create_branch(&name))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn create_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("create_branch", move |inner| inner.create_branch(&name))
    }

    fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("checkout_branch", move |inner| inner.checkout_branch(&name))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn checkout_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("checkout_branch", move |inner| inner.checkout_branch(&name))
    }

    fn checkout_branch_with_auto_stash(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("checkout_branch_with_auto_stash", move |inner| {
            inner.checkout_branch_with_auto_stash(&name)
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn checkout_branch_with_auto_stash_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("checkout_branch_with_auto_stash", move |inner| {
            inner.checkout_branch_with_auto_stash(&name)
        })
    }

    fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("delete_branch", move |inner| inner.delete_branch(&name))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn delete_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("delete_branch", move |inner| inner.delete_branch(&name))
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let upstream_result = self.spawn_repo_job("fetch_upstream_probe", |inner| {
            Ok(inner.upstream_remote_name())
        });
        let remote = match upstream_result.and_then(|rx| {
            rx.recv()
                .map_err(|_| GitError::Git2("worker disconnected".to_string()))
        }) {
            Ok(Ok(Some(name))) => name,
            _ => "origin".to_string(),
        };

        self.spawn_repo_job_external("fetch", move |inner| {
            let mut remote_obj = inner.repo.find_remote(&remote)?;
            let mut opts = git2::FetchOptions::new();
            opts.prune(git2::FetchPrune::On);
            remote_obj.fetch(&[] as &[&str], Some(&mut opts), None)?;
            Ok(remote)
        })
    }
}
