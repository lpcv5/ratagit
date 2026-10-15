//! Application-owned bindings for the diff container's behavior ports.
use super::{CommandEnv, CommandId, ViewContext};
use crate::flux::action::{DomainAction, RuntimeEvent};
use crate::git::patch::PatchDirection;
use crate::ui::components::diff::{DiffBehaviorBindings, DiffIntent};

impl CommandId {
    pub(super) fn diff_intent(self) -> Option<DiffIntent> {
        Some(match self {
            Self::FocusDiff => DiffIntent::Focus,
            Self::DiffSide => DiffIntent::SwitchSide,
            Self::DiffApply => DiffIntent::ApplySelection,
            Self::DiffNextLine => DiffIntent::NextLine,
            Self::DiffPreviousLine => DiffIntent::PreviousLine,
            Self::HunkNext => DiffIntent::NextHunk,
            Self::HunkPrevious => DiffIntent::PreviousHunk,
            Self::HunkSelect => DiffIntent::ToggleSelection,
            Self::DiffClear => DiffIntent::ClearSelection,
            Self::DiffNextFile => DiffIntent::NextFile,
            Self::DiffPreviousFile => DiffIntent::PreviousFile,
            Self::DiffLineMode => DiffIntent::ToggleMode,
            Self::DiffLineRange => DiffIntent::ToggleRange,
            Self::DiffDiscard => DiffIntent::DiscardSelection,
            Self::HunkStage => DiffIntent::StageSelection,
            Self::HunkUnstage => DiffIntent::UnstageSelection,
            Self::DiffUp => DiffIntent::ScrollUp,
            Self::DiffDown => DiffIntent::ScrollDown,
            _ => return None,
        })
    }
}

impl DiffBehaviorBindings for CommandEnv<'_> {
    type Action = DomainAction;

    fn resolve_diff_intent(&self, intent: DiffIntent) -> Option<Self::Action> {
        if self.input.is_some() {
            return None;
        }
        Some(match intent {
            DiffIntent::Focus if !self.diff.interaction.focused && self.diff.can_toggle_focus() => {
                DomainAction::ToggleDiffFocus
            }
            DiffIntent::Blur if self.diff.interaction.focused => DomainAction::ToggleDiffFocus,
            DiffIntent::SwitchSide if self.diff.can_switch_side(&self.selection.diff_target()) => {
                DomainAction::SwitchDiffSide
            }
            DiffIntent::NextLine if self.diff.can_navigate() => DomainAction::DiffNextLine,
            DiffIntent::PreviousLine if self.diff.can_navigate() => DomainAction::DiffPreviousLine,
            DiffIntent::NextHunk if self.diff.can_navigate() => DomainAction::HunkNext,
            DiffIntent::PreviousHunk if self.diff.can_navigate() => DomainAction::HunkPrevious,
            DiffIntent::ToggleSelection if self.diff.can_navigate() => {
                DomainAction::ToggleHunkSelection
            }
            DiffIntent::ToggleMode if self.diff.can_navigate() => DomainAction::ToggleDiffLineMode,
            DiffIntent::ToggleRange if self.diff.can_navigate() => {
                DomainAction::ToggleDiffLineRange
            }
            DiffIntent::ClearSelection
                if self.diff.interaction.focused && self.diff.interaction.has_selection() =>
            {
                DomainAction::ClearHunkSelection
            }
            DiffIntent::NextFile | DiffIntent::PreviousFile
                if self.diff.interaction.focused && *self.view() == ViewContext::Files =>
            {
                DomainAction::Runtime(RuntimeEvent::NavigateDiffFile {
                    forward: intent == DiffIntent::NextFile,
                })
            }
            DiffIntent::ApplySelection | DiffIntent::StageSelection
                if self.diff.can_apply(PatchDirection::Stage) =>
            {
                DomainAction::ApplySelectedHunks(PatchDirection::Stage)
            }
            DiffIntent::ApplySelection | DiffIntent::UnstageSelection
                if self.diff.can_apply(PatchDirection::Unstage) =>
            {
                DomainAction::ApplySelectedHunks(PatchDirection::Unstage)
            }
            DiffIntent::DiscardSelection if self.diff.can_apply(PatchDirection::Discard) => {
                DomainAction::ApplySelectedHunks(PatchDirection::Discard)
            }
            DiffIntent::DiscardSelection if self.diff.can_apply(PatchDirection::Unstage) => {
                DomainAction::ApplySelectedHunks(PatchDirection::Unstage)
            }
            DiffIntent::ScrollUp => DomainAction::DiffScrollUp,
            DiffIntent::ScrollDown => DomainAction::DiffScrollDown,
            _ => return None,
        })
    }
}
