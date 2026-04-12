use anyhow::Result;
use git2::DiffOptions;

use super::commits::CommitEntry;
use super::repo::GitRepo;
use crate::components::core::GitFileStatus;

/// 获取某个 commit 修改的文件列表及其状态
pub fn get_commit_files(
    repo: &GitRepo,
    commit: &CommitEntry,
) -> Result<Vec<(String, GitFileStatus)>> {
    let commit_obj = repo.repo.find_commit(git2::Oid::from_str(&commit.id)?)?;
    let tree = commit_obj.tree()?;

    // 获取父提交的树
    let parent_tree = if commit_obj.parent_count() > 0 {
        let parent = commit_obj.parent(0)?;
        Some(parent.tree()?)
    } else {
        // 初始提交，比较空树
        None
    };

    let mut options = DiffOptions::new();
    let diff = match parent_tree {
        Some(ref parent) => {
            repo.repo
                .diff_tree_to_tree(Some(parent), Some(&tree), Some(&mut options))?
        }
        None => repo
            .repo
            .diff_tree_to_tree(None, Some(&tree), Some(&mut options))?,
    };

    let mut files = Vec::new();

    diff.foreach(
        &mut |delta, _progress| {
            let path = delta
                .new_file()
                .path()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();

            let status = match delta.status() {
                git2::Delta::Added => GitFileStatus::Added,
                git2::Delta::Deleted => GitFileStatus::Deleted,
                git2::Delta::Modified => GitFileStatus::Modified,
                git2::Delta::Renamed => GitFileStatus::Renamed,
                _ => GitFileStatus::Unmodified,
            };

            files.push((path, status));
            true
        },
        None,
        None,
        None,
    )?;

    // 按路径排序
    files.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(files)
}
