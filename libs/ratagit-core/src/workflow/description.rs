use super::*;

impl Operation {
    pub fn description(&self) -> String {
        match self {
            Self::CustomPatch { patch, action } => format!(
                "{action:?} custom patch: {} file(s) from {}",
                patch.files.len(),
                patch.oid
            ),
            Self::CommitFileCheckout { oid, paths } => {
                format!("Checkout {} file(s) from {oid}", paths.len())
            }
            Self::CommitFileDiscard { oid, paths } => format!(
                "Remove {} file(s) of changes from commit {oid}",
                paths.len()
            ),
            Self::CommitPatch { ids, action, .. } => {
                format!("{action:?} {} selected changed line(s)", ids.len())
            }
            Self::Stash(mode) => format!("Stash {mode:?} changes"),
            Self::Resolve { paths, side } => {
                format!("Resolve {} conflicted file(s) using {side:?}", paths.len())
            }
            Self::Sequence(action) => format!("{action:?} the current Git operation"),
            Self::Reset { target, mode } => format!("Reset current branch to {target} ({mode:?})"),
            Self::Merge { target, squash } => format!(
                "{} {target} into current branch",
                if *squash { "Squash merge" } else { "Merge" }
            ),
            Self::Rename { old, new } => format!("Rename branch {old} to {new}"),
            Self::Upstream { branch, target } => format!(
                "Set upstream of {branch} to {}",
                target.as_deref().unwrap_or("none")
            ),
            Self::FastForward { branch } => format!("Fast-forward {branch} from its upstream"),
            Self::Checkout { name } => format!("Checkout {name}"),
            Self::CreateBranch { name, target } => format!("Create branch {name} from {target}"),
            Self::CherryPick(ids) => format!("Cherry-pick {} commit(s)", ids.len()),
            Self::Revert(id) => format!("Revert commit {id}"),
            Self::MoveCommit { id, up } => format!(
                "Move commit {id} {} one position",
                if *up { "up" } else { "down" }
            ),
            Self::Fixup(id) => format!("Create fixup commit for {id}"),
            Self::Autosquash(id) => format!("Autosquash fixup commits from {id}"),
            Self::Tag { name, target } => format!("Create tag {name} at {target}"),
            Self::Commit { .. } => "Create commit without hooks".into(),
        }
    }
    pub fn consequence(&self) -> &'static str {
        match self {
            Self::CustomPatch {
                action: super::commit_files::PatchAction::Discard,
                ..
            } => {
                "The accumulated patch will be removed from its original commit and descendants replayed. Success clears the custom patch. Requires a clean worktree and linear history; replay conflicts leave HEAD unchanged."
            }
            Self::CustomPatch { .. } => {
                "The entire accumulated patch will be applied to the worktree. The index and commit history are preserved; conflicting patches are rejected without partial application."
            }
            Self::CommitFileCheckout { .. } => {
                "Selected worktree files will be replaced. The index is preserved; locally modified paths are rejected."
            }
            Self::CommitFileDiscard { .. }
            | Self::CommitPatch {
                action: super::commit_files::PatchAction::Discard,
                ..
            } => {
                "The original commit and its descendants will be rewritten. Success clears any accumulated custom patch. Requires a clean worktree and linear current-branch history; replay conflicts leave HEAD unchanged."
            }
            Self::CommitPatch { .. } => {
                "Selected changes will be applied to the worktree. The index and commit history are preserved; conflicting patches are rejected."
            }
            Self::Reset {
                mode: ResetMode::Hard,
                ..
            } => "Tracked worktree and staged changes will be replaced.",
            Self::Resolve { .. } => {
                "Selected files will be replaced and staged; manual conflict edits will be lost."
            }
            Self::Sequence(SequenceAction::Abort) => {
                "Resolution work will be discarded; Git restores the operation's starting state."
            }
            _ => "Commit history will be rewritten. Resolve conflicts with M and continue with m.",
        }
    }
}
