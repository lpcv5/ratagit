//! Approximate owned capacities, excluding cache bookkeeping and allocator overhead.
use crate::git::patch::{LineId, Patch, PatchPreview, PatchRequest, PatchSource};
use crate::git::DiffLine;
use std::mem::size_of;
use std::path::PathBuf;

const ARC_HEADER: usize = 2 * size_of::<usize>();

fn vector<T>(items: &Vec<T>) -> usize {
    items.capacity().saturating_mul(size_of::<T>())
}

fn path(path: &Option<PathBuf>) -> usize {
    path.as_ref().map_or(0, PathBuf::capacity)
}

pub(super) fn lines_bytes(lines: &Vec<DiffLine>) -> usize {
    lines.iter().fold(
        ARC_HEADER
            .saturating_add(size_of::<Vec<DiffLine>>())
            .saturating_add(vector(lines)),
        |bytes, line| bytes.saturating_add(line.content.capacity()),
    )
}

pub(super) fn patch_bytes(patch: &Patch) -> usize {
    let preview = patch.preview.as_ref().map_or(0, |preview| {
        // B-tree node allocation includes keys, values and links; account for
        // partially filled nodes conservatively in the bounded cache estimate.
        size_of::<PatchPreview>()
            .saturating_add(vector(&preview.rows))
            .saturating_add(
                preview
                    .rows
                    .iter()
                    .map(|row| row.line.content.capacity())
                    .sum::<usize>(),
            )
            .saturating_add(
                preview
                    .positions
                    .len()
                    .saturating_mul(2 * (size_of::<LineId>() + 3 * size_of::<usize>())),
            )
    });
    let request = match &patch.request {
        PatchRequest::File(path)
        | PatchRequest::Worktree(path)
        | PatchRequest::Index(path)
        | PatchRequest::Untracked(path)
        | PatchRequest::Directory(path) => path.capacity(),
        PatchRequest::Commit { oid, path: scope } => oid.capacity().saturating_add(path(scope)),
        PatchRequest::Stash { path: scope, .. } => path(scope),
    };
    patch.files.iter().fold(
        ARC_HEADER
            .saturating_add(size_of::<Patch>())
            .saturating_add(request)
            .saturating_add(preview)
            .saturating_add(vector(&patch.files)),
        |mut bytes, file| {
            if let PatchSource::Commit(oid) = &file.source {
                bytes = bytes.saturating_add(oid.capacity());
            }
            for side in [&file.old, &file.new] {
                bytes = bytes
                    .saturating_add(path(&side.path))
                    .saturating_add(side.oid.as_ref().map_or(0, String::capacity));
            }
            bytes = bytes
                .saturating_add(vector(&file.hunks))
                .saturating_add(file.encoded.as_ref().map_or(0, Vec::capacity));
            for hunk in &file.hunks {
                bytes = bytes
                    .saturating_add(hunk.header.capacity())
                    .saturating_add(vector(&hunk.lines));
                for line in &hunk.lines {
                    bytes = bytes.saturating_add(line.content.capacity());
                }
            }
            bytes
        },
    )
}
