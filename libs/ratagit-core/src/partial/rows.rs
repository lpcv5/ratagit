use super::*;

pub fn patch_rows(patch: &Patch) -> Vec<PatchRow> {
    if let Some(rows) = &patch.preview {
        return rows.clone();
    }
    let mut rows = Vec::new();
    for (file, entry) in patch.files.iter().enumerate() {
        rows.extend(
            String::from_utf8_lossy(&entry.header)
                .lines()
                .map(|line| header(line.to_string())),
        );
        for (hunk, entry) in entry.hunks.iter().enumerate() {
            rows.push(header(
                String::from_utf8_lossy(&entry.header)
                    .trim_end_matches(['\r', '\n'])
                    .to_string(),
            ));
            for (line, entry) in entry.lines.iter().enumerate() {
                let content = if entry.kind == PatchLineKind::NoNewline {
                    entry.content.strip_prefix(b"\n").unwrap_or(&entry.content)
                } else {
                    &entry.content
                };
                let text = String::from_utf8_lossy(content)
                    .trim_end_matches(['\r', '\n'])
                    .chars()
                    .map(|c| {
                        if c.is_control() && c != '\t' {
                            '\u{fffd}'
                        } else {
                            c
                        }
                    })
                    .collect();
                rows.push(PatchRow {
                    text,
                    id: (entry.kind != PatchLineKind::NoNewline).then_some(LineId {
                        file,
                        hunk,
                        line,
                    }),
                    kind: entry.kind,
                    old_line: entry.old_line,
                    new_line: entry.new_line,
                });
            }
        }
    }
    if patch.truncated {
        rows.push(header("Patch truncated; partial changes disabled".into()));
    }
    rows
}

fn header(text: String) -> PatchRow {
    PatchRow {
        text,
        id: None,
        kind: PatchLineKind::Context,
        old_line: None,
        new_line: None,
    }
}
