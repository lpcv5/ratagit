//! Build a partial patch from the exact reviewed bytes, never display strings.
use crate::git::patch::*;
use crate::git::GitError;
use std::collections::BTreeSet;

pub(super) fn selected_bytes(
    patch: &Patch,
    ids: &[LineId],
    direction: PatchDirection,
) -> Result<Vec<u8>, GitError> {
    let selected: BTreeSet<_> = ids.iter().copied().collect();
    let mut output = Vec::new();
    for file_id in ids.iter().map(|id| id.file).collect::<BTreeSet<_>>() {
        let file = &patch.files[file_id];
        let all_selected = patch
            .line_ids()
            .iter()
            .filter(|id| id.file == file_id)
            .all(|id| selected.contains(id));
        let becomes_modified = !all_selected
            && match file.change {
                PatchChange::Added => direction != PatchDirection::Stage,
                PatchChange::Deleted => direction == PatchDirection::Stage,
                _ => false,
            };
        let encoded = file.encoded.as_deref().ok_or(GitError::InvalidState)?;
        let headers: Vec<_> = encoded
            .split_inclusive(|byte| *byte == b'\n')
            .take_while(|line| !line.starts_with(b"@@ "))
            .collect();
        for header in &headers {
            // Object IDs describe the whole original change, not this subset.
            if header.starts_with(b"index ") {
                continue;
            }
            if becomes_modified
                && (header.starts_with(b"new file mode ")
                    || header.starts_with(b"deleted file mode "))
            {
                continue;
            }
            if becomes_modified
                && (header.starts_with(b"--- /dev/null") || header.starts_with(b"+++ /dev/null"))
            {
                let old = header.starts_with(b"--- ");
                let source = headers
                    .iter()
                    .find_map(|line| line.strip_prefix(if old { b"+++ " } else { b"--- " }))
                    .ok_or(GitError::InvalidState)?;
                let mut path = source.to_vec();
                let prefix = usize::from(path.first() == Some(&b'"'));
                if path.get(prefix + 1) != Some(&b'/') {
                    return Err(GitError::InvalidState);
                }
                path[prefix] = if old { b'a' } else { b'b' };
                output.extend_from_slice(if old { b"--- " } else { b"+++ " });
                output.extend_from_slice(&path);
            } else {
                output.extend_from_slice(header);
            }
        }
        let mut delta = 0i64;
        for (hunk_id, hunk) in file.hunks.iter().enumerate() {
            if !ids
                .iter()
                .any(|id| id.file == file_id && id.hunk == hunk_id)
            {
                continue;
            }
            let mut body = Vec::new();
            let mut old_count = 0i64;
            let mut new_count = 0i64;
            for (line_id, line) in hunk.lines.iter().enumerate() {
                let chosen = selected.contains(&LineId {
                    file: file_id,
                    hunk: hunk_id,
                    line: line_id,
                });
                let prefix = match (line.kind, chosen, direction == PatchDirection::Stage) {
                    (PatchLineKind::Context, _, _) => b' ',
                    (PatchLineKind::Removed, true, _) => b'-',
                    (PatchLineKind::Added, true, _) => b'+',
                    (PatchLineKind::Removed, false, true)
                    | (PatchLineKind::Added, false, false) => b' ',
                    _ => continue,
                };
                old_count += i64::from(prefix != b'+');
                new_count += i64::from(prefix != b'-');
                body.push(prefix);
                body.extend_from_slice(&line.content);
                // libgit2 exposes EOF markers as separate pseudo-lines. Emit
                // them only for retained lines and preserve their exact bytes.
                if !line.content.ends_with(b"\n") {
                    body.extend_from_slice(b"\n\\ No newline at end of file\n");
                }
            }
            let (old_boundary, new_boundary) = if direction == PatchDirection::Stage {
                let old = i64::from(hunk.old_start) - i64::from(hunk.old_lines > 0);
                (old, old + delta)
            } else {
                let new = i64::from(hunk.new_start) - i64::from(hunk.new_lines > 0);
                (new - delta, new)
            };
            if old_boundary < 0 || new_boundary < 0 {
                return Err(GitError::InvalidState);
            }
            output.extend_from_slice(
                format!(
                    "@@ -{},{} +{},{} @@\n",
                    old_boundary + i64::from(old_count > 0),
                    old_count,
                    new_boundary + i64::from(new_count > 0),
                    new_count
                )
                .as_bytes(),
            );
            output.extend_from_slice(&body);
            delta += new_count - old_count;
        }
    }
    Ok(output)
}
