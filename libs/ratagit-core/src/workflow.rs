//! Reducer-owned workflows shared by the three Git panels.
pub mod commit_files;
mod description;
mod files;
mod open;
mod prompt;
pub use prompt::title as prompt_title;

use crate::{
    AppContext, Command, CommandPaletteCommand, CommandPaletteEntry, CommandPaletteSection,
    ResetMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Conflict,
    Sequence,
    Stash,
    Filter,
    ToggleTree,
    CollapseAll,
    ExpandAll,
    UpstreamReset,
    CommitNoVerify,
    CommitExternal,
    Merge,
    Rename,
    Upstream,
    FastForward,
    CheckoutByName,
    CheckoutPrevious,
    RemoteBranches,
    CopyBranch,
    CopyCommits,
    PasteCommits,
    ClearCopied,
    Revert,
    Reset,
    BranchFromCommit,
    CopyCommit,
    MoveUp,
    MoveDown,
    Fixup,
    Autosquash,
    Tag,
    CopyAttributes,
    CommitFileCheckout,
    CommitFileDiscard,
    CommitFilePatch,
    CommitFileCopy,
    CommitFilePatchAll,
    CustomPatchOptions,
    CustomPatchPreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashMode {
    All,
    Staged,
    Unstaged,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSide {
    Ours,
    Theirs,
    Resolved,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceAction {
    Continue,
    Abort,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    CustomPatch {
        patch: Box<crate::custom_patch::CustomPatch>,
        action: commit_files::PatchAction,
    },
    CommitFileCheckout {
        oid: String,
        paths: Vec<String>,
    },
    CommitFileDiscard {
        oid: String,
        paths: Vec<String>,
    },
    CommitPatch {
        patch: Box<crate::patch::Patch>,
        ids: Vec<crate::patch::LineId>,
        action: commit_files::PatchAction,
    },
    Stash(StashMode),
    Resolve {
        paths: Vec<String>,
        side: ConflictSide,
    },
    Sequence(SequenceAction),
    Reset {
        target: String,
        mode: ResetMode,
    },
    Merge {
        target: String,
        squash: bool,
    },
    Rename {
        old: String,
        new: String,
    },
    Upstream {
        branch: String,
        target: Option<String>,
    },
    FastForward {
        branch: String,
    },
    Checkout {
        name: String,
    },
    CreateBranch {
        name: String,
        target: String,
    },
    CherryPick(Vec<String>),
    Revert(String),
    MoveCommit {
        id: String,
        up: bool,
    },
    Fixup(String),
    Autosquash(String),
    Tag {
        name: String,
        target: String,
    },
    Commit {
        message: String,
        skip_hooks: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Rename,
    Upstream,
    Checkout,
    Branch,
    Tag,
    CommitNoVerify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChoiceAction {
    CustomPatch(crate::custom_patch::CustomPatchAction),
    Copy(String),
    Run(Operation),
    Prompt { kind: PromptKind, target: String },
    Filter(files::FileFilter),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    pub action: ChoiceAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    Menu {
        title: &'static str,
        choices: Vec<Choice>,
        selected: usize,
    },
    Prompt {
        kind: PromptKind,
        target: String,
        text: String,
        cursor: usize,
    },
    Confirm(Operation),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowState {
    pub dialog: Option<Dialog>,
    pub copied_commits: Vec<String>,
    pub remote_request: Option<u64>,
    pub next_request: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowAction {
    Open(Feature),
    Up,
    Down,
    Confirm,
    Cancel,
    Input(char),
    Backspace,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    ExternalCommit(String),
}

pub use files::FileFilter;

pub fn update(state: &mut AppContext, action: WorkflowAction) -> Vec<Command> {
    use WorkflowAction as A;
    match action {
        A::Open(feature) => {
            if state.work.mutation.operation_pending.is_some() {
                return Vec::new();
            }
            open::open(state, feature)
        }
        A::Cancel => {
            state.ui.workflow.dialog = None;
            state.ui.workflow.remote_request = None;
            Vec::new()
        }
        A::ExternalCommit(message) => {
            if message.trim().is_empty() {
                return Vec::new();
            }
            crate::commit_workflow::create_commit(state, message)
        }
        A::Confirm => confirm(state),
        other => {
            prompt::edit(state, other);
            Vec::new()
        }
    }
}

fn confirm(state: &mut AppContext) -> Vec<Command> {
    if state.work.mutation.operation_pending.is_some() {
        return Vec::new();
    }
    let Some(dialog) = state.ui.workflow.dialog.take() else {
        return Vec::new();
    };
    match dialog {
        Dialog::Menu {
            choices, selected, ..
        } => match choices.get(selected).map(|choice| choice.action.clone()) {
            Some(ChoiceAction::Run(operation)) => request(state, operation),
            Some(ChoiceAction::Prompt { kind, target }) => {
                prompt::open(state, kind, target);
                Vec::new()
            }
            Some(ChoiceAction::Filter(filter)) => files::filter(state, filter),
            Some(ChoiceAction::Copy(_)) => Vec::new(),
            Some(ChoiceAction::CustomPatch(action)) => crate::custom_patch::update(state, action),
            None => Vec::new(),
        },
        Dialog::Prompt {
            kind, target, text, ..
        } => prompt::confirm(state, kind, target, text),
        Dialog::Confirm(operation) => crate::extras::run(
            state,
            crate::extras::RepositoryOperation::Workflow(operation),
        ),
    }
}

fn request(state: &mut AppContext, operation: Operation) -> Vec<Command> {
    let dangerous = matches!(
        &operation,
        Operation::Reset {
            mode: ResetMode::Hard,
            ..
        } | Operation::Resolve {
            side: ConflictSide::Ours | ConflictSide::Theirs,
            ..
        } | Operation::Sequence(SequenceAction::Abort)
            | Operation::MoveCommit { .. }
            | Operation::Autosquash(_)
            | Operation::CommitFileCheckout { .. }
            | Operation::CommitFileDiscard { .. }
            | Operation::CommitPatch { .. }
            | Operation::CustomPatch { .. }
    );
    if dangerous {
        state.ui.workflow.dialog = Some(Dialog::Confirm(operation));
        Vec::new()
    } else {
        crate::extras::run(
            state,
            crate::extras::RepositoryOperation::Workflow(operation),
        )
    }
}

fn menu(state: &mut AppContext, title: &'static str, choices: Vec<Choice>) {
    state.ui.workflow.dialog = Some(Dialog::Menu {
        title,
        choices,
        selected: 0,
    });
}

fn run_choice(label: impl Into<String>, operation: Operation) -> Choice {
    Choice {
        label: label.into(),
        action: ChoiceAction::Run(operation),
    }
}

pub fn remote_result(state: &mut AppContext, request_id: u64, result: Result<Vec<String>, String>) {
    if state.ui.workflow.remote_request != Some(request_id) {
        return;
    }
    state.ui.workflow.remote_request = None;
    if state.active_left_view() != Some(crate::ActiveLeftView::BranchesList) {
        return;
    }
    match result {
        Ok(names) if !names.is_empty() => menu(
            state,
            "Remote branches",
            names
                .into_iter()
                .map(|name| Choice {
                    label: name.clone(),
                    action: ChoiceAction::Prompt {
                        kind: PromptKind::Branch,
                        target: name,
                    },
                })
                .collect(),
        ),
        Ok(_) => crate::push_notice(state, "No remote branches; fetch first"),
        Err(error) => crate::push_notice(state, &error),
    }
}

pub fn selected_commit_ids(state: &AppContext) -> Vec<String> {
    match state.active_commit_list_target() {
        Some(crate::CommitListTarget::Branch) => {
            crate::selected_commit_ids(&state.repo.branches.commits, &state.ui.branches.commits)
        }
        Some(crate::CommitListTarget::Main) => {
            crate::selected_commit_ids(&state.repo.commits.items, &state.ui.commits)
        }
        None => Vec::new(),
    }
}

pub fn palette_entries(state: &AppContext) -> Vec<CommandPaletteEntry> {
    use Feature as F;
    let mut commands = vec![
        ("m", "merge/rebase continue, abort, skip", F::Sequence),
        ("Ctrl+p", "custom patch options", F::CustomPatchOptions),
    ];
    match state.active_left_view() {
        Some(crate::ActiveLeftView::Files) => commands.extend([
            ("M", "resolve conflicts", F::Conflict),
            ("S", "stash options", F::Stash),
            ("Ctrl+b", "filter by status", F::Filter),
            ("`", "flat/tree", F::ToggleTree),
            ("-", "collapse all", F::CollapseAll),
            ("=", "expand all", F::ExpandAll),
            ("g", "reset to upstream", F::UpstreamReset),
            ("w", "commit without hooks", F::CommitNoVerify),
            ("C", "commit with external editor", F::CommitExternal),
        ]),
        Some(crate::ActiveLeftView::BranchesList) => commands.extend([
            ("M", "merge branch", F::Merge),
            ("R", "rename branch", F::Rename),
            ("u", "upstream options", F::Upstream),
            ("f", "fast-forward branch", F::FastForward),
            ("c", "checkout by name", F::CheckoutByName),
            ("-", "previous branch", F::CheckoutPrevious),
            (
                "Tab",
                "remote branches / tracking checkout",
                F::RemoteBranches,
            ),
            ("Ctrl+o", "copy branch name", F::CopyBranch),
        ]),
        Some(crate::ActiveLeftView::Commits | crate::ActiveLeftView::BranchCommits) => commands
            .extend([
                ("C", "copy commits for cherry-pick", F::CopyCommits),
                ("V", "paste commits", F::PasteCommits),
                ("Ctrl+r", "clear copied commits", F::ClearCopied),
                ("t", "revert commit", F::Revert),
                ("g", "reset to commit", F::Reset),
                ("n", "branch from commit", F::BranchFromCommit),
                ("Ctrl+o", "copy commit hash", F::CopyCommit),
                ("Ctrl+k", "move commit up", F::MoveUp),
                ("y", "copy commit attribute", F::CopyAttributes),
                ("Ctrl+j", "move commit down", F::MoveDown),
                ("F", "create fixup commit", F::Fixup),
                ("S", "autosquash fixup commits", F::Autosquash),
                ("T", "tag commit", F::Tag),
            ]),
        Some(crate::ActiveLeftView::CommitFiles | crate::ActiveLeftView::BranchCommitFiles) => {
            commands.extend([
                (
                    "c",
                    "checkout selected file versions",
                    F::CommitFileCheckout,
                ),
                (
                    "d",
                    "remove file changes from original commit",
                    F::CommitFileDiscard,
                ),
                (
                    "Space",
                    "toggle selected files in custom patch",
                    F::CommitFilePatch,
                ),
                ("y", "copy selected paths", F::CommitFileCopy),
                (
                    "a",
                    "toggle all files in custom patch",
                    F::CommitFilePatchAll,
                ),
            ])
        }
        _ => {}
    }
    commands
        .into_iter()
        .map(|(key, label, feature)| CommandPaletteEntry {
            section: if matches!(feature, F::Sequence | F::CustomPatchOptions) {
                CommandPaletteSection::Global
            } else {
                CommandPaletteSection::Local
            },
            key,
            label,
            command: CommandPaletteCommand::Workflow(feature),
        })
        .collect()
}
