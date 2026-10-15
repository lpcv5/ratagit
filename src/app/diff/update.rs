use super::{DiffSide, DiffState};
use crate::git::patch::{HunkId, LineId, Patch, PatchDirection, PatchRequest};
use std::sync::Arc;

/// Feature input independent of Flux actions and the rest of application state.
pub enum DiffAction<'a> {
    ToggleFocus,
    SwitchSide,
    NextHunk,
    PreviousHunk,
    NextLine,
    PreviousLine,
    ToggleSelection,
    ToggleLineMode,
    ToggleRange,
    ClearSelection,
    ApplySelected(PatchDirection),
    WriteFinished {
        id: u64,
        direction: PatchDirection,
        result: &'a Result<(), crate::git::GitError>,
    },
    SelectionChanged,
    FileChanged,
    ScrollUp,
    ScrollDown,
    StageHunks {
        patch: &'a Arc<Patch>,
        file: usize,
        hunks: &'a [usize],
    },
    StageHunksFinished(&'a Result<(), crate::git::GitError>),
}

pub enum HunkWrite {
    Lines {
        id: u64,
        patch: Arc<Patch>,
        ids: Vec<LineId>,
        direction: PatchDirection,
    },
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
                    self.interaction.selected.clear();
                    self.interaction.selected_lines.clear();
                    self.interaction.range_anchor = None;
                    self.interaction.hunk_anchor = None;
                } else if self.is_loading() && files_context {
                    // Enter should focus immediately while the selected file's
                    // debounced patch is still loading; installation sets cursors.
                    self.interaction.focused = true;
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
                    if self.interaction.line_mode && self.interaction.line_cursor.is_none() {
                        self.interaction.line_cursor =
                            self.view().hunks(files_context).line_ids().first().copied();
                    }
                    self.scroll_to_selection();
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
            DiffAction::NextHunk
            | DiffAction::PreviousHunk
            | DiffAction::NextLine
            | DiffAction::PreviousLine
                if self.interaction.focused =>
            {
                if self.interaction.line_mode
                    && matches!(action, DiffAction::NextLine | DiffAction::PreviousLine)
                {
                    let ids = self.view().hunks(files_context).line_ids();
                    if !ids.is_empty() {
                        let current = self
                            .interaction
                            .line_cursor
                            .and_then(|cursor| ids.iter().position(|id| *id == cursor))
                            .unwrap_or(0);
                        let next = if matches!(action, DiffAction::NextLine) {
                            (current + 1).min(ids.len() - 1)
                        } else {
                            current.saturating_sub(1)
                        };
                        self.interaction.line_cursor = Some(ids[next]);
                        self.interaction.cursor = Some(ids[next].hunk());
                        self.scroll_to_selection();
                    }
                    output.redraw = true;
                    return output;
                }
                let ids = self.view().hunks(files_context).ids();
                if !ids.is_empty() {
                    let current = self
                        .interaction
                        .cursor
                        .and_then(|cursor| ids.iter().position(|id| *id == cursor))
                        .unwrap_or(0);
                    let next = if matches!(action, DiffAction::NextHunk | DiffAction::NextLine) {
                        (current + 1).min(ids.len() - 1)
                    } else {
                        current.saturating_sub(1)
                    };
                    self.interaction.cursor = Some(ids[next]);
                    if self.interaction.line_mode {
                        self.interaction.line_cursor = self
                            .document
                            .patch
                            .as_ref()
                            .unwrap()
                            .line_ids()
                            .into_iter()
                            .find(|id| id.hunk() == ids[next]);
                    }
                    self.scroll_to_selection();
                }
            }
            DiffAction::ToggleSelection if self.interaction.focused => {
                if self.interaction.line_mode {
                    if let Some(cursor) = self
                        .interaction
                        .line_cursor
                        .filter(|id| self.view().hunks(files_context).line_ids().contains(id))
                    {
                        let patch = self.document.patch.as_ref().unwrap();
                        let source = patch.files[cursor.file].source.direction();
                        if self
                            .interaction
                            .selected_lines
                            .iter()
                            .any(|id| patch.files[id.file].source.direction() != source)
                        {
                            self.interaction.selected_lines.clear();
                        }
                        self.interaction.range_anchor = None;
                        if !self.interaction.selected_lines.remove(&cursor) {
                            self.interaction.selected_lines.insert(cursor);
                        }
                    }
                    output.redraw = true;
                    return output;
                }
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
            DiffAction::ToggleLineMode if self.view().hunks(files_context).can_navigate() => {
                self.interaction.line_mode = !self.interaction.line_mode;
                self.interaction.selected.clear();
                self.interaction.selected_lines.clear();
                self.interaction.range_anchor = None;
                self.interaction.hunk_anchor = None;
                if self.interaction.line_mode {
                    self.interaction.line_cursor = self
                        .document
                        .patch
                        .as_ref()
                        .unwrap()
                        .line_ids()
                        .into_iter()
                        .find(|id| Some(id.hunk()) == self.interaction.cursor);
                }
                self.scroll_to_selection();
            }
            DiffAction::ToggleRange if self.interaction.focused && self.interaction.line_mode => {
                if self.interaction.range_anchor.is_some() {
                    self.interaction.selected_lines = self
                        .view()
                        .hunks(files_context)
                        .marked_lines()
                        .into_iter()
                        .collect();
                    self.interaction.range_anchor = None;
                } else {
                    self.interaction.selected_lines.clear();
                    self.interaction.range_anchor = self.interaction.line_cursor;
                }
            }
            DiffAction::ToggleRange if self.interaction.focused => {
                if self.interaction.hunk_anchor.is_some() {
                    self.interaction.selected = self
                        .view()
                        .hunks(files_context)
                        .marked_hunks()
                        .into_iter()
                        .collect();
                    self.interaction.hunk_anchor = None;
                } else {
                    self.interaction.selected.clear();
                    self.interaction.hunk_anchor = self.interaction.cursor;
                }
            }
            DiffAction::ClearSelection => {
                self.interaction.selected.clear();
                self.interaction.selected_lines.clear();
                self.interaction.range_anchor = None;
                self.interaction.hunk_anchor = None;
            }
            DiffAction::ApplySelected(direction) if self.interaction.focused => {
                if self.interaction.line_mode {
                    let ids = self.view().hunks(files_context).line_targets(direction);
                    if !ids.is_empty() {
                        self.interaction.next_write += 1;
                        let id = self.interaction.next_write;
                        self.interaction.pending_write = Some(id);
                        output.write = Some(HunkWrite::Lines {
                            id,
                            patch: self.document.patch.clone().unwrap(),
                            ids,
                            direction,
                        });
                    }
                    output.redraw = true;
                    return output;
                }
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
                        match direction {
                            PatchDirection::Stage => "Selected changes staged",
                            PatchDirection::Unstage => "Selected changes unstaged",
                            PatchDirection::Discard => "Selected changes discarded",
                        }
                        .to_string(),
                        true,
                    ),
                    Err(error) => (crate::app::git_error::message(error), false),
                });
                // Failed writes also require a fresh status and patch.
                self.interaction.reset_selection();
                output.refresh_status = true;
                output.reload = DiffReload::Deferred;
            }
            DiffAction::SelectionChanged | DiffAction::FileChanged => {
                self.interaction.focused = matches!(action, DiffAction::FileChanged);
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
                    Err(error) => (crate::app::git_error::message(error), false),
                });
                return output;
            }
            _ => return output,
        }
        output.redraw = true;
        output
    }

    fn scroll_to_selection(&mut self) {
        if self.interaction.line_mode {
            if let Some(row) = self.document.patch.as_ref().and_then(|patch| {
                self.interaction
                    .line_cursor
                    .and_then(|id| patch.line_row(id))
            }) {
                self.scroll = row;
            }
            return;
        }
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
