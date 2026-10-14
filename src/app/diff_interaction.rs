use super::AppState;
use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;
use crate::git::patch::{HunkId, Patch, PatchDirection};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Unstaged,
    Staged,
}

#[derive(Debug, Clone, Default)]
pub struct DiffInteraction {
    pub side: Option<DiffSide>,
    pub focused: bool,
    pub cursor: Option<HunkId>,
    pub selected: BTreeSet<HunkId>,
    pub pending_write: Option<u64>,
    next_write: u64,
}

impl DiffInteraction {
    pub fn reset_selection(&mut self) {
        self.cursor = None;
        self.selected.clear();
    }
}

/// The same immutable projection controls input, palette availability and hints.
pub struct HunkView<'a> {
    pub patch: Option<&'a Patch>,
    pub interaction: &'a DiffInteraction,
    pub ready: bool,
    pub files_context: bool,
}

impl HunkView<'_> {
    pub fn ids(&self) -> Vec<HunkId> {
        self.patch
            .filter(|_| self.ready)
            .map(Patch::hunk_ids)
            .unwrap_or_default()
    }

    pub fn targets(&self, direction: PatchDirection) -> Vec<HunkId> {
        if !self.ready || !self.files_context || self.interaction.pending_write.is_some() {
            return Vec::new();
        }
        let ids: Vec<_> = if self.interaction.selected.is_empty() {
            self.interaction.cursor.into_iter().collect()
        } else {
            self.interaction.selected.iter().copied().collect()
        };
        match self.patch {
            Some(patch) if patch.validate_selection(&ids, direction).is_ok() => ids,
            _ => Vec::new(),
        }
    }
}

impl AppState {
    pub fn hunk_view(&self) -> HunkView<'_> {
        HunkView {
            patch: self.git.current_patch.as_deref(),
            interaction: &self.ui.diff,
            ready: !self.is_diff_loading(),
            files_context: self.ui.active_panel == super::SidePanel::Files,
        }
    }

    pub(crate) fn apply_diff_action(&mut self, action: &DomainAction) {
        match action {
            DomainAction::ToggleDiffFocus => {
                if self.ui.diff.focused {
                    self.ui.diff.focused = false;
                } else if let Some(first) = self.hunk_view().ids().first().copied() {
                    self.ui.diff.focused = true;
                    self.ui.diff.side =
                        self.git
                            .current_patch
                            .as_ref()
                            .and_then(|patch| match patch.request {
                                crate::git::patch::PatchRequest::Worktree(_)
                                | crate::git::patch::PatchRequest::Untracked(_) => {
                                    Some(DiffSide::Unstaged)
                                }
                                crate::git::patch::PatchRequest::Index(_) => Some(DiffSide::Staged),
                                _ => None,
                            });
                    self.ui.diff.cursor.get_or_insert(first);
                    self.scroll_to_hunk();
                }
            }
            DomainAction::SwitchDiffSide if self.ui.diff.focused && !self.is_diff_loading() => {
                let staged = self.ui.diff.side == Some(DiffSide::Staged)
                    || self.git.current_patch.as_ref().is_some_and(|patch| {
                        matches!(patch.request, crate::git::patch::PatchRequest::Index(_))
                    });
                self.ui.diff.side = Some(if staged {
                    DiffSide::Unstaged
                } else {
                    DiffSide::Staged
                });
                self.schedule_diff_reload();
                self.flush_pending_diff_reload();
            }
            DomainAction::HunkNext | DomainAction::HunkPrevious if self.ui.diff.focused => {
                let ids = self.hunk_view().ids();
                if !ids.is_empty() {
                    let current = self
                        .ui
                        .diff
                        .cursor
                        .and_then(|cursor| ids.iter().position(|id| *id == cursor))
                        .unwrap_or(0);
                    let next = if matches!(action, DomainAction::HunkNext) {
                        (current + 1).min(ids.len() - 1)
                    } else {
                        current.saturating_sub(1)
                    };
                    self.ui.diff.cursor = Some(ids[next]);
                    self.scroll_to_hunk();
                }
            }
            DomainAction::ToggleHunkSelection if self.ui.diff.focused => {
                if let Some(cursor) = self
                    .ui
                    .diff
                    .cursor
                    .filter(|id| self.hunk_view().ids().contains(id))
                {
                    let source = self
                        .git
                        .current_patch
                        .as_ref()
                        .and_then(|patch| patch.files[cursor.file].source.direction());
                    if self.ui.diff.selected.iter().any(|id| {
                        self.git
                            .current_patch
                            .as_ref()
                            .and_then(|patch| patch.files[id.file].source.direction())
                            != source
                    }) {
                        self.ui.diff.selected.clear();
                    }
                    if !self.ui.diff.selected.remove(&cursor) {
                        self.ui.diff.selected.insert(cursor);
                    }
                }
            }
            DomainAction::ClearHunkSelection => self.ui.diff.selected.clear(),
            DomainAction::ApplySelectedHunks(direction) if self.ui.diff.focused => {
                let ids = self.hunk_view().targets(*direction);
                if !ids.is_empty() {
                    self.ui.diff.next_write += 1;
                    let id = self.ui.diff.next_write;
                    self.ui.diff.pending_write = Some(id);
                    self.effects.push(EffectRequest::ApplyHunks {
                        id,
                        patch: self.git.current_patch.clone().unwrap(),
                        ids,
                        direction: *direction,
                    });
                }
            }
            DomainAction::ApplyHunksFinished {
                id,
                direction,
                result,
            } if self.ui.diff.pending_write == Some(*id) => {
                self.ui.diff.pending_write = None;
                match result {
                    Ok(()) => self.push_log(
                        if *direction == PatchDirection::Stage {
                            "Selected hunks staged"
                        } else {
                            "Selected hunks unstaged"
                        },
                        true,
                    ),
                    Err(error) => self.push_log(error.clone(), false),
                }
                // Reload on failure too: an external Git client may have changed the patch.
                self.ui.diff.reset_selection();
                self.request_refresh(super::RefreshKind::StatusOnly);
                self.schedule_diff_reload();
            }
            DomainAction::PanelNext
            | DomainAction::PanelPrev
            | DomainAction::PanelGoto(_)
            | DomainAction::ListUp
            | DomainAction::ListDown => {
                self.ui.diff.focused = false;
                self.ui.diff.side = None;
                self.ui.diff.reset_selection();
                return;
            }
            _ => return,
        }
        self.ui.dirty.mark_all();
    }

    fn scroll_to_hunk(&mut self) {
        if let Some(patch) = &self.git.current_patch {
            if let Some(index) = patch
                .hunk_spans()
                .iter()
                .find(|span| Some(span.id) == self.ui.diff.cursor)
                .map(|span| span.range.start)
            {
                self.ui.diff_scroll = index;
            }
        }
    }
}
