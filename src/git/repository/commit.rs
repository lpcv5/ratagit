//! Commit creation on the primary worker.

use super::{Git2RepoInner, Git2Repository};
use crate::git::GitError;
use std::sync::mpsc::Receiver;

impl Git2RepoInner {
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
            Err(_) => self
                .repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?,
        };

        Ok(commit_id.to_string())
    }
}

impl crate::git::CommitRepository for Git2Repository {
    fn amend_head(&self) -> Result<String, GitError> {
        self.spawn_repo_job("amend_head", |inner| {
            if inner.repo.state() != git2::RepositoryState::Clean {
                return Err(GitError::Git2(
                    "cannot amend during a repository operation".into(),
                ));
            }
            let parent = inner.repo.head()?.peel_to_commit()?;
            let mut index = inner.repo.index()?;
            index.read(true)?;
            if index.has_conflicts() {
                return Err(GitError::Git2(
                    "cannot amend with unresolved conflicts".into(),
                ));
            }
            let tree_id = index.write_tree()?;
            let tree = inner.repo.find_tree(tree_id)?;
            let committer = inner.signature()?;
            let oid = parent.amend(
                Some("HEAD"),
                None,
                Some(&committer),
                None,
                None,
                Some(&tree),
            )?;
            Ok(oid.to_string())
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".into()))?
    }

    fn commit(&self, message: &str) -> Result<String, GitError> {
        let message = message.to_string();
        self.spawn_repo_job("commit", move |inner| inner.commit(&message))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn commit_async(
        &self,
        message: String,
    ) -> Result<Receiver<Result<String, GitError>>, GitError> {
        self.spawn_repo_job("commit", move |inner| inner.commit(&message))
    }
}
