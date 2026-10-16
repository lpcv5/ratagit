//! Complete native whole-file bytes and aggregate reviewed partial selections.
use super::*;
use ratagit_core::custom_patch::{CustomPatch, Inclusion, MAX_BYTES, MAX_FILES, file_key};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn load(repo: &mut Repository, request: PatchRequest) -> Result<Patch, GitError> {
    let PatchRequest::CommitFiles { oid, paths } = &request else {
        return Err(GitError::new(
            "custom patch loading requires captured commit paths",
        ));
    };
    if paths.is_empty() || paths.len() > MAX_FILES * 2 {
        return Err(GitError::new(
            "custom patch file selection is empty or too large",
        ));
    }
    let commit = repo.find_commit(git2::Oid::from_str(oid)?)?;
    let tree = commit.tree()?;
    let parent = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };
    let mut opts = options(None)?;
    opts.show_binary(true).disable_pathspec_match(true);
    for path in paths {
        crate::validate_repo_relative_path(
            path.to_str()
                .ok_or_else(|| GitError::new("patch path is not UTF-8"))?,
        )?;
        opts.pathspec(path);
    }
    let mut diff = repo.diff_tree_to_tree(parent.as_ref(), Some(&tree), Some(&mut opts))?;
    diff.find_similar(None)?;
    let mut encoded = BTreeMap::<String, Vec<u8>>::new();
    let mut size = 0usize;
    let printed = diff.print(git2::DiffFormat::Patch, |delta, _, line| {
        let key = if delta.status() == git2::Delta::Deleted {
            delta.old_file().path()
        } else {
            delta.new_file().path()
        }
        .and_then(|p| p.to_str());
        let Some(key) = key else {
            return false;
        };
        let prefix = matches!(line.origin(), '+' | '-' | ' ');
        size = size.saturating_add(line.content().len() + usize::from(prefix));
        if size > MAX_BYTES {
            return false;
        }
        let file = encoded.entry(key.to_string()).or_default();
        if prefix {
            file.push(line.origin() as u8);
        }
        file.extend_from_slice(line.content());
        true
    });
    if size > MAX_BYTES {
        return Err(GitError::new("custom patch exceeds the 16 MiB limit"));
    }
    printed?;
    let mut patch = Patch {
        request: request.clone(),
        files: Vec::new(),
        truncated: false,
        preview: None,
    };
    let mut remaining = MAX_PATCH_LINES;
    append(
        &diff,
        PatchSource::Commit(oid.clone()),
        &mut patch,
        &mut remaining,
    )?;
    if patch.truncated {
        return Err(GitError::new(
            "custom patch source exceeds the native line limit; select fewer files",
        ));
    }
    for file in &mut patch.files {
        let key = file_key(file).map_err(GitError::new)?.to_string();
        let bytes = encoded
            .remove(&key)
            .ok_or_else(|| GitError::new("missing complete native file patch"))?;
        if bytes.is_empty() {
            return Err(GitError::new("empty native file patch"));
        }
        if file.header.is_empty() {
            file.header = bytes
                .split_inclusive(|b| *b == b'\n')
                .take_while(|line| {
                    !line.starts_with(b"GIT binary patch") && !line.starts_with(b"@@ ")
                })
                .flatten()
                .copied()
                .collect();
        }
        file.encoded = Some(bytes);
    }
    Ok(patch)
}

pub(crate) fn bytes(patch: &CustomPatch, reverse_base: bool) -> Result<Vec<u8>, GitError> {
    patch.validate().map_err(GitError::new)?;
    let mut output = Vec::new();
    for selected in &patch.files {
        match &selected.inclusion {
            Inclusion::Whole => output.extend_from_slice(
                selected
                    .file
                    .encoded
                    .as_ref()
                    .ok_or_else(|| GitError::new("missing native file bytes"))?,
            ),
            Inclusion::Lines(lines) => {
                let source = Patch {
                    request: PatchRequest::CommitFiles {
                        oid: patch.oid.clone(),
                        paths: vec![file_key(&selected.file).map_err(GitError::new)?.into()],
                    },
                    files: vec![selected.file.clone()],
                    truncated: false,
                    preview: None,
                };
                let ids = lines
                    .iter()
                    .map(|&(hunk, line)| LineId {
                        file: 0,
                        hunk,
                        line,
                    })
                    .collect::<Vec<_>>();
                output.extend(lines::selected_bytes(
                    &source,
                    &ids,
                    if reverse_base {
                        PatchDirection::Discard
                    } else {
                        PatchDirection::Stage
                    },
                )?);
            }
        }
        if output.len() > MAX_BYTES {
            return Err(GitError::new("custom patch exceeds the 16 MiB limit"));
        }
    }
    Ok(output)
}

pub(crate) fn checked_bytes(
    repo: &mut Repository,
    patch: &CustomPatch,
    reverse_base: bool,
) -> Result<Vec<u8>, GitError> {
    patch.validate().map_err(GitError::new)?;
    let paths = patch
        .files
        .iter()
        .flat_map(|selected| {
            [
                selected.file.old.path.clone(),
                selected.file.new.path.clone(),
            ]
        })
        .flatten()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let fresh = load(
        repo,
        PatchRequest::CommitFiles {
            oid: patch.oid.clone(),
            paths,
        },
    )?;
    for selected in &patch.files {
        let key = file_key(&selected.file).map_err(GitError::new)?;
        if !fresh
            .files
            .iter()
            .any(|file| file_key(file).ok() == Some(key) && file == &selected.file)
        {
            return Err(GitError::new(
                "custom patch source changed; reset and select again",
            ));
        }
    }
    bytes(patch, reverse_base)
}
