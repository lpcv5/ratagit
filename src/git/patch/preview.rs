//! Presentation coordinates are independent of source-local application IDs.
use super::{LineId, Patch, PatchDisplayRow};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PatchRowMetadata {
    pub id: Option<LineId>,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPreview {
    pub rows: Vec<PatchDisplayRow>,
    pub positions: BTreeMap<LineId, usize>,
}

impl Patch {
    pub fn row_metadata(&self) -> Vec<PatchRowMetadata> {
        if let Some(preview) = &self.preview {
            return preview.rows.iter().map(|row| row.metadata).collect();
        }
        let mut rows = vec![PatchRowMetadata::default(); self.display_row_count()];
        for span in self.hunk_spans() {
            for (line, entry) in self.files[span.id.file].hunks[span.id.hunk]
                .lines
                .iter()
                .enumerate()
            {
                rows[span.range.start + 1 + line] = PatchRowMetadata {
                    id: (entry.kind != super::PatchLineKind::NoNewline).then_some(LineId {
                        file: span.id.file,
                        hunk: span.id.hunk,
                        line,
                    }),
                    old_line: entry.old_line,
                    new_line: entry.new_line,
                };
            }
        }
        rows
    }

    fn display_row_count(&self) -> usize {
        self.display_groups()
            .iter()
            .map(|group| {
                self.group_header_rows(group)
                    + group
                        .hunks
                        .iter()
                        .map(|id| 1 + self.files[id.file].hunks[id.hunk].lines.len())
                        .sum::<usize>()
            })
            .sum::<usize>()
            + usize::from(self.truncated)
    }
}
