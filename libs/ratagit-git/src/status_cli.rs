use ratagit_core::FileEntry;

use crate::GitError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedStatus {
    pub(crate) files: Vec<FileEntry>,
    pub(crate) truncated: bool,
}

pub(crate) fn parse_porcelain_v1_z_limited(
    output: &[u8],
    max_entries: usize,
) -> Result<ParsedStatus, GitError> {
    let mut files = Vec::new();
    let mut records = output.split(|byte| *byte == 0).peekable();
    let mut truncated = false;
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        if files.len() >= max_entries {
            truncated = true;
            break;
        }
        let Some((status, path)) = parse_record(record)? else {
            continue;
        };
        if status.is_rename_or_copy() {
            let _ = records.next();
        }
        if status.is_ignored() {
            continue;
        }
        files.push(FileEntry {
            path,
            staged: status.is_staged(),
            untracked: status.is_untracked(),
        });
    }
    Ok(ParsedStatus { files, truncated })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PorcelainStatus {
    index: u8,
    worktree: u8,
}

impl PorcelainStatus {
    fn is_untracked(self) -> bool {
        self.index == b'?' && self.worktree == b'?'
    }

    fn is_ignored(self) -> bool {
        self.index == b'!' && self.worktree == b'!'
    }

    fn is_staged(self) -> bool {
        matches!(self.index, b'A' | b'M' | b'D' | b'R' | b'C' | b'T' | b'U')
    }

    fn is_rename_or_copy(self) -> bool {
        matches!(self.index, b'R' | b'C') || matches!(self.worktree, b'R' | b'C')
    }
}

fn parse_record(record: &[u8]) -> Result<Option<(PorcelainStatus, String)>, GitError> {
    if record.len() < 4 || record[2] != b' ' {
        return Err(GitError::new("invalid git status porcelain record"));
    }
    let status = PorcelainStatus {
        index: record[0],
        worktree: record[1],
    };
    let path = String::from_utf8(record[3..].to_vec())
        .map_err(|error| GitError::new(format!("invalid utf-8 path from git status: {error}")))?;
    Ok(Some((status, path.replace('\\', "/"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &[u8]) -> Vec<FileEntry> {
        parse_porcelain_v1_z_limited(input, usize::MAX)
            .expect("porcelain should parse")
            .files
    }

    #[test]
    fn parses_modified_staged_untracked_conflict_and_unicode_paths() {
        let files = parse(
            b" M src/lib.rs\0A  staged.txt\0?? nested/new.txt\0UU conflict.txt\0 M docs/\xe4\xbd\xa0\xe5\xa5\xbd.md\0",
        );

        assert_eq!(
            files,
            vec![
                FileEntry {
                    path: "src/lib.rs".to_string(),
                    staged: false,
                    untracked: false,
                },
                FileEntry {
                    path: "staged.txt".to_string(),
                    staged: true,
                    untracked: false,
                },
                FileEntry {
                    path: "nested/new.txt".to_string(),
                    staged: false,
                    untracked: true,
                },
                FileEntry {
                    path: "conflict.txt".to_string(),
                    staged: true,
                    untracked: false,
                },
                FileEntry {
                    path: "docs/你好.md".to_string(),
                    staged: false,
                    untracked: false,
                },
            ]
        );
    }

    #[test]
    fn rename_and_copy_entries_use_target_path_and_skip_origin_path() {
        let files = parse(b"R  new-name.txt\0old-name.txt\0C  copy.txt\0source.txt\0");

        assert_eq!(
            files,
            vec![
                FileEntry {
                    path: "new-name.txt".to_string(),
                    staged: true,
                    untracked: false,
                },
                FileEntry {
                    path: "copy.txt".to_string(),
                    staged: true,
                    untracked: false,
                },
            ]
        );
    }

    #[test]
    fn ignored_entries_are_dropped() {
        assert!(parse(b"!! target/debug/app.exe\0").is_empty());
    }

    #[test]
    fn invalid_record_is_rejected() {
        assert!(parse_porcelain_v1_z_limited(b"M missing-space\0", usize::MAX).is_err());
    }

    #[test]
    fn parsing_can_stop_at_entry_limit() {
        let parsed = parse_porcelain_v1_z_limited(b" M a.txt\0 M b.txt\0", 1)
            .expect("porcelain should parse");

        assert_eq!(parsed.files.len(), 1);
        assert!(parsed.truncated);
    }
}
