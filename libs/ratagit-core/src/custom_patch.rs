//! Immutable source files, accumulated inclusion and independently owned reads.
mod controller;
mod selection;
#[cfg(test)]
mod tests;
use crate::PanelFocus;
use crate::patch::{LineId, Patch, PatchFile, PatchRequest};
pub use controller::{invalidate_reads, loaded, rendered, update};
pub use selection::{capture, file_inclusion, line_included};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILES: usize = 1024;
pub const PREVIEW_LINES: usize = 5000;
pub const OPTIONS_TITLE: &str = "Custom patch options";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inclusion {
    Whole,
    Lines(BTreeSet<(usize, usize)>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Data {
    pub files: BTreeMap<String, PatchFile>,
    pub preview: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ui {
    pub source: Option<String>,
    pub included: BTreeMap<String, Inclusion>,
    pub preview_return_focus: Option<PanelFocus>,
    pub scroll: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSelection {
    pub file: PatchFile,
    pub inclusion: Inclusion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomPatch {
    pub oid: String,
    pub files: Vec<FileSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Files {
        oid: String,
        paths: Vec<std::path::PathBuf>,
    },
    Lines {
        patch: Box<Patch>,
        ids: Vec<LineId>,
    },
    Options,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Options,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    Load {
        id: u64,
        request: PatchRequest,
    },
    Render {
        id: u64,
        patch: Box<CustomPatch>,
        output: Output,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Work {
    pub next_id: u64,
    pub pending: Option<Pending>,
    pub queue: VecDeque<Intent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomPatchAction {
    ToggleFiles { all: bool },
    ToggleLines,
    Options,
    Preview,
    ClosePreview,
    Scroll { up: bool, lines: usize },
    Reset,
    Keep,
    Replace(Box<Intent>),
    MenuKey(char),
}

pub fn file_key(file: &PatchFile) -> Result<&str, String> {
    file.new
        .path
        .as_ref()
        .or(file.old.path.as_ref())
        .and_then(|p| p.to_str())
        .ok_or_else(|| "custom patch has no UTF-8 file path".into())
}

pub fn source(request: &PatchRequest) -> Option<&str> {
    match request {
        PatchRequest::Commit { oid, .. } | PatchRequest::CommitFiles { oid, .. } => Some(oid),
        _ => None,
    }
}

pub fn line_ids(file: &PatchFile) -> BTreeSet<(usize, usize)> {
    file.hunks
        .iter()
        .enumerate()
        .flat_map(|(h, hunk)| {
            hunk.lines.iter().enumerate().filter_map(move |(l, line)| {
                matches!(
                    line.kind,
                    crate::patch::PatchLineKind::Added | crate::patch::PatchLineKind::Removed
                )
                .then_some((h, l))
            })
        })
        .collect()
}

impl CustomPatch {
    pub fn validate(&self) -> Result<(), String> {
        if self.files.is_empty() || self.files.len() > MAX_FILES {
            return Err("custom patch must contain 1–1024 complete files".into());
        }
        let mut paths = BTreeSet::new();
        let mut bytes = 0usize;
        for selected in &self.files {
            let file = &selected.file;
            let key = file_key(file)?;
            if !paths.insert(key)
                || file.source != crate::patch::PatchSource::Commit(self.oid.clone())
            {
                return Err("custom patch source or file identity mismatch".into());
            }
            bytes = bytes.saturating_add(
                file.encoded
                    .as_ref()
                    .ok_or("custom patch requires complete native file bytes")?
                    .len(),
            );
            if let Inclusion::Lines(lines) = &selected.inclusion {
                let patch = Patch {
                    request: PatchRequest::CommitFiles {
                        oid: self.oid.clone(),
                        paths: vec![key.into()],
                    },
                    files: vec![file.clone()],
                    preview: None,
                    truncated: false,
                };
                let ids = lines
                    .iter()
                    .map(|&(hunk, line)| LineId {
                        file: 0,
                        hunk,
                        line,
                    })
                    .collect::<Vec<_>>();
                crate::workflow::commit_files::validate(
                    &patch,
                    &ids,
                    crate::workflow::commit_files::PatchAction::Apply,
                )?;
            }
        }
        if bytes > MAX_BYTES {
            return Err("custom patch exceeds the 16 MiB limit".into());
        }
        Ok(())
    }
}
