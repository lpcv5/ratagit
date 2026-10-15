use super::patch::*;
use super::GitError;
use git2::{DiffOptions, Repository};
use std::path::Path;

const MAX_PATCH_LINES: usize = 5000;
const MAX_PREVIEW_BYTES: i64 = 1024 * 1024;
mod lines;

fn options(path: Option<&Path>) -> Result<DiffOptions, GitError> {
    let mut opts = DiffOptions::new();
    opts.max_size(MAX_PREVIEW_BYTES).include_typechange(true);
    if let Some(path) = path.filter(|p| !p.as_os_str().is_empty()) {
        // Literal paths must not accidentally select other files with glob characters.
        opts.disable_pathspec_match(true).pathspec(path);
    }
    Ok(opts)
}

pub(super) fn load(repo: &mut Repository, request: PatchRequest) -> Result<Patch, GitError> {
    repo.index()?.read(true)?;
    let mut patch = Patch {
        request: request.clone(),
        files: Vec::new(),
        truncated: false,
    };
    let mut remaining = MAX_PATCH_LINES;
    match &request {
        PatchRequest::Worktree(path) => {
            append_worktree(repo, Some(path), &mut patch, &mut remaining)?;
        }
        PatchRequest::Index(path) => {
            append_index(repo, Some(path), &mut patch, &mut remaining)?;
        }
        PatchRequest::Untracked(path) => {
            append_untracked(repo, path, &mut patch, &mut remaining)?;
        }
        PatchRequest::Directory(path) => {
            append_index(repo, Some(path), &mut patch, &mut remaining)?;
            append_worktree(repo, Some(path), &mut patch, &mut remaining)?;
            let mut opts = options(Some(path))?;
            opts.include_untracked(true)
                .recurse_untracked_dirs(true)
                .show_untracked_content(true);
            let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
            append(&diff, PatchSource::Untracked, &mut patch, &mut remaining)?;
        }
        PatchRequest::Commit { oid, path } => {
            let commit = repo.find_commit(git2::Oid::from_str(oid)?)?;
            let tree = commit.tree()?;
            let parent = if commit.parent_count() > 0 {
                Some(commit.parent(0)?.tree()?)
            } else {
                None
            };
            let mut opts = options(path.as_deref())?;
            let diff = repo.diff_tree_to_tree(parent.as_ref(), Some(&tree), Some(&mut opts))?;
            append(
                &diff,
                PatchSource::Commit(oid.clone()),
                &mut patch,
                &mut remaining,
            )?;
        }
        PatchRequest::Stash { index, path } => {
            let mut oid = None;
            repo.stash_foreach(|i, _, id| {
                if i == *index {
                    oid = Some(*id);
                    false
                } else {
                    true
                }
            })?;
            let commit = repo.find_commit(oid.ok_or(GitError::InvalidState)?)?;
            let tree = commit.tree()?;
            let parent = commit.parent(0)?.tree()?;
            let mut opts = options(path.as_deref())?;
            let diff = repo.diff_tree_to_tree(Some(&parent), Some(&tree), Some(&mut opts))?;
            append(
                &diff,
                PatchSource::Stash(*index),
                &mut patch,
                &mut remaining,
            )?;
        }
    }
    Ok(patch)
}

fn append_worktree(
    repo: &Repository,
    path: Option<&Path>,
    patch: &mut Patch,
    remaining: &mut usize,
) -> Result<(), GitError> {
    let mut opts = options(path)?;
    let mut diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
    diff.find_similar(None)?;
    append(&diff, PatchSource::Worktree, patch, remaining)
}

fn append_index(
    repo: &Repository,
    path: Option<&Path>,
    patch: &mut Patch,
    remaining: &mut usize,
) -> Result<(), GitError> {
    let head = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let mut opts = options(path)?;
    let mut diff = repo.diff_tree_to_index(head.as_ref(), None, Some(&mut opts))?;
    diff.find_similar(None)?;
    append(&diff, PatchSource::Index, patch, remaining)
}

fn append_untracked(
    repo: &Repository,
    path: &Path,
    patch: &mut Patch,
    remaining: &mut usize,
) -> Result<(), GitError> {
    if !repo.status_file(path)?.is_wt_new() {
        return Err(GitError::StalePatch(
            "requested path is no longer untracked".into(),
        ));
    }
    let mut opts = options(Some(path))?;
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
    // An explicit untracked request must not mislabel tracked changes as additions.
    append(&diff, PatchSource::Untracked, patch, remaining)
}

fn side(file: git2::DiffFile<'_>, absent: bool) -> PatchSide {
    PatchSide {
        path: if absent {
            None
        } else {
            file.path().map(Path::to_path_buf)
        },
        oid: if file.is_valid_id() && !file.id().is_zero() {
            Some(file.id().to_string())
        } else {
            None
        },
        mode: if absent { 0 } else { file.mode().into() },
    }
}

