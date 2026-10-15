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
    fn commit(&self, message: &str) -> Result<String, GitError> {
        let message = message.to_string();
        self.spawn_repo_job("commit", move |inner| inner.commit(&message))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commit_async(
        &self,
        message: String,
    ) -> Result<Receiver<Result<String, GitError>>, GitError> {
        self.spawn_repo_job("commit", move |inner| inner.commit(&message))
    }
}
