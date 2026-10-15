//! Approximate owned capacities, excluding cache bookkeeping and allocator overhead.
use crate::git::patch::{Patch, PatchRequest, PatchSource};
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
    let request = match &patch.request {
        PatchRequest::Worktree(path)
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
