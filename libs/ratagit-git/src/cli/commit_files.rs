//! Commit-file writes stay behind the exclusive GitBackend mutation boundary.
use super::*;
use git2::{Oid, Repository, RepositoryState};
use ratagit_core::patch::{LineId, Patch};
use ratagit_core::workflow::commit_files::PatchAction;

impl GitCli {
    pub(super) fn custom_patch(
        &self,
        patch: &ratagit_core::custom_patch::CustomPatch,
        action: PatchAction,
    ) -> Result<(), GitError> {
        let mut repo = Repository::open(&self.repo_path)?;
        let bytes = crate::patch_backend::custom::checked_bytes(
            &mut repo,
            patch,
            action != PatchAction::Apply,
        )?;
        if action != PatchAction::Discard {
            return crate::patch_backend::apply_git_bytes(
                &mut repo,
                &bytes,
                false,
                action == PatchAction::Reverse,
                None,
            );
        }
        self.rewrite_commit(&patch.oid, |repo, original| {
            let temp = TemporaryFiles::new(repo)?;
            self.run_git_owned_with_env(
                vec!["read-tree".into(), original.tree_id().to_string()],
                temp.env()?,
            )?;
            crate::patch_backend::apply_git_bytes(
                repo,
                &bytes,
                true,
                true,
                Some(&temp.index_path),
            )?;
            git2::Index::open(&temp.index_path)?
                .write_tree_to(repo)
                .map_err(Into::into)
        })
    }
    pub(super) fn checkout_commit_files(
        &self,
        oid: &str,
        paths: &[String],
    ) -> Result<(), GitError> {
        let oid = self.resolve_commit(oid)?;
        validate_paths(paths)?;
        let repo = Repository::open(&self.repo_path)?;
        if repo.state() != RepositoryState::Clean {
            return Err(GitError::new("finish the current Git operation first"));
        }
        // Check both index and worktree, including untracked/ignored obstructions,
        // with Git's literal pathspecs before restoring the entire selection.
        ensure_paths_clean(&repo, paths)?;
        // --ignore-unmatch is not supported by restore. Only pass paths present
        // in the source or index; a renamed source may have an absent old path.
        let tree = repo.find_commit(Oid::from_str(&oid)?)?.tree()?;
        let index = repo.index()?;
        let selected = paths
            .iter()
            .filter(|path| {
                tree.get_path(Path::new(path)).is_ok()
                    || index.get_path(Path::new(path), 0).is_some()
            })
            .map(|path| format!(":(literal){path}\0"))
            .collect::<String>();
        if selected.is_empty() {
            return Err(GitError::new(
                "selected paths are absent from both the commit and index",
            ));
        }
        // NUL-delimited literal pathspecs preserve unusual filenames and avoid
        // Windows' command-line length limit for directory/range operations.
        let temp = TemporaryFiles::new(&repo)?;
        let pathspec = temp.dir.join("paths");
        fs::write(&pathspec, selected)?;
        self.run_git_owned(vec![
            "restore".into(),
            format!("--source={oid}"),
            "--worktree".into(),
            format!("--pathspec-from-file={}", path_to_env_value(&pathspec)?),
            "--pathspec-file-nul".into(),
        ])
        .map(|_| ())
    }

    pub(super) fn commit_patch(
        &self,
        patch: &Patch,
        ids: &[LineId],
        action: PatchAction,
    ) -> Result<(), GitError> {
        let mut repo = Repository::open(&self.repo_path)?;
        let bytes = crate::patch_backend::commit_bytes(&mut repo, patch, ids, action)?;
        if action != PatchAction::Discard {
            return crate::patch_backend::apply_git_bytes(
                &mut repo,
                &bytes,
                false,
                action == PatchAction::Reverse,
                None,
            );
        }
        let oid = match &patch.request {
            ratagit_core::patch::PatchRequest::Commit { oid, .. }
            | ratagit_core::patch::PatchRequest::CommitFiles { oid, .. } => oid,
            _ => return Err(GitError::new("expected a commit patch")),
        };
        self.rewrite_commit(oid, |repo, original| {
            let temp = TemporaryFiles::new(repo)?;
            self.run_git_owned_with_env(
                vec!["read-tree".into(), original.tree_id().to_string()],
                temp.env()?,
            )?;
            crate::patch_backend::apply_git_bytes(
                repo,
                &bytes,
                true,
                true,
                Some(&temp.index_path),
            )?;
            let mut index = git2::Index::open(&temp.index_path)?;
            index.write_tree_to(repo).map_err(Into::into)
        })
    }

    pub(super) fn discard_commit_files(&self, oid: &str, paths: &[String]) -> Result<(), GitError> {
        validate_paths(paths)?;
        self.rewrite_commit(oid, |repo, original| {
            let tree = original.tree()?;
            let parent = if original.parent_count() > 0 {
                Some(original.parent(0)?.tree()?)
            } else {
                None
            };
            let mut opts = git2::DiffOptions::new();
            opts.disable_pathspec_match(true);
            for path in paths {
                opts.pathspec(path);
            }
            // Enumerate changed tree entries, then restore their parent entries.
            // This also preserves binary bytes, modes, symlinks and gitlinks.
            let diff = repo.diff_tree_to_tree(Some(&tree), parent.as_ref(), Some(&mut opts))?;
            if diff.deltas().len() == 0 {
                return Err(GitError::new(
                    "selected files have no changes in this commit",
                ));
            }
            let changed = diff
                .deltas()
                .flat_map(|d| {
                    [
                        d.old_file().path().map(Path::to_path_buf),
                        d.new_file().path().map(Path::to_path_buf),
                    ]
                })
                .flatten()
                .collect::<BTreeSet<_>>();
            let mut index = git2::Index::new()?;
            index.read_tree(&tree)?;
            let mut parent_index = git2::Index::new()?;
            if let Some(parent) = parent {
                parent_index.read_tree(&parent)?;
            }
            for path in &changed {
                if index.get_path(path, 0).is_some() {
                    index.remove_path(path)?;
                }
            }
            for path in &changed {
                if let Some(entry) = parent_index.get_path(path, 0) {
                    index.add(&entry)?;
                }
            }
            index.write_tree_to(repo).map_err(Into::into)
        })
    }

