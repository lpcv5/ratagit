//! Repository workflow observations and a pure, generation-aware command state machine.
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Merge,
    Rebase { interactive: bool },
    CherryPick,
    Revert,
    ApplyMailbox,
    Bisect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum OperationControl {
    Continue,
    Skip,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationProgress {
    pub current: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryOperation {
    pub kind: Option<OperationKind>,
    pub conflicts: Vec<PathBuf>,
    pub progress: Option<OperationProgress>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryPhase {
    Idle,
    InProgress,
    Conflicted,
}

impl RepositoryOperation {
    pub fn phase(&self) -> RepositoryPhase {
        if !self.conflicts.is_empty() {
            RepositoryPhase::Conflicted
        } else if self.kind.is_some() {
            RepositoryPhase::InProgress
        } else {
            RepositoryPhase::Idle
        }
    }

    pub fn allows(&self, control: OperationControl) -> bool {
        use OperationControl::*;
        match (self.kind, control) {
            (Some(OperationKind::Bisect), _) | (None, _) => false,
            (Some(OperationKind::Merge), Skip) => false,
            (_, Continue) => self.phase() != RepositoryPhase::Conflicted,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OperationCommandState {
    #[default]
    Ready,
    Running {
        id: u64,
        control: OperationControl,
    },
    Failed {
        id: u64,
        error: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct OperationMachine {
    pub observed: RepositoryOperation,
    pub command: OperationCommandState,
    pub epoch: u64,
}

impl OperationMachine {
    pub fn observe(&mut self, epoch: u64, observed: RepositoryOperation) -> bool {
        if epoch != self.epoch || matches!(self.command, OperationCommandState::Running { .. }) {
            return false;
        }
        self.observed = observed;
        true
    }

    pub fn begin(&mut self, control: OperationControl) -> Result<u64, String> {
        if matches!(self.command, OperationCommandState::Running { .. }) {
            return Err("repository operation is already running".into());
        }
        if !self.observed.allows(control) {
            return Err("operation is unavailable in the current repository state".into());
        }
        self.epoch += 1;
        self.command = OperationCommandState::Running {
            id: self.epoch,
            control,
        };
        Ok(self.epoch)
    }

    pub fn finish(
        &mut self,
        id: u64,
        result: Result<(), String>,
        observed: Option<RepositoryOperation>,
    ) -> bool {
        if !matches!(self.command, OperationCommandState::Running { id: pending, .. } if pending == id)
        {
            return false;
        }
        if let Some(observed) = observed {
            self.observed = observed;
        }
        self.epoch += 1;
        self.command = match result {
            Ok(()) => OperationCommandState::Ready,
            Err(error) => OperationCommandState::Failed { id, error },
        };
        true
    }
}

/// An editable plan model; execution/editor UI can be added independently.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseAction {
    Pick,
    Reword,
    Edit,
    Squash,
    Fixup,
    Drop,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseStep {
    pub action: RebaseAction,
    pub oid: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebasePlan {
    pub onto: String,
    pub steps: Vec<RebaseStep>,
}

#[allow(dead_code)]
impl RebasePlan {
    pub fn validate(&self) -> Result<(), String> {
        let valid_oid = |s: &str| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit());
        if !valid_oid(&self.onto) || self.steps.is_empty() {
            return Err("invalid rebase base or empty plan".into());
        }
        let mut seen = std::collections::HashSet::new();
        let mut has_previous = false;
        for step in &self.steps {
            if !valid_oid(&step.oid) || !seen.insert(step.oid.to_ascii_lowercase()) {
                return Err("invalid or duplicate commit in rebase plan".into());
            }
            match step.action {
                RebaseAction::Squash | RebaseAction::Fixup if !has_previous => {
                    return Err("squash/fixup needs a preceding retained commit".into())
                }
                RebaseAction::Drop => {}
                _ => has_previous = true,
            }
        }
        Ok(())
    }
}
