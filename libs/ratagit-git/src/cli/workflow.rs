use super::*;
use ratagit_core::workflow::{ConflictSide, Operation, SequenceAction, StashMode};

impl GitCli {
    pub(crate) fn remote_branch_names(&self) -> Result<Vec<String>, GitError> {
        let output = self.run_git_read_owned(vec![
            "for-each-ref".into(),
            "--format=%(refname:short) %(symref)".into(),
            "refs/remotes/".into(),
        ])?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let (name, symref) = line.split_once(' ')?;
                symref.trim().is_empty().then(|| name.to_string())
            })
            .collect())
    }

    pub(crate) fn workflow(&mut self, operation: &Operation) -> Result<(), GitError> {
        use Operation as O;
        match operation {
            O::Stash(mode) => {
                let flag = match mode {
                    StashMode::All => "--include-untracked",
                    StashMode::Staged => "--staged",
                    StashMode::Unstaged => "--keep-index",
                };
                self.run_git(&["stash", "push", flag]).map(|_| ())
            }
            O::Resolve { paths, side } => self.resolve_conflicts(paths, *side),
            O::Sequence(action) => self.sequence(*action),
            O::Reset { target, mode } => {
                let oid = self.resolve_commit(target)?;
                let flag = match mode {
                    ResetMode::Soft => "--soft",
                    ResetMode::Mixed => "--mixed",
                    ResetMode::Hard => "--hard",
                };
                self.run_git(&["reset", flag, &oid]).map(|_| ())
            }
            O::Merge { target, squash } => {
                self.ensure_clean_worktree()?;
                let oid = self.resolve_commit(target)?;
                self.run_git(&[
                    "merge",
                    if *squash { "--squash" } else { "--no-edit" },
                    &oid,
                ])
                .map(|_| ())
            }
            O::Rename { old, new } => {
                self.validate_branch(old)?;
                self.validate_branch(new)?;
                self.run_git(&["branch", "-m", old, new]).map(|_| ())
            }
            O::Upstream { branch, target } => {
                self.validate_branch(branch)?;
                match target {
                    Some(target) => {
                        self.validate_ref(target)?;
                        self.run_git(&["branch", &format!("--set-upstream-to={target}"), branch])
                            .map(|_| ())
                    }
                    None => self
                        .run_git(&["branch", "--unset-upstream", branch])
                        .map(|_| ()),
                }
            }
            O::FastForward { branch } => self.fast_forward(branch),
            O::Checkout { name } => {
                if name != "-" {
                    self.validate_branch(name)?;
                }
                self.run_git(&["switch", "--", name]).map(|_| ())
            }
            O::CreateBranch { name, target } => {
                self.validate_branch(name)?;
                let oid = self.resolve_commit(target)?;
                if self
                    .remote_branch_names()?
                    .iter()
                    .any(|remote| remote == target)
                {
                    self.ensure_clean_worktree()?;
                    self.run_git(&["switch", "--track", "-c", name, target])
                        .map(|_| ())
                } else {
                    self.run_git(&["branch", name, &oid]).map(|_| ())
                }
            }
            O::CherryPick(ids) => {
                self.ensure_clean_worktree()?;
                if ids.is_empty() {
                    return Err(GitError::new("no commits copied"));
                }
                let mut args = vec!["cherry-pick".to_string()];
                for id in ids {
                    args.push(self.resolve_commit(id)?);
                }
                self.run_git_owned(args).map(|_| ())
            }
            O::Revert(id) => {
                self.ensure_clean_worktree()?;
                let oid = self.resolve_commit(id)?;
                self.run_git(&["revert", "--no-edit", &oid]).map(|_| ())
            }
            O::MoveCommit { id, up } => self.move_commit(id, *up),
            O::Fixup(id) => {
                let oid = self.resolve_commit(id)?;
                self.ensure_staged()?;
                self.run_git(&["commit", &format!("--fixup={oid}")])
                    .map(|_| ())
            }
            O::Autosquash(id) => {
                self.ensure_clean_worktree()?;
                let oid = self.resolve_commit(id)?;
                self.run_git(&["merge-base", "--is-ancestor", &oid, "HEAD"])?;
                if self.parent_count(&oid)? != 1 {
                    return Err(GitError::new(
                        "autosquash requires a non-root, non-merge base commit",
                    ));
                }
                self.ensure_linear_range(&oid)?;
                self.run_git_owned_with_env(
                    vec![
                        "rebase".into(),
                        "-i".into(),
                        "--autosquash".into(),
                        format!("{oid}^"),
                    ],
                    vec![
                        ("GIT_SEQUENCE_EDITOR".into(), ":".into()),
                        ("GIT_EDITOR".into(), ":".into()),
                    ],
                )
                .map(|_| ())
            }
            O::Tag { name, target } => {
                self.validate_ref(&format!("refs/tags/{name}"))?;
                let oid = self.resolve_commit(target)?;
                self.run_git(&["tag", "--", name, &oid]).map(|_| ())
            }
            O::Commit {
                message,
                skip_hooks,
            } => {
                if message.trim().is_empty() {
                    return Err(GitError::new("commit message is required"));
                }
                self.ensure_staged()?;
                let mut args = vec!["commit", "-m", message];
                if *skip_hooks {
                    args.push("--no-verify");
                }
                self.run_git(&args).map(|_| ())
            }
        }
    }

    fn validate_ref(&self, name: &str) -> Result<(), GitError> {
        if name.is_empty() || name.starts_with('-') {
            return Err(GitError::new("invalid ref name"));
        }
        self.run_git(&["check-ref-format", "--allow-onelevel", name])
            .map(|_| ())
    }

    fn validate_branch(&self, name: &str) -> Result<(), GitError> {
        if name.is_empty() || name.starts_with('-') {
            return Err(GitError::new("invalid branch name"));
        }
        self.run_git(&["check-ref-format", "--branch", name])
            .map(|_| ())
    }

    fn ensure_staged(&self) -> Result<(), GitError> {
        let output = self.run_git(&["diff", "--cached", "--name-only"])?;
        if output.trim().is_empty() {
            Err(GitError::new("stage changes before committing"))
        } else {
            Ok(())
        }
    }

    fn resolve_conflicts(&self, paths: &[String], side: ConflictSide) -> Result<(), GitError> {
        if paths.is_empty() {
            return Err(GitError::new("no conflicts selected"));
        }
        let repo = git2::Repository::open(&self.repo_path)?;
        let index = repo.index()?;
        let mut conflicts = std::collections::BTreeMap::new();
        for conflict in index.conflicts()? {
            let conflict = conflict?;
            if let Some(entry) = conflict
                .our
                .as_ref()
                .or(conflict.their.as_ref())
                .or(conflict.ancestor.as_ref())
            {
                conflicts.insert(
                    String::from_utf8_lossy(&entry.path).into_owned(),
                    (conflict.our.is_some(), conflict.their.is_some()),
                );
            }
        }
        // Validate the entire selection before changing any file.
        for path in paths {
            validate_repo_relative_path(path)?;
            if !conflicts.contains_key(path) {
                return Err(GitError::new(format!("not conflicted: {path}")));
            }
            if side == ConflictSide::Resolved {
                let file = self.repo_path.join(path);
                let marker_size = repo
                    .get_attr(
                        Path::new(path),
                        "conflict-marker-size",
                        git2::AttrCheckFlags::empty(),
                    )?
                    .and_then(|size| size.parse::<usize>().ok())
                    .filter(|size| *size > 0)
                    .unwrap_or(7);
                if let Ok(bytes) = fs::read(file)
                    && bytes
                        .split(|b| *b == b'\n')
                        .any(|line| is_conflict_marker(line, marker_size))
                {
                    return Err(GitError::new(format!("conflict markers remain in {path}")));
                }
            }
        }
        for path in paths {
            let pathspec = format!(":(literal){path}");
            let present = match side {
                ConflictSide::Ours => conflicts[path].0,
                ConflictSide::Theirs => conflicts[path].1,
                ConflictSide::Resolved => true,
            };
            if !present {
                self.run_git(&["rm", "--", &pathspec])?;
                continue;
            }
            if side != ConflictSide::Resolved {
                self.run_git(&[
                    "checkout",
                    if side == ConflictSide::Ours {
                        "--ours"
                    } else {
                        "--theirs"
                    },
                    "--",
                    &pathspec,
                ])?;
            }
            self.run_git(&["add", "--", &pathspec])?;
        }
        Ok(())
    }

    fn sequence(&self, action: SequenceAction) -> Result<(), GitError> {
        use git2::RepositoryState as S;
        let repo = git2::Repository::open(&self.repo_path)?;
        let command = match repo.state() {
            S::Merge => "merge",
            S::Rebase | S::RebaseInteractive | S::RebaseMerge => "rebase",
            S::CherryPick | S::CherryPickSequence => "cherry-pick",
            S::Revert | S::RevertSequence => "revert",
            _ => {
                return Err(GitError::new(
                    "no merge, rebase, cherry-pick or revert in progress",
                ));
            }
        };
        if command == "merge" && action == SequenceAction::Skip {
            return Err(GitError::new(
                "merge does not support skip; resolve or abort",
            ));
        }
        let flag = match action {
            SequenceAction::Continue => "--continue",
            SequenceAction::Abort => "--abort",
            SequenceAction::Skip => "--skip",
        };
        self.run_git_owned_with_env(
            vec![command.into(), flag.into()],
            vec![("GIT_EDITOR".into(), ":".into())],
        )
        .map(|_| ())
    }

    fn fast_forward(&self, branch: &str) -> Result<(), GitError> {
        self.validate_branch(branch)?;
        let repo = git2::Repository::open(&self.repo_path)?;
        let head = repo.head()?;
        let current = head.is_branch().then(|| head.shorthand()).flatten();
        let upstream = self.resolve_commit(&format!("{branch}@{{upstream}}"))?;
        if current == Some(branch) {
            self.ensure_clean_worktree()?;
            self.run_git(&["merge", "--ff-only", &upstream]).map(|_| ())
        } else {
            // fetch . updates an un-checked-out local branch with Git's own
            // fast-forward and linked-worktree protections.
            self.run_git(&["fetch", ".", &format!("{upstream}:refs/heads/{branch}")])
                .map(|_| ())
        }
    }

    fn ensure_linear_range(&self, id: &str) -> Result<(), GitError> {
        let commits = self.run_git(&["rev-list", "--merges", &format!("{id}^..HEAD")])?;
        if commits.trim().is_empty() {
            Ok(())
        } else {
            Err(GitError::new(
                "history rewrite across merge commits is not supported",
            ))
        }
    }

    fn move_commit(&self, id: &str, up: bool) -> Result<(), GitError> {
        self.ensure_clean_worktree()?;
        let id = self.resolve_commit(id)?;
        let mut history = self.rev_list_reverse_head()?;
        let position = history
            .iter()
            .position(|commit| commit == &id)
            .ok_or_else(|| GitError::new("commit is not on current branch"))?;
        let neighbor = if up {
            position
                .checked_add(1)
                .filter(|index| *index < history.len())
        } else {
            position.checked_sub(1)
        }
        .ok_or_else(|| GitError::new("commit is already at the boundary"))?;
        let first = position.min(neighbor);
        if first == 0 {
            return Err(GitError::new("moving the root commit is not supported"));
        }
        let upstream = history[first - 1].clone();
        for commit in &history[first..] {
            if self.parent_count(commit)? != 1 {
                return Err(GitError::new(
                    "moving commits across merges is not supported",
                ));
            }
        }
        history.swap(position, neighbor);
        let plan = super::InteractiveRebasePlan {
            upstream,
            // Populate the existing deterministic sequence script rather than
            // invoking a user editor in a background worker.
            todo: history[first..]
                .iter()
                .map(|commit| format!("pick {commit}"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
            todo_commits: history[first..].to_vec(),
            message_editor: super::RebaseMessageEditor::Noop,
        };
        let scripts = super::RebaseScripts::write(&self.repo_path, &plan)?;
        self.run_git_owned_with_env(
            vec!["rebase".into(), "-i".into(), plan.upstream],
            scripts.env(),
        )
        .map(|_| ())
    }
}

fn is_conflict_marker(line: &[u8], marker_size: usize) -> bool {
    let Some(marker @ (b'<' | b'>' | b'|' | b'=')) = line.first() else {
        return false;
    };
    let count = line.iter().take_while(|byte| *byte == marker).count();
    if count != marker_size {
        return false;
    }
    if *marker == b'=' {
        line[count..].iter().all(u8::is_ascii_whitespace)
    } else {
        line.get(count) == Some(&b' ')
    }
}
