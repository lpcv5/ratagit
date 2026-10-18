use super::*;
use crate::{AppContext, patch::PatchSource};

pub fn capture(state: &AppContext) -> Result<Box<CustomPatch>, String> {
    let oid = state
        .ui
        .custom_patch
        .source
        .clone()
        .ok_or("No custom patch; select files or hunks with Space first")?;
    let files = state
        .ui
        .custom_patch
        .included
        .iter()
        .map(|(key, inclusion)| {
            Ok(FileSelection {
                file: state
                    .repo
                    .custom_patch
                    .files
                    .get(key)
                    .ok_or("missing custom patch source file")?
                    .clone(),
                inclusion: inclusion.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let patch = Box::new(CustomPatch { oid, files });
    patch.validate()?;
    Ok(patch)
}

pub(super) fn files(state: &mut AppContext, patch: &Patch) -> Result<(), String> {
    if patch.truncated || source(&patch.request) != state.ui.custom_patch.source.as_deref() {
        return Err("custom patch requires a complete matching source".into());
    }
    let adding = patch.files.iter().any(|file| {
        file_key(file)
            .ok()
            .and_then(|key| state.ui.custom_patch.included.get(key))
            != Some(&Inclusion::Whole)
    });
    change(state, patch, |included| {
        for file in &patch.files {
            let key = file_key(file)?.to_string();
            if adding {
                included.insert(key, Inclusion::Whole);
            } else {
                included.remove(&key);
            }
        }
        Ok(())
    })
}

pub(super) fn lines(state: &mut AppContext, patch: &Patch, ids: &[LineId]) -> Result<(), String> {
    crate::workflow::commit_files::validate(
        patch,
        ids,
        crate::workflow::commit_files::PatchAction::Apply,
    )?;
    let first = ids.first().ok_or("No changed lines selected")?;
    let removing = line_included(state, &patch.files[first.file], first.hunk, first.line);
    let mut by_file = BTreeMap::<usize, BTreeSet<(usize, usize)>>::new();
    for id in ids {
        by_file
            .entry(id.file)
            .or_default()
            .insert((id.hunk, id.line));
    }
    change(state, patch, |included| {
        for (file_id, requested) in by_file {
            let file = &patch.files[file_id];
            let key = file_key(file)?.to_string();
            let all = line_ids(file);
            let mut selected = match included.get(&key) {
                Some(Inclusion::Whole) => all.clone(),
                Some(Inclusion::Lines(lines)) => lines.clone(),
                None => BTreeSet::new(),
            };
            for position in requested {
                if removing {
                    selected.remove(&position);
                } else {
                    selected.insert(position);
                }
            }
            if selected.is_empty() {
                included.remove(&key);
            } else {
                included.insert(
                    key,
                    if selected == all {
                        Inclusion::Whole
                    } else {
                        Inclusion::Lines(selected)
                    },
                );
            }
        }
        Ok(())
    })
}

fn change(
    state: &mut AppContext,
    patch: &Patch,
    edit: impl FnOnce(&mut BTreeMap<String, Inclusion>) -> Result<(), String>,
) -> Result<(), String> {
    let mut files = state.repo.custom_patch.files.clone();
    for file in &patch.files {
        let key = file_key(file)?.to_string();
        if files.get(&key).is_some_and(|old| old != file) {
            return Err("custom patch source changed; reset it before selecting again".into());
        }
        files.insert(key, file.clone());
    }
    let mut included = state.ui.custom_patch.included.clone();
    edit(&mut included)?;
    files.retain(|key, _| included.contains_key(key));
    if !included.is_empty() {
        CustomPatch {
            oid: source(&patch.request).ok_or("not a commit patch")?.into(),
            files: included
                .iter()
                .map(|(key, inclusion)| FileSelection {
                    file: files[key].clone(),
                    inclusion: inclusion.clone(),
                })
                .collect(),
        }
        .validate()?;
    }
    state.repo.custom_patch.files = files;
    state.repo.custom_patch.preview.clear();
    state.ui.custom_patch.included = included;
    Ok(())
}

pub fn line_included(state: &AppContext, file: &PatchFile, hunk: usize, line: usize) -> bool {
    let PatchSource::Commit(oid) = &file.source else {
        return false;
    };
    if state.ui.custom_patch.source.as_ref() != Some(oid) {
        return false;
    }
    match file_key(file)
        .ok()
        .and_then(|key| state.ui.custom_patch.included.get(key))
    {
        Some(Inclusion::Whole) => true,
        Some(Inclusion::Lines(lines)) => lines.contains(&(hunk, line)),
        None => false,
    }
}

/// 0 absent, 1 partial, 2 whole. Directory inclusion aggregates descendants.
pub fn file_inclusion(state: &AppContext, oid: &str, path: &str, directory: bool) -> u8 {
    if state.ui.custom_patch.source.as_deref() != Some(oid) {
        return 0;
    }
    if !directory {
        return match state.ui.custom_patch.included.get(path) {
            Some(Inclusion::Whole) => 2,
            Some(_) => 1,
            None => 0,
        };
    }
    let items = if state.ui.commits.files.commit_id.as_deref() == Some(oid) {
        &state.repo.commits.files.items
    } else if state.ui.branches.commit_files.commit_id.as_deref() == Some(oid) {
        &state.repo.branches.commit_files.items
    } else {
        return 0;
    };
    let prefix = format!("{path}/");
    let statuses = items
        .iter()
        .filter(|f| f.path.starts_with(&prefix))
        .map(|f| file_inclusion(state, oid, &f.path, false))
        .collect::<Vec<_>>();
    if !statuses.is_empty() && statuses.iter().all(|v| *v == 2) {
        2
    } else {
        u8::from(statuses.iter().any(|v| *v != 0))
    }
}