fn append(
    diff: &git2::Diff<'_>,
    source: PatchSource,
    result: &mut Patch,
    remaining: &mut usize,
) -> Result<(), GitError> {
    for (idx, delta) in diff.deltas().enumerate() {
        if source == PatchSource::Untracked && delta.status() != git2::Delta::Untracked {
            continue;
        }
        if *remaining == 0 {
            result.truncated = true;
            break;
        }
        let raw = git2::Patch::from_diff(diff, idx)?;
        let change = match delta.status() {
            git2::Delta::Added | git2::Delta::Untracked => PatchChange::Added,
            git2::Delta::Deleted => PatchChange::Deleted,
            git2::Delta::Modified => PatchChange::Modified,
            git2::Delta::Renamed => PatchChange::Renamed,
            git2::Delta::Copied => PatchChange::Copied,
            git2::Delta::Typechange => PatchChange::TypeChange,
            git2::Delta::Conflicted => PatchChange::Conflicted,
            _ => PatchChange::Other,
        };
        let mut file = PatchFile {
            source: source.clone(),
            old: side(delta.old_file(), change == PatchChange::Added),
            new: side(delta.new_file(), change == PatchChange::Deleted),
            binary: delta.old_file().is_binary() || delta.new_file().is_binary(),
            change,
            hunks: Vec::new(),
            encoded: None,
        };
        // Count metadata too, so thousands of empty/binary files remain bounded.
        *remaining -= 1;
        if let Some(raw) = raw {
            'hunks: for h in 0..raw.num_hunks() {
                let (info, count) = raw.hunk(h)?;
                let mut hunk = PatchHunk {
                    old_start: info.old_start(),
                    old_lines: info.old_lines(),
                    new_start: info.new_start(),
                    new_lines: info.new_lines(),
                    header: info.header().to_vec(),
                    lines: Vec::new(),
                };
                for l in 0..count {
                    if *remaining == 0 {
                        result.truncated = true;
                        file.hunks.push(hunk);
                        break 'hunks;
                    }
                    let line = raw.line_in_hunk(h, l)?;
                    let kind = match line.origin() {
                        '+' => PatchLineKind::Added,
                        '-' => PatchLineKind::Removed,
                        ' ' => PatchLineKind::Context,
                        _ => PatchLineKind::NoNewline,
                    };
                    hunk.lines.push(PatchLine {
                        kind,
                        old_line: line.old_lineno(),
                        new_line: line.new_lineno(),
                        content: line.content().to_vec(),
                    });
                    *remaining -= 1;
                }
                file.hunks.push(hunk);
            }
            if !result.truncated && !file.binary {
                let mut raw = raw;
                file.encoded = Some(raw.to_buf()?.to_vec());
            }
        }
        result.files.push(file);
    }
    Ok(())
}

pub(super) fn stage(
    repo: &mut Repository,
    patch: &Patch,
    file: usize,
    hunks: &[usize],
) -> Result<(), GitError> {
    let ids: Vec<_> = hunks
        .iter()
        .map(|hunk| HunkId { file, hunk: *hunk })
        .collect();
    apply(repo, patch, &ids, PatchDirection::Stage)
}

pub(super) fn apply(
    repo: &mut Repository,
    patch: &Patch,
    ids: &[HunkId],
    direction: PatchDirection,
) -> Result<(), GitError> {
    patch.validate_selection(ids, direction)?;
    let fresh = load(repo, patch.request.clone())?;
    if fresh.truncated
        || ids
            .iter()
            .any(|id| fresh.files.get(id.file) != patch.files.get(id.file))
    {
        return Err(GitError::StalePatch(
            "patch changed; reload before applying hunks".into(),
        ));
    }
    let bytes = selected_bytes(patch, ids)?;
    apply_bytes(repo, &bytes, direction)
}

pub(super) fn apply_lines(
    repo: &mut Repository,
    patch: &Patch,
    ids: &[LineId],
    direction: PatchDirection,
) -> Result<(), GitError> {
    patch.validate_lines(ids, direction)?;
    let fresh = load(repo, patch.request.clone())?;
    if fresh.truncated
        || ids
            .iter()
            .any(|id| fresh.files.get(id.file) != patch.files.get(id.file))
    {
        return Err(GitError::StalePatch(
            "patch changed; reload before applying lines".into(),
        ));
    }
    let bytes = lines::selected_bytes(patch, ids, direction)?;
    apply_bytes(repo, &bytes, direction)
}

fn apply_bytes(
    repo: &mut Repository,
    bytes: &[u8],
    direction: PatchDirection,
) -> Result<(), GitError> {
    let workdir = repo.workdir().ok_or(GitError::InvalidState)?;
    for check in [true, false] {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut command = Command::new("git");
        command
            .arg("apply")
            .arg("--whitespace=nowarn")
            .arg("--unidiff-zero");
        if direction != PatchDirection::Discard {
            command.arg("--cached");
        }
        if direction != PatchDirection::Stage {
            command.arg("--reverse");
        }
        if check {
            command.arg("--check");
        }
        let mut child = command
            .current_dir(workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(GitError::from)?;
        let sent = child
            .stdin
            .take()
            .ok_or(GitError::InvalidState)?
            .write_all(bytes);
        let output = child.wait_with_output().map_err(GitError::from)?;
        if !output.status.success() {
            return Err(GitError::process(output));
        }
        sent.map_err(GitError::from)?;
    }
    repo.index()?.read(true)?;
    Ok(())
}

fn selected_bytes(patch: &Patch, ids: &[HunkId]) -> Result<Vec<u8>, GitError> {
    let mut bytes = Vec::new();
    let files: std::collections::BTreeSet<_> = ids.iter().map(|id| id.file).collect();
    for index in files {
        let file = &patch.files[index];
        let diff = git2::Diff::from_buffer(file.encoded.as_deref().ok_or(GitError::InvalidState)?)?;
        diff.print(git2::DiffFormat::Patch, |_, hunk, line| {
            let chosen = hunk.is_none_or(|hunk| {
                ids.iter().filter(|id| id.file == index).any(|id| {
                    let selected = &file.hunks[id.hunk];
                    (
                        hunk.old_start(),
                        hunk.old_lines(),
                        hunk.new_start(),
                        hunk.new_lines(),
                    ) == (
                        selected.old_start,
                        selected.old_lines,
                        selected.new_start,
                        selected.new_lines,
                    )
                })
            });
            if chosen {
                if matches!(line.origin(), '+' | '-' | ' ') {
                    bytes.push(line.origin() as u8);
                }
                bytes.extend_from_slice(line.content());
            }
            true
        })?;
    }
    Ok(bytes)
}
