use super::operation::*;
use super::GitError;
use git2::{Repository, RepositoryState};

pub(super) fn snapshot(repo: &Repository) -> Result<RepositoryOperation, GitError> {
    let kind = match repo.state() {
        RepositoryState::Clean => None,
        RepositoryState::Merge => Some(OperationKind::Merge),
        RepositoryState::Revert | RepositoryState::RevertSequence => Some(OperationKind::Revert),
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence => {
            Some(OperationKind::CherryPick)
        }
        RepositoryState::Rebase => Some(OperationKind::Rebase { interactive: false }),
        RepositoryState::RebaseInteractive => Some(OperationKind::Rebase { interactive: true }),
        RepositoryState::RebaseMerge => Some(OperationKind::Rebase {
            interactive: repo.path().join("rebase-merge/interactive").exists(),
        }),
        RepositoryState::ApplyMailbox => Some(OperationKind::ApplyMailbox),
        RepositoryState::ApplyMailboxOrRebase => Some(OperationKind::Rebase { interactive: false }),
        RepositoryState::Bisect => Some(OperationKind::Bisect),
    };
    let mut conflicts = Vec::new();
    let mut index = repo.index()?;
    // CLI controls and other Git clients can replace the on-disk index.
    index.read(true)?;
    for entry in index.conflicts()? {
        let entry = entry?;
        for side in [entry.ancestor, entry.our, entry.their]
            .into_iter()
            .flatten()
        {
            #[cfg(unix)]
            let path = {
                use std::os::unix::ffi::OsStringExt;
                std::ffi::OsString::from_vec(side.path).into()
            };
            #[cfg(not(unix))]
            let path = std::path::PathBuf::from(
                String::from_utf8(side.path)
                    .map_err(|_| GitError::Git2("conflict path is not UTF-8".into()))?,
            );
            conflicts.push(path);
        }
    }
    conflicts.sort();
    conflicts.dedup();
    let number = |name: &str| {
        std::fs::read_to_string(repo.path().join(name))
            .ok()?
            .trim()
            .parse::<usize>()
            .ok()
    };
    let progress = if matches!(
        kind,
        Some(OperationKind::Rebase { .. } | OperationKind::ApplyMailbox)
    ) {
        number("rebase-merge/msgnum")
            .zip(number("rebase-merge/end"))
            .or_else(|| number("rebase-apply/next").zip(number("rebase-apply/last")))
            .filter(|(current, total)| *total > 0 && *current <= *total)
            .map(|(current, total)| OperationProgress { current, total })
    } else {
        None
    };
    Ok(RepositoryOperation {
        kind,
        conflicts,
        progress,
    })
}

pub(super) fn arguments(
    operation: &RepositoryOperation,
    control: OperationControl,
) -> Result<Vec<String>, GitError> {
    if !operation.allows(control) {
        return Err(GitError::InvalidState);
    }
    let command = match operation.kind.ok_or(GitError::InvalidState)? {
        OperationKind::Merge => "merge",
        OperationKind::Rebase { .. } => "rebase",
        OperationKind::CherryPick => "cherry-pick",
        OperationKind::Revert => "revert",
        OperationKind::ApplyMailbox => "am",
        OperationKind::Bisect => return Err(GitError::InvalidState),
    };
    let flag = match control {
        OperationControl::Continue => "--continue",
        OperationControl::Skip => "--skip",
        OperationControl::Abort => "--abort",
    };
    Ok(vec![command.into(), flag.into()])
}
