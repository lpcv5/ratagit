use crate::flux::stores::DirtyHint;

/// Tracks which UI regions need redrawing.
///
/// Uses the same bit layout as `DirtyHint` so that `StateAccess::mark_dirty()`
/// can be implemented as a trivial bitwise OR instead of translating between
/// two separate representations.
#[derive(Debug, Default, Clone, Copy)]
pub struct DirtyFlags(u8);

impl DirtyFlags {
    pub fn mark_all(&mut self) {
        self.0 = DirtyHint::MAIN_CONTENT
            | DirtyHint::DIFF
            | DirtyHint::COMMAND_LOG
            | DirtyHint::SHORTCUT_BAR
            | DirtyHint::OVERLAY;
    }

    pub fn mark_diff(&mut self) {
        self.0 |= DirtyHint::DIFF;
    }

    pub fn mark_command_log(&mut self) {
        self.0 |= DirtyHint::COMMAND_LOG;
    }

    pub fn apply_hint(&mut self, hint: DirtyHint) {
        self.0 |= hint.0;
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn is_dirty(self) -> bool {
        self.0 != 0
    }

    /// True if the main content panels (files, branches, commits, stash) need redrawing.
    pub fn main_content(self) -> bool {
        self.0 & DirtyHint::MAIN_CONTENT != 0
    }
}
