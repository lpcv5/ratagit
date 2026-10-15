//! Relocate the cursor after an index write changes source-local hunk IDs.
use crate::git::patch::{LineId, Patch, PatchDirection, PatchHunk, PatchLineKind};
use std::path::PathBuf;

pub(super) struct HunkPosition {
    path: PathBuf,
    direction: PatchDirection,
    start: u32,
    changes: Vec<(PatchLineKind, Vec<u8>)>,
    row: usize,
    scroll: usize,
}

pub(super) struct HunkLocation {
    pub cursor: LineId,
    pub previous_row: usize,
    pub row: usize,
    pub scroll: usize,
}

impl HunkPosition {
    pub(super) fn capture(
        patch: &Patch,
        id: LineId,
        direction: PatchDirection,
        applied: bool,
        scroll: usize,
    ) -> Option<Self> {
        let file = patch.files.get(id.file)?;
        let hunk = file.hunks.get(id.hunk)?;
        let source = file.source.direction()?;
        let direction = match direction {
            PatchDirection::Stage => PatchDirection::Unstage,
            PatchDirection::Unstage => PatchDirection::Stage,
            PatchDirection::Discard => return None,
        };
        let direction = if applied { direction } else { source };
        let block = patch
            .change_blocks()
            .into_iter()
            .find(|block| block.id == id)?;
        let changes = hunk.lines[block.lines]
            .iter()
            .filter(|line| matches!(line.kind, PatchLineKind::Added | PatchLineKind::Removed))
            .map(|line| (line.kind, line.content.clone()))
            .collect();
        let line = id.line;
        // Compare coordinates in the index before and after the write. Other
        // unstaged edits can shift worktree coordinates by many lines.
        let start = index_coordinate(hunk, source, line);
        Some(Self {
            path: file.new.path.as_ref().or(file.old.path.as_ref())?.clone(),
            direction,
            start,
            changes,
            row: patch.line_row(LineId {
                file: id.file,
                hunk: id.hunk,
                line,
            })?,
            scroll,
        })
    }

    pub(super) fn find(&self, patch: &Patch) -> Option<HunkLocation> {
        if self.changes.is_empty() {
            return None;
        }
        patch
            .hunk_ids()
            .into_iter()
            .filter(|id| {
                let file = &patch.files[id.file];
                file.source.direction() == Some(self.direction)
                    && file.new.path.as_ref().or(file.old.path.as_ref()) == Some(&self.path)
            })
            .filter_map(|id| {
                // Context and line coordinates may change, and adjacent hunks may
                // merge. Match the reviewed changed bytes in the destination side.
                let candidate: Vec<_> = patch.files[id.file].hunks[id.hunk]
                    .lines
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| {
                        matches!(line.kind, PatchLineKind::Added | PatchLineKind::Removed)
                    })
                    .collect();
                let window = candidate
                    .windows(self.changes.len())
                    .filter(|window| {
                        window.iter().zip(&self.changes).all(
                            |((_, line), (expected_kind, expected_bytes))| {
                                line.kind == *expected_kind && line.content == *expected_bytes
                            },
                        ) && !patch.files[id.file].hunks[id.hunk].lines
                            [window[0].0..=window[window.len() - 1].0]
                            .iter()
                            .any(|line| line.kind == PatchLineKind::Context)
                    })
                    .min_by_key(|window| {
                        index_coordinate(
                            &patch.files[id.file].hunks[id.hunk],
                            self.direction,
                            window[0].0,
                        )
                        .abs_diff(self.start)
                    })?;
                Some(LineId {
                    file: id.file,
                    hunk: id.hunk,
                    line: window[0].0,
                })
            })
            .min_by_key(|id| {
                index_coordinate(
                    &patch.files[id.file].hunks[id.hunk],
                    self.direction,
                    id.line,
                )
                .abs_diff(self.start)
            })
            .and_then(|cursor| {
                let row = patch.line_row(cursor)?;
                Some(HunkLocation {
                    cursor,
                    previous_row: self.row,
                    row,
                    scroll: if row >= self.row {
                        self.scroll.saturating_add(row - self.row)
                    } else {
                        self.scroll.saturating_sub(self.row - row)
                    },
                })
            })
    }
}

fn index_coordinate(hunk: &PatchHunk, direction: PatchDirection, line: usize) -> u32 {
    let (start, included) = if direction == PatchDirection::Unstage {
        (hunk.new_start, PatchLineKind::Added)
    } else {
        (hunk.old_start, PatchLineKind::Removed)
    };
    start.saturating_add(
        hunk.lines[..line]
            .iter()
            .filter(|line| line.kind == PatchLineKind::Context || line.kind == included)
            .count() as u32,
    )
}
