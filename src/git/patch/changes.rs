//! Selectable contiguous edits inside native Git hunks. Native IDs and patch
//! bytes stay intact; a block is identified by its first changed line.
use super::{ChangeBlock, LineId, Patch, PatchLineKind};

impl Patch {
    pub fn change_blocks(&self) -> Vec<ChangeBlock> {
        let mut blocks = Vec::new();
        for span in self.hunk_spans() {
            let hunk = &self.files[span.id.file].hunks[span.id.hunk];
            let mut line = 0;
            while line < hunk.lines.len() {
                if !matches!(
                    hunk.lines[line].kind,
                    PatchLineKind::Added | PatchLineKind::Removed
                ) {
                    line += 1;
                    continue;
                }
                let start = line;
                // An EOF marker belongs to its edit, but never starts an edit
                // or becomes an independently applicable line.
                while line < hunk.lines.len() && hunk.lines[line].kind != PatchLineKind::Context {
                    line += 1;
                }
                blocks.push(ChangeBlock {
                    id: LineId {
                        file: span.id.file,
                        hunk: span.id.hunk,
                        line: start,
                    },
                    lines: start..line,
                    range: span.range.start + 1 + start..span.range.start + 1 + line,
                });
            }
        }
        if self.preview.is_some() {
            for block in &mut blocks {
                let rows: Vec<_> = block
                    .lines
                    .clone()
                    .filter_map(|line| self.line_row(LineId { line, ..block.id }))
                    .collect();
                if let (Some(first), Some(last)) = (rows.iter().min(), rows.iter().max()) {
                    block.range = *first..*last + 1;
                }
            }
            blocks.sort_by_key(|block| block.range.start);
        }
        blocks
    }

    pub fn change_ids(&self) -> Vec<LineId> {
        self.change_blocks()
            .into_iter()
            .map(|block| block.id)
            .collect()
    }

    pub fn change_for_line(&self, line: LineId) -> Option<LineId> {
        self.change_blocks()
            .into_iter()
            .filter(|block| block.id.hunk() == line.hunk())
            .min_by_key(|block| {
                if block.lines.contains(&line.line) {
                    0
                } else {
                    line.line
                        .abs_diff(block.lines.start)
                        .min(line.line.abs_diff(block.lines.end - 1))
                }
            })
            .map(|block| block.id)
    }

    pub fn change_lines(&self, ids: &[LineId]) -> Option<Vec<LineId>> {
        let blocks = self.change_blocks();
        let mut lines = Vec::new();
        for id in ids {
            let block = blocks.iter().find(|block| block.id == *id)?;
            lines.extend(block.lines.clone().filter_map(|line| {
                matches!(
                    self.files[id.file].hunks[id.hunk].lines[line].kind,
                    PatchLineKind::Added | PatchLineKind::Removed
                )
                .then_some(LineId { line, ..*id })
            }));
        }
        Some(lines)
    }
}
