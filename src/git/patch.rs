//! Lossless patch data. Display limits never turn a preview into an applicable patch.
use super::{DiffLine, DiffLineKind, GitError, PatchRepository};
use std::path::PathBuf;
use std::sync::Arc;

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

pub struct PatchDisplayRow {
    pub line: DiffLine,
    pub hunk: Option<HunkId>,
}

pub struct HunkSpan {
    pub id: HunkId,
    pub range: std::ops::Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchRequest {
    Worktree(PathBuf),
    Index(PathBuf),
    Untracked(PathBuf),
    Directory(PathBuf),
    Commit { oid: String, path: Option<PathBuf> },
    Stash { index: usize, path: Option<PathBuf> },
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
}

impl Patch {
    pub fn display_lines(&self) -> Vec<DiffLine> {
        self.display_rows()
            .into_iter()
            .map(|row| row.line)
            .collect()
    }

    pub fn display_rows(&self) -> Vec<PatchDisplayRow> {
        let mut lines = Vec::new();
        for file in &self.files {
            let old = file
                .old
                .path
                .as_ref()
                .map(|p| format!("a/{}", p.display()))
                .unwrap_or_else(|| "/dev/null".into());
            let new = file
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
            if file.old.mode != file.new.mode {
                lines.push(DiffLine {
                    kind: DiffLineKind::Header,
                    content: format!("mode {:o} -> {:o}", file.old.mode, file.new.mode),
                });
            }
            if file.binary {
                lines.push(DiffLine {
                    kind: DiffLineKind::Header,
                    content: "<binary file>".into(),
                });
            }
            for hunk in &file.hunks {
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
            .map(|line| PatchDisplayRow { line, hunk: None })
            .collect();
        for span in self.hunk_spans() {
            for row in &mut rows[span.range] {
                row.hunk = Some(span.id);
            }
        }
        rows
    }

    pub fn hunk_spans(&self) -> Vec<HunkSpan> {
        let mut spans = Vec::new();
        let mut offset = 0;
        for (file_index, file) in self.files.iter().enumerate() {
            offset += 2 + usize::from(file.old.mode != file.new.mode) + usize::from(file.binary);
            for (hunk_index, hunk) in file.hunks.iter().enumerate() {
                let id = HunkId {
                    file: file_index,
                    hunk: hunk_index,
                };
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
        self.files
            .iter()
            .enumerate()
            .flat_map(|(file, item)| (0..item.hunks.len()).map(move |hunk| HunkId { file, hunk }))
            .collect()
    }

    pub fn line_ids(&self) -> Vec<LineId> {
        self.files
            .iter()
            .enumerate()
            .flat_map(|(file, entry)| {
                entry
                    .hunks
                    .iter()
                    .enumerate()
                    .flat_map(move |(hunk, entry)| {
                        entry
                            .lines
                            .iter()
                            .enumerate()
                            .filter_map(move |(line, entry)| {
                                matches!(entry.kind, PatchLineKind::Added | PatchLineKind::Removed)
                                    .then_some(LineId { file, hunk, line })
                            })
                    })
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
        self.files
            .iter()
            .enumerate()
            .flat_map(|(file, entry)| {
                entry
                    .hunks
                    .iter()
                    .enumerate()
                    .flat_map(move |(hunk, entry)| {
                        entry
                            .lines
                            .iter()
                            .enumerate()
                            .filter_map(move |(line, entry)| {
                                (entry.kind != PatchLineKind::NoNewline).then_some(LineId {
                                    file,
                                    hunk,
                                    line,
                                })
                            })
                    })
            })
            .collect()
    }

    pub fn line_row(&self, id: LineId) -> Option<usize> {
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
