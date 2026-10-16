//! Lossless reviewed patch bytes, independent of Git I/O and rendering.
use std::path::PathBuf;

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
    CommitFiles {
        oid: String,
        paths: Vec<PathBuf>,
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
    /// Native Git file header, independent of whether the body is writable.
    pub header: Vec<u8>,
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
    /// Display-only combined rows. Writes always use the lossless source files.
    pub preview: Option<Vec<PatchRow>>,
    pub request: PatchRequest,
    pub files: Vec<PatchFile>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRow {
    pub text: String,
    pub id: Option<LineId>,
    pub kind: PatchLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

impl Patch {
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

    pub fn validate_lines(&self, ids: &[LineId], direction: PatchDirection) -> Result<(), String> {
        if ids.is_empty() || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
        {
            return Err("invalid line selection".into());
        }
        for id in ids {
            let file = self.files.get(id.file).ok_or("invalid patch file")?;
            if self.truncated
                || file.binary
                || file.encoded.is_none()
                || !match direction {
                    PatchDirection::Discard => {
                        matches!(file.source, PatchSource::Worktree | PatchSource::Untracked)
                    }
                    _ => file.source.direction() == Some(direction),
                }
                || !matches!(
                    file.change,
                    PatchChange::Modified | PatchChange::Added | PatchChange::Deleted
                )
                || (file.change == PatchChange::Modified && file.old.mode != file.new.mode)
                || [file.old.mode, file.new.mode]
                    .iter()
                    .any(|mode| !matches!(mode, 0 | 0o100644 | 0o100755))
            {
                return Err(
                    "partial changes require a complete text patch without rename or mode changes"
                        .into(),
                );
            }
            if !file
                .hunks
                .get(id.hunk)
                .and_then(|hunk| hunk.lines.get(id.line))
                .is_some_and(|line| {
                    matches!(line.kind, PatchLineKind::Added | PatchLineKind::Removed)
                })
            {
                return Err("only changed lines can be applied".into());
            }
        }
        Ok(())
    }
}
