use super::{DiffSide, DiffState};
use crate::git::patch::{HunkId, Patch, PatchDirection, PatchRequest};
use std::sync::Arc;

/// Feature input independent of Flux actions and the rest of application state.
pub enum DiffAction<'a> {
    ToggleFocus,
    SwitchSide,
    NextHunk,
    PreviousHunk,
    ToggleSelection,
    ClearSelection,
    ApplySelected(PatchDirection),
    WriteFinished {
        id: u64,
        direction: PatchDirection,
        result: &'a Result<(), String>,
    },
    SelectionChanged,
    ScrollUp,
    ScrollDown,
    StageHunks {
        patch: &'a Arc<Patch>,
        file: usize,
        hunks: &'a [usize],
    },
    StageHunksFinished(&'a Result<(), String>),
}

pub enum HunkWrite {
    Selected {
        id: u64,
        patch: Arc<Patch>,
        ids: Vec<HunkId>,
        direction: PatchDirection,
    },
    Legacy {
        patch: Arc<Patch>,
        file: usize,
        hunks: Vec<usize>,
    },
}

#[derive(Default, PartialEq, Eq)]
pub enum DiffReload {
    #[default]
    None,
    Deferred,
    Immediate,
}

#[derive(Default)]
pub struct DiffUpdate {
    pub write: Option<HunkWrite>,
    pub log: Option<(String, bool)>,
    pub refresh_status: bool,
    pub reload: DiffReload,
    pub redraw: bool,
}

impl DiffState {
    pub fn update(&mut self, action: DiffAction<'_>, files_context: bool) -> DiffUpdate {
        let mut output = DiffUpdate::default();
        match action {
            DiffAction::ToggleFocus => {
                if self.interaction.focused {
                    self.interaction.focused = false;
                } else if let Some(first) = self.view().hunks(files_context).ids().first().copied()
                {
                    self.interaction.focused = true;
                    self.interaction.side =
                        self.document
                            .patch
                            .as_ref()
                            .and_then(|patch| match patch.request {
                                PatchRequest::Worktree(_) | PatchRequest::Untracked(_) => {
                                    Some(DiffSide::Unstaged)
                                }
                                PatchRequest::Index(_) => Some(DiffSide::Staged),
                                _ => None,
                            });
                    self.interaction.cursor.get_or_insert(first);
                    self.scroll_to_hunk();
                }
            }
            DiffAction::SwitchSide if self.interaction.focused && !self.is_loading() => {
                let staged = self.interaction.side == Some(DiffSide::Staged)
                    || self
                        .document
                        .patch
                        .as_ref()
                        .is_some_and(|patch| matches!(patch.request, PatchRequest::Index(_)));
                self.interaction.side = Some(if staged {
                    DiffSide::Unstaged
                } else {
                    DiffSide::Staged
                });
                output.reload = DiffReload::Immediate;
            }
            DiffAction::NextHunk | DiffAction::PreviousHunk if self.interaction.focused => {
                let ids = self.view().hunks(files_context).ids();
                if !ids.is_empty() {
                    let current = self
                        .interaction
                        .cursor
                        .and_then(|cursor| ids.iter().position(|id| *id == cursor))
                        .unwrap_or(0);
                    let next = if matches!(action, DiffAction::NextHunk) {
                        (current + 1).min(ids.len() - 1)
                    } else {
                        current.saturating_sub(1)
                    };
                    self.interaction.cursor = Some(ids[next]);
                    self.scroll_to_hunk();
                }
            }
            DiffAction::ToggleSelection if self.interaction.focused => {
                if let Some(cursor) = self
                    .interaction
                    .cursor
                    .filter(|id| self.view().hunks(files_context).ids().contains(id))
                {
                    let source = self
                        .document
                        .patch
                        .as_ref()
                        .and_then(|patch| patch.files[cursor.file].source.direction());
                    if self.interaction.selected.iter().any(|id| {
                        self.document
                            .patch
                            .as_ref()
                            .and_then(|patch| patch.files[id.file].source.direction())
                            != source
                    }) {
                        self.interaction.selected.clear();
                    }
                    if !self.interaction.selected.remove(&cursor) {
                        self.interaction.selected.insert(cursor);
                    }
                }
            }
            DiffAction::ClearSelection => self.interaction.selected.clear(),
            DiffAction::ApplySelected(direction) if self.interaction.focused => {
                let ids = self.view().hunks(files_context).targets(direction);
                if !ids.is_empty() {
                    self.interaction.next_write += 1;
                    let id = self.interaction.next_write;
                    self.interaction.pending_write = Some(id);
                    output.write = Some(HunkWrite::Selected {
                        id,
                        patch: self.document.patch.clone().unwrap(),
                        ids,
                        direction,
                    });
                }
            }
            DiffAction::WriteFinished {
                id,
                direction,
                result,
            } if self.interaction.pending_write == Some(id) => {
                self.interaction.pending_write = None;
                output.log = Some(match result {
                    Ok(()) => (
                        if direction == PatchDirection::Stage {
                            "Selected hunks staged"
                        } else {
                            "Selected hunks unstaged"
                        }
                        .to_string(),
                        true,
                    ),
                    Err(error) => (error.clone(), false),
                });
                // Failed writes also require a fresh status and patch.
                self.interaction.reset_selection();
                output.refresh_status = true;
                output.reload = DiffReload::Deferred;
            }
            DiffAction::SelectionChanged => {
                self.interaction.focused = false;
                self.interaction.side = None;
                self.interaction.reset_selection();
                return output;
            }
            DiffAction::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(10);
                return output;
            }
            DiffAction::ScrollDown => {
                let max = self.document.lines.len().saturating_sub(1);
                self.scroll = (self.scroll + 10).min(max);
                return output;
            }
            DiffAction::StageHunks { patch, file, hunks } => {
                match patch.validate_hunks(file, hunks) {
                    Ok(_) => {
                        output.write = Some(HunkWrite::Legacy {
                            patch: patch.clone(),
                            file,
                            hunks: hunks.to_vec(),
                        })
                    }
                    Err(error) => output.log = Some((error.to_string(), false)),
                }
                return output;
            }
            DiffAction::StageHunksFinished(result) => {
                output.log = Some(match result {
                    Ok(()) => ("Selected hunks staged".to_string(), true),
                    Err(error) => (error.clone(), false),
                });
                return output;
            }
            _ => return output,
        }
        output.redraw = true;
        output
    }

    fn scroll_to_hunk(&mut self) {
        if let Some(patch) = &self.document.patch {
            if let Some(index) = patch
                .hunk_spans()
                .iter()
                .find(|span| Some(span.id) == self.interaction.cursor)
                .map(|span| span.range.start)
            {
                self.scroll = index;
            }
        }
    }
}
