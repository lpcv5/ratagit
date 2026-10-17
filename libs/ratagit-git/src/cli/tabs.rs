use super::GitCli;
use crate::GitError;
use ratagit_core::RepositoryItem;

impl GitCli {
    pub(crate) fn tab_worktrees(&self) -> Result<Vec<RepositoryItem>, GitError> {
        let (bytes, _) = self
            .run_git_read_output_limited(["worktree", "list", "--porcelain", "-z"], 1024 * 1024)?;
        Ok(parse_worktrees(&bytes))
    }
}

fn parse_worktrees(bytes: &[u8]) -> Vec<RepositoryItem> {
    let mut result = Vec::new();
    let mut start = 0;
    for (index, pair) in bytes.windows(2).enumerate() {
        if pair != [0, 0] {
            continue;
        }
        let fields = bytes[start..index]
            .split(|byte| *byte == 0)
            .filter(|field| !field.is_empty())
            .map(|field| String::from_utf8_lossy(field).into_owned())
            .collect::<Vec<_>>();
        if !fields.is_empty() {
            result.push(parse_worktree(&fields, result.is_empty()));
        }
        start = index + 2;
        if result.len() == ratagit_core::tabs::REPOSITORY_LIST_LIMIT {
            break;
        }
    }
    result
}

fn parse_worktree(fields: &[String], main: bool) -> RepositoryItem {
    let value = |prefix: &str| {
        fields
            .iter()
            .find_map(|field| field.strip_prefix(prefix))
            .unwrap_or("")
            .to_owned()
    };
    let flag = |name: &str| {
        fields.iter().find_map(|field| {
            if field == name {
                Some(String::new())
            } else {
                field.strip_prefix(&format!("{name} ")).map(str::to_owned)
            }
        })
    };
    RepositoryItem::Worktree {
        path: value("worktree "),
        head: value("HEAD "),
        branch: value("branch ")
            .trim_start_matches("refs/heads/")
            .to_owned(),
        detached: fields.iter().any(|field| field == "detached"),
        locked: flag("locked"),
        prunable: flag("prunable"),
        main,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worktree_parser_requires_complete_record_boundary() {
        let complete = b"worktree /repo\0HEAD abc\0branch refs/heads/main\0\0";
        assert_eq!(parse_worktrees(complete).len(), 1);
        assert!(parse_worktrees(&complete[..complete.len() - 1]).is_empty());
        let mut truncated = complete.to_vec();
        truncated.extend_from_slice(b"worktree /linked\0HEAD def\0");
        assert_eq!(parse_worktrees(&truncated).len(), 1);
    }
}
