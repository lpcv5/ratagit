//! Blend presentation without changing the lossless patches used for writes.
use super::{MAX_PATCH_LINES, append, options};
use crate::GitError;
use git2::Repository;
use ratagit_core::{
    partial::{SelectionMode, groups, patch_rows},
    patch::*,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) fn install(repo: &Repository, path: &Path, patch: &mut Patch) -> Result<(), GitError> {
    if patch.truncated
        || !patch
            .files
            .iter()
            .any(|file| file.source == PatchSource::Index)
        || !patch
            .files
            .iter()
            .any(|file| file.source != PatchSource::Index)
        || patch.files.iter().any(|file| {
            file.binary
                || !matches!(
                    file.change,
                    PatchChange::Added | PatchChange::Deleted | PatchChange::Modified
                )
                || (file.change == PatchChange::Modified && file.old.mode != file.new.mode)
        })
    {
        return Ok(());
    }
    let head = repo.head().ok().and_then(|head| head.peel_to_tree().ok());
    let mut opts = options(Some(path))?;
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    let diff = repo.diff_tree_to_workdir_with_index(head.as_ref(), Some(&mut opts))?;
    let mut native = Patch {
        request: patch.request.clone(),
        files: Vec::new(),
        truncated: false,
        preview: None,
    };
    let mut remaining = MAX_PATCH_LINES;
    append(&diff, PatchSource::Worktree, &mut native, &mut remaining)?;
    if !native.truncated {
        patch.preview = project(patch, &native);
    }
    Ok(())
}

type Coordinate = (PathBuf, u8, Option<u32>, Option<u32>);

fn key(path: &Path, kind: PatchLineKind, old: Option<u32>, new: Option<u32>) -> Coordinate {
    (path.to_owned(), kind as u8, old, new)
}

fn file_path(file: &PatchFile) -> Option<&Path> {
    file.new.path.as_deref().or(file.old.path.as_deref())
}

fn project(patch: &Patch, native: &Patch) -> Option<Vec<PatchRow>> {
    let mut candidates: BTreeMap<Coordinate, Vec<LineId>> = BTreeMap::new();
    let mut changed = BTreeSet::new();
    for (file_id, file) in patch.files.iter().enumerate() {
        let path = file_path(file)?;
        let other = patch.files.iter().find(|other| {
            file_path(other) == Some(path)
                && (other.source == PatchSource::Index) != (file.source == PatchSource::Index)
        });
        for (hunk_id, hunk) in file.hunks.iter().enumerate() {
            for (line_id, line) in hunk.lines.iter().enumerate() {
                if line.kind == PatchLineKind::NoNewline {
                    continue;
                }
                let id = LineId {
                    file: file_id,
                    hunk: hunk_id,
                    line: line_id,
                };
                let (old, new) = if file.source == PatchSource::Index {
                    (
                        line.old_line,
                        line.new_line.and_then(|n| translate(other, n, false)),
                    )
                } else {
                    (
                        line.old_line.and_then(|n| translate(other, n, true)),
                        line.new_line,
                    )
                };
                // An edit rewritten by the other source cannot be represented
                // by a two-way diff. Keep its full reviewed source presentation.
                if (line.old_line.is_some() && old.is_none())
                    || (line.new_line.is_some() && new.is_none())
                {
                    if line.kind != PatchLineKind::Context {
                        return None;
                    }
                    continue;
                }
                candidates
                    .entry(key(path, line.kind, old, new))
                    .or_default()
                    .push(id);
                if line.kind != PatchLineKind::Context {
                    changed.insert(id);
                }
            }
        }
    }
    let mut rows = patch_rows(native);
    let mut positions = BTreeMap::new();
    let mut matched = BTreeSet::new();
    for (index, row) in rows.iter_mut().enumerate() {
        let native_id = row.id.take();
        let Some(id) = native_id else { continue };
        let file = &native.files[id.file];
        let line = &file.hunks[id.hunk].lines[id.line];
        let candidate = candidates
            .get(&key(
                file_path(file)?,
                line.kind,
                line.old_line,
                line.new_line,
            ))
            .and_then(|ids| {
                ids.iter().copied().find(|id| {
                    patch.files[id.file].hunks[id.hunk].lines[id.line].content == line.content
                        && !positions.contains_key(id)
                })
            });
        if let Some(id) = candidate {
            row.id = Some(id);
            positions.insert(id, index);
            if line.kind != PatchLineKind::Context && !matched.insert(id) {
                return None;
            }
        } else if line.kind != PatchLineKind::Context {
            return None;
        }
    }
    if changed != matched {
        return None;
    }
    // One selectable edit must remain contiguous in presentation. Otherwise
    // retain source hunks so focus never includes another edit's changed rows.
    for block in groups(patch, SelectionMode::Block, false)
        .into_iter()
        .chain(groups(patch, SelectionMode::Block, true))
    {
        let mut mapped = block.iter().filter_map(|id| positions.get(id).copied());
        let mut previous = mapped.next()?;
        for row in mapped {
            if row <= previous
                || (row != previous + 1
                    && !rows[previous + 1..row].iter().all(|row| row.id.is_none()))
            {
                return None;
            }
            previous = row;
        }
    }
    for row in &mut rows {
        if row.id.is_none() {
            row.text = row
                .text
                .replace("[Worktree]", "[Combined staged + unstaged]");
        }
    }
    Some(rows)
}

/// Translate an unchanged line through a source diff. A changed input line has
/// no corresponding coordinate. Empty-side hunks insert *after* their start.
fn translate(file: Option<&PatchFile>, number: u32, reverse: bool) -> Option<u32> {
    let Some(file) = file else {
        return Some(number);
    };
    let mut offset = 0_i64;
    for hunk in &file.hunks {
        let (start, count, output_count) = if reverse {
            (hunk.new_start, hunk.new_lines, hunk.old_lines)
        } else {
            (hunk.old_start, hunk.old_lines, hunk.new_lines)
        };
        if count > 0 && (start..start.saturating_add(count)).contains(&number) {
            return hunk.lines.iter().find_map(|line| {
                let (from, to) = if reverse {
                    (line.new_line, line.old_line)
                } else {
                    (line.old_line, line.new_line)
                };
                (line.kind == PatchLineKind::Context && from == Some(number))
                    .then_some(to)
                    .flatten()
            });
        }
        if (count == 0 && number > start) || (count > 0 && number >= start.saturating_add(count)) {
            offset += i64::from(output_count) - i64::from(count);
        }
    }
    u32::try_from(i64::from(number) + offset).ok()
}
