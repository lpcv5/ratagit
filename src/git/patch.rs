//! Lossless patch data. Display limits never turn a preview into an applicable patch.
use super::{DiffLine, DiffLineKind, GitError, PatchRepository};
use std::path::PathBuf;
use std::sync::Arc;

mod changes;
mod preview;

pub use preview::{PatchPreview, PatchRowMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchSource {
    Worktree,
    Index,
    Untracked,
    Commit(String),
    Stash(usize),
}

impl PatchSource {
    pub fn direction(&self) -> Option<PatchDirection> {
        match self {
            Self::Worktree | Self::Untracked => Some(PatchDirection::Stage),
            Self::Index => Some(PatchDirection::Unstage),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HunkId {
    pub file: usize,
    pub hunk: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineId {
    pub file: usize,
    pub hunk: usize,
    pub line: usize,
}

impl LineId {
    pub fn hunk(self) -> HunkId {
        HunkId {
            file: self.file,
            hunk: self.hunk,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchDirection {
    Stage,
    Unstage,
    /// Reverse only the reviewed worktree changes, leaving the index untouched.
    Discard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchDisplayRow {
    pub line: DiffLine,
    pub hunk: Option<HunkId>,
    pub metadata: PatchRowMetadata,
}

pub struct HunkSpan {
    pub id: HunkId,
    pub range: std::ops::Range<usize>,
}

/// A contiguous edit inside a native hunk, identified by its first changed line.
#[derive(Debug, Clone)]
pub struct ChangeBlock {
    pub id: LineId,
    pub lines: std::ops::Range<usize>,
    pub range: std::ops::Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchRequest {
    /// All index and worktree changes for one path.
    File(PathBuf),
    #[cfg_attr(not(test), allow(dead_code))]
    Worktree(PathBuf),
    #[cfg_attr(not(test), allow(dead_code))]
    Index(PathBuf),
    Untracked(PathBuf),
    Directory(PathBuf),
    Commit {
        oid: String,
        path: Option<PathBuf>,
    },
    Stash {
        index: usize,
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchChange {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    TypeChange,
    Conflicted,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchSide {
    pub path: Option<PathBuf>,
    pub oid: Option<String>,
    pub mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchLineKind {
    Context,
    Added,
    Removed,
    NoNewline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchLine {
    pub kind: PatchLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    /// Exact bytes, including CRLF, missing final newline and non-UTF-8 text.
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: Vec<u8>,
    pub lines: Vec<PatchLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchFile {
    pub source: PatchSource,
    pub change: PatchChange,
    pub old: PatchSide,
    pub new: PatchSide,
    pub binary: bool,
    pub hunks: Vec<PatchHunk>,
    /// Native patch encoding is kept only for complete text files. Application
    /// uses these reviewed bytes rather than regenerating a newer worktree diff.
    pub encoded: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub request: PatchRequest,
    pub files: Vec<PatchFile>,
    pub truncated: bool,
    /// Native HEAD-to-worktree presentation mapped to the reviewed source IDs.
    /// Application always uses `files`, never these display-only rows.
    pub preview: Option<PatchPreview>,
}

struct DisplayGroup {
    files: Vec<usize>,
    hunks: Vec<HunkId>,
}

impl Patch {
    fn display_groups(&self) -> Vec<DisplayGroup> {
        let combine = matches!(
            self.request,
            PatchRequest::File(_) | PatchRequest::Directory(_)
        );
        let mut paths = std::collections::HashMap::new();
        let mut groups: Vec<DisplayGroup> = Vec::new();
        for (index, file) in self.files.iter().enumerate() {
            let path = file.new.path.as_ref().or(file.old.path.as_ref());
            let existing = if combine {
                path.and_then(|path| paths.get(path).copied())
            } else {
                None
            };
            let group = existing.unwrap_or_else(|| {
                let group = groups.len();
                if let Some(path) = path.filter(|_| combine) {
                    paths.insert(path, group);
                }
                groups.push(DisplayGroup {
                    files: Vec::new(),
                    hunks: Vec::new(),
                });
                group
            });
            groups[group].files.push(index);
            groups[group]
                .hunks
                .extend((0..file.hunks.len()).map(|hunk| HunkId { file: index, hunk }));
        }
        for group in &mut groups {
            if group.files.len() > 1 {
                // Both coordinates refer to the index: staged hunks end there,
                // while unstaged hunks start there. Keep application IDs intact.
                group.hunks.sort_by_key(|id| {
                    let file = &self.files[id.file];
                    let hunk = &file.hunks[id.hunk];
                    if file.source == PatchSource::Index {
                        hunk.new_start
                    } else {
                        hunk.old_start
                    }
                });
            }
        }
        groups
    }

    fn group_header_rows(&self, group: &DisplayGroup) -> usize {
        let first = &self.files[group.files[0]];
        let last = &self.files[*group.files.last().unwrap()];
        2 + usize::from(first.old.mode != last.new.mode)
            + usize::from(group.files.iter().any(|index| self.files[*index].binary))
    }

    pub fn display_file_count(&self) -> usize {
        self.display_groups().len()
    }

    pub fn display_lines(&self) -> Vec<DiffLine> {
        self.display_rows()
            .into_iter()
            .map(|row| row.line)
            .collect()
    }

    pub fn display_rows(&self) -> Vec<PatchDisplayRow> {
        if let Some(preview) = &self.preview {
            return preview.rows.clone();
        }
        let mut lines = Vec::new();
        for group in self.display_groups() {
            let file = &self.files[group.files[0]];
            let last = &self.files[*group.files.last().unwrap()];
            let old = file
                .old
                .path
                .as_ref()
                .map(|p| format!("a/{}", p.display()))
                .unwrap_or_else(|| "/dev/null".into());
            let new = last
                .new
                .path
                .as_ref()
                .map(|p| format!("b/{}", p.display()))
                .unwrap_or_else(|| "/dev/null".into());
            lines.push(DiffLine {
                kind: DiffLineKind::Header,
                content: format!("--- {old}"),
            });
            lines.push(DiffLine {
                kind: DiffLineKind::Header,
                content: format!("+++ {new}"),
            });
            if file.old.mode != last.new.mode {
                lines.push(DiffLine {
                    kind: DiffLineKind::Header,
                    content: format!("mode {:o} -> {:o}", file.old.mode, last.new.mode),
                });
            }
            if group.files.iter().any(|index| self.files[*index].binary) {
                lines.push(DiffLine {
                    kind: DiffLineKind::Header,
                    content: "<binary file>".into(),
                });
            }
            for id in group.hunks {
                let hunk = &self.files[id.file].hunks[id.hunk];
                lines.push(DiffLine {
                    kind: DiffLineKind::Header,
                    content: String::from_utf8_lossy(&hunk.header)
                        .trim_end_matches('\n')
                        .into(),
                });
                for line in &hunk.lines {
                    let kind = match line.kind {
                        PatchLineKind::Added => DiffLineKind::Added,
                        PatchLineKind::Removed => DiffLineKind::Removed,
                        PatchLineKind::Context => DiffLineKind::Context,
                        PatchLineKind::NoNewline => DiffLineKind::Header,
                    };
                    lines.push(DiffLine {
                        kind,
                        content: String::from_utf8_lossy(&line.content)
                            .trim_end_matches('\n')
                            .into(),
                    });
                }
            }
        }
        if self.truncated {
            lines.push(DiffLine {
                kind: DiffLineKind::Header,
                content: "... patch preview truncated ...".into(),
            });
        }
        let mut rows: Vec<_> = lines
            .into_iter()
            .map(|line| PatchDisplayRow {
                line,
                hunk: None,
                metadata: PatchRowMetadata::default(),
            })
            .collect();
        for span in self.hunk_spans() {
            for (offset, row) in rows[span.range].iter_mut().enumerate() {
                row.hunk = Some(span.id);
                if let Some(line) = offset.checked_sub(1) {
                    let entry = &self.files[span.id.file].hunks[span.id.hunk].lines[line];
                    row.metadata = PatchRowMetadata {
                        id: (entry.kind != PatchLineKind::NoNewline).then_some(LineId {
                            file: span.id.file,
                            hunk: span.id.hunk,
                            line,
                        }),
                        old_line: entry.old_line,
                        new_line: entry.new_line,
                    };
                }
            }
        }
        rows
    }

    /// Source-hunk ranges for the lossless patches. Combined presentation uses
    /// `row_metadata` and `line_row` because one native header can cover both sources.
    pub fn hunk_spans(&self) -> Vec<HunkSpan> {
        let mut spans = Vec::new();
        let mut offset = 0;
        for group in self.display_groups() {
            offset += self.group_header_rows(&group);
            for id in group.hunks {
                let hunk = &self.files[id.file].hunks[id.hunk];
                spans.push(HunkSpan {
                    id,
                    range: offset..offset + 1 + hunk.lines.len(),
                });
                offset += 1 + hunk.lines.len();
            }
        }
        spans
    }

    pub fn validate_hunks(&self, file: usize, hunks: &[usize]) -> Result<&PatchFile, GitError> {
        self.validate_file_hunks(file, hunks, PatchDirection::Stage)
    }

    pub fn validate_file_hunks(
        &self,
        file: usize,
        hunks: &[usize],
        direction: PatchDirection,
    ) -> Result<&PatchFile, GitError> {
        let selected = self
            .files
            .get(file)
            .ok_or_else(|| GitError::Git2("invalid patch file".into()))?;
        if self.truncated
            || selected.binary
            || selected.encoded.is_none()
            || !match direction {
                PatchDirection::Discard => matches!(
                    selected.source,
                    PatchSource::Worktree | PatchSource::Untracked
                ),
                _ => selected.source.direction() == Some(direction),
            }
            || !matches!(
                selected.change,
                PatchChange::Modified | PatchChange::Added | PatchChange::Deleted
            )
            || (selected.change == PatchChange::Modified && selected.old.mode != selected.new.mode)
            || [selected.old.mode, selected.new.mode]
                .iter()
                .any(|mode| !matches!(mode, 0 | 0o100644 | 0o100755))
        {
            return Err(GitError::Git2(
                "partial index changes require a complete text patch without rename or mode changes"
                    .into(),
            ));
        }
        let unique: std::collections::HashSet<_> = hunks.iter().collect();
        if hunks.is_empty()
            || unique.len() != hunks.len()
            || hunks.iter().any(|i| *i >= selected.hunks.len())
        {
            return Err(GitError::Git2("invalid hunk selection".into()));
        }
        Ok(selected)
    }

    pub fn hunk_ids(&self) -> Vec<HunkId> {
        self.hunk_spans().into_iter().map(|span| span.id).collect()
    }

    pub fn line_ids(&self) -> Vec<LineId> {
        self.cursor_line_ids()
            .into_iter()
            .filter(|id| {
                matches!(
                    self.files[id.file].hunks[id.hunk].lines[id.line].kind,
                    PatchLineKind::Added | PatchLineKind::Removed
                )
            })
            .collect()
    }

    pub fn validate_lines(
        &self,
        ids: &[LineId],
        direction: PatchDirection,
    ) -> Result<(), GitError> {
        if ids.is_empty() || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
        {
            return Err(GitError::Git2("invalid line selection".into()));
        }
        for id in ids {
            let file = self.validate_file_hunks(id.file, &[id.hunk], direction)?;
            if !file.hunks[id.hunk].lines.get(id.line).is_some_and(|line| {
                matches!(line.kind, PatchLineKind::Added | PatchLineKind::Removed)
            }) {
                return Err(GitError::Git2("only changed lines can be applied".into()));
            }
        }
        Ok(())
    }

    /// Cursor movement includes context; only added/removed rows are writable.
    pub fn cursor_line_ids(&self) -> Vec<LineId> {
        if let Some(preview) = &self.preview {
            return preview
                .rows
                .iter()
                .filter_map(|row| row.metadata.id)
                .collect();
        }
        self.hunk_ids()
            .into_iter()
            .flat_map(|id| {
                self.files[id.file].hunks[id.hunk]
                    .lines
                    .iter()
                    .enumerate()
                    .filter_map(move |(line, entry)| {
                        (entry.kind != PatchLineKind::NoNewline).then_some(LineId {
                            file: id.file,
                            hunk: id.hunk,
                            line,
                        })
                    })
            })
            .collect()
    }

    pub fn line_row(&self, id: LineId) -> Option<usize> {
        if let Some(preview) = &self.preview {
            return preview.positions.get(&id).copied();
        }
        self.hunk_spans()
            .iter()
            .find(|span| span.id == id.hunk())
            .filter(|span| id.line < span.range.len().saturating_sub(1))
            .map(|span| span.range.start + 1 + id.line)
    }

    pub fn validate_selection(
        &self,
        ids: &[HunkId],
        direction: PatchDirection,
    ) -> Result<(), GitError> {
        if ids.is_empty() || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
        {
            return Err(GitError::Git2("invalid hunk selection".into()));
        }
        for id in ids {
            self.validate_file_hunks(id.file, &[id.hunk], direction)?;
        }
        Ok(())
    }
}

/// Textual branch logs deliberately have no patch structure.
#[derive(Debug, Clone, Default)]
pub struct DiffDocument {
    pub lines: crate::shared::Shared<Vec<DiffLine>>,
    pub patch: Option<Arc<Patch>>,
}

impl From<Vec<DiffLine>> for DiffDocument {
    fn from(lines: Vec<DiffLine>) -> Self {
        Self {
            lines: lines.into(),
            patch: None,
        }
    }
}

impl std::ops::Deref for DiffDocument {
    type Target = [DiffLine];
    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

/// Narrow consumer: does not require history, stash or reference mutation capabilities.
pub(crate) fn document(
    repo: &dyn PatchRepository,
    request: PatchRequest,
) -> Result<Option<DiffDocument>, GitError> {
    Ok(repo.patch(request)?.map(|patch| DiffDocument {
        lines: patch.display_lines().into(),
        patch: Some(std::sync::Arc::new(patch)),
    }))
}