    fn rewrite_commit(
        &self,
        oid: &str,
        edit: impl FnOnce(&mut Repository, &git2::Commit<'_>) -> Result<Oid, GitError>,
    ) -> Result<(), GitError> {
        self.ensure_clean_worktree()?;
        let mut repo = Repository::open(&self.repo_path)?;
        if repo.state() != RepositoryState::Clean || repo.head_detached()? {
            return Err(GitError::new(
                "history changes require a checked-out branch with no Git operation in progress",
            ));
        }
        let target = Oid::from_str(&self.resolve_commit(oid)?)?;
        let old_head = repo.head()?.peel_to_commit()?.id();
        let head_name = repo
            .head()?
            .name()
            .ok_or_else(|| GitError::new("invalid branch"))?
            .to_string();
        let mut history = Vec::new();
        let mut cursor = old_head;
        loop {
            let commit = repo.find_commit(cursor)?;
            if commit.parent_count() > 1 {
                return Err(GitError::new(
                    "history changes across merge commits are not supported",
                ));
            }
            if cursor == target {
                break;
            }
            history.push(cursor);
            if commit.parent_count() == 0 {
                return Err(GitError::new("commit is not on the current branch"));
            }
            cursor = commit.parent_id(0)?;
        }
        // Open a separate read handle so edit may refresh repo-owned indexes.
        let read = Repository::open(&self.repo_path)?;
        let original = read.find_commit(target)?;
        let tree_id = edit(&mut repo, &original)?;
        let mut next = copy_commit(&repo, &original, tree_id, original.parent_ids().next())?;
        for id in history.into_iter().rev() {
            let commit = repo.find_commit(id)?;
            let parent = repo.find_commit(next)?;
            let mut index = repo.cherrypick_commit(&commit, &parent, 0, None)?;
            if index.has_conflicts() {
                return Err(GitError::new(
                    "replaying a later commit conflicts; HEAD and worktree were left unchanged",
                ));
            }
            let tree = index.write_tree_to(&repo)?;
            next = copy_commit(&repo, &commit, tree, Some(next))?;
        }
        self.ensure_clean_worktree()?;
        let next_tree = repo.find_commit(next)?.tree()?;
        let old_tree = repo.find_commit(old_head)?.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&next_tree), None)?;
        let changed_paths = diff
            .deltas()
            .flat_map(|d| {
                [
                    d.old_file().path().map(Path::to_path_buf),
                    d.new_file().path().map(Path::to_path_buf),
                ]
            })
            .flatten()
            .map(|path| {
                path.to_str()
                    .map(str::to_string)
                    .ok_or_else(|| GitError::new("non-UTF-8 checkout paths are unsupported"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !changed_paths.is_empty() {
            ensure_paths_clean(&repo, &changed_paths)?;
        }
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.dry_run().overwrite_ignored(false);
        repo.checkout_tree(next_tree.as_object(), Some(&mut checkout))?;
        let head = repo.head()?;
        if head.name() != Some(head_name.as_str()) || head.target() != Some(old_head) {
            return Err(GitError::new(
                "HEAD changed during history preparation; retry",
            ));
        }
        // Git performs the final worktree/index update with its own obstruction
        // and lock checks; old history remains reachable through the reflog.
        self.run_git(&["reset", "--keep", &next.to_string()])
            .map(|_| ())
    }
}

fn copy_commit(
    repo: &Repository,
    original: &git2::Commit<'_>,
    tree: Oid,
    parent: Option<Oid>,
) -> Result<Oid, GitError> {
    let tree = repo.find_tree(tree)?;
    let parent = parent.map(|id| repo.find_commit(id)).transpose()?;
    repo.commit(
        None,
        &original.author(),
        &repo.signature()?,
        original
            .message()
            .ok_or_else(|| GitError::new("non-UTF-8 commit messages cannot be rewritten"))?,
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .map_err(Into::into)
}

fn validate_paths(paths: &[String]) -> Result<(), GitError> {
    if paths.is_empty() {
        return Err(GitError::new("no commit files selected"));
    }
    for path in paths {
        validate_repo_relative_path(path)?;
    }
    Ok(())
}

fn ensure_paths_clean(repo: &Repository, paths: &[String]) -> Result<(), GitError> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(true)
        .recurse_ignored_dirs(true)
        .disable_pathspec_match(true);
    for path in paths {
        opts.pathspec(path);
    }
    if repo.statuses(Some(&mut opts))?.is_empty() {
        Ok(())
    } else {
        Err(GitError::new(
            "selected paths have local changes or ignored obstructions; commit or stash them first",
        ))
    }
}

struct TemporaryFiles {
    index_path: PathBuf,
    dir: PathBuf,
}
impl TemporaryFiles {
    fn new(repo: &Repository) -> Result<Self, GitError> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| GitError::io(e.to_string()))?
            .as_nanos();
        let dir = repo.path().join(format!(
            "ratagit-commit-patch-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&dir)?;
        Ok(Self {
            index_path: dir.join("index"),
            dir,
        })
    }
    fn env(&self) -> Result<Vec<(String, String)>, GitError> {
        Ok(vec![(
            "GIT_INDEX_FILE".into(),
            path_to_env_value(&self.index_path)?,
        )])
    }
}
impl Drop for TemporaryFiles {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
