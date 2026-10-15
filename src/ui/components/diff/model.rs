use crate::git::{
    patch::{LineId, Patch},
    DiffLine,
};

/// Selection and write state are controlled by the owner, never by rendering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiffSelectionMode {
    Line,
    #[default]
    Hunk,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiffStatus {
    #[default]
    Idle,
    Loading,
    Applying,
    ReadOnly,
}

/// One-based navigation progress. Zero means there is no current target.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiffPosition {
    pub current: usize,
    pub total: usize,
}

/// Borrowed, controlled selection; patch IDs keep display and command targets aligned.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiffSelection<'a> {
    pub focused: bool,
    pub mode: DiffSelectionMode,
    /// Block cursor/marks identify the first changed line, not a native hunk.
    pub hunk_cursor: Option<LineId>,
    pub line_cursor: Option<LineId>,
    pub marked_hunks: &'a [LineId],
    pub marked_lines: &'a [LineId],
    pub range_active: bool,
    pub position: DiffPosition,
}

impl DiffSelection<'_> {
    pub(super) fn is_line_mode(&self) -> bool {
        self.mode == DiffSelectionMode::Line
    }
}

/// Keep an existing display row at the same screen position after replacement.
/// The owner supplies corresponding rows and increases `revision` per request.
#[derive(Debug, Clone, Copy)]
pub struct DiffViewportAnchor {
    pub revision: u64,
    pub previous_row: usize,
    pub row: usize,
}

/// A complete display input. `patch` must describe the supplied `lines`, including
/// when both belong to the retained preview during a background reload.
#[derive(Debug, Clone, Copy)]
pub struct DiffViewModel<'a> {
    pub lines: &'a [DiffLine],
    pub patch: Option<&'a Patch>,
    pub selection: DiffSelection<'a>,
    pub scroll: usize,
    pub viewport_anchor: Option<DiffViewportAnchor>,
    pub status: DiffStatus,
    pub empty_message: &'a str,
}

impl Default for DiffViewModel<'_> {
    fn default() -> Self {
        Self {
            lines: &[],
            patch: None,
            selection: DiffSelection::default(),
            scroll: 0,
            viewport_anchor: None,
            status: DiffStatus::Idle,
            empty_message: "No changes",
        }
    }
}

impl DiffViewModel<'_> {
    pub(super) fn ready(&self) -> bool {
        self.status != DiffStatus::Loading
    }
}
