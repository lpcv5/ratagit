//! Whole-worktree operations and preparation of external file actions.
use super::Git2RepoInner;
use crate::desktop::EditorCommand;
use crate::git::GitError;
use std::io::Write;
use std::path::{Component, Path};

fn relative_path(path: &Path) -> Result<String, GitError> {
    if path.as_os_str().is_empty() || !path.components().all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(GitError::Git2("expected a repository-relative path".into()));
    }
    let parts = path
        .components()
        .map(|c| {
            c.as_os_str()
                .to_str()
                .ok_or_else(|| GitError::Git2("path is not UTF-8".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parts[0].eq_ignore_ascii_case(".git") || parts.iter().any(|p| p.contains(['\r', '\n', '\0']))
    {
        return Err(GitError::Git2("invalid file path".into()));
    }
    Ok(parts.join("/"))
}

impl Git2RepoInner {
    pub(super) fn toggle_stage_all(&self) -> Result<bool, GitError> {
        let status = self.status()?;
        if self.repo.index()?.has_conflicts() {
            return Err(GitError::Git2(
                "resolve conflicts before staging all files".into(),
            ));
        }
        if !status.unstaged.is_empty() || !status.untracked.is_empty() {
            self.run_git_owned(&["add".into(), "--all".into()])?;
            Ok(true)
        } else if !status.staged.is_empty() {
            let head = match self.repo.head() {
                Ok(head) => Some(head.peel(git2::ObjectType::Commit)?),
                Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
                Err(error) => return Err(error.into()),
            };
            self.repo.reset_default(head.as_ref(), ["*"])?;
            Ok(false)
        } else {
            Err(GitError::Git2("no changed files".into()))
        }
    }

    pub(super) fn ignore_file(&self, path: &Path) -> Result<(), GitError> {
        let relative = relative_path(path)?;
        let mut index = self.repo.index()?;
        index.read(true)?;
        let prefix = format!("{relative}/");
        if index
            .iter()
            .any(|e| e.path == relative.as_bytes() || e.path.starts_with(prefix.as_bytes()))
        {
            return Err(GitError::Git2(
                "only untracked files or directories can be ignored".into(),
            ));
        }
        let root = self.repo_root()?;
        let metadata = std::fs::symlink_metadata(root.join(path))?;
        let ignore_path = root.join(".gitignore");
        match std::fs::symlink_metadata(&ignore_path) {
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err(GitError::Git2(".gitignore must be a regular file".into()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if self.repo.is_path_ignored(path)? {
            return Ok(());
        }
        let existing = match std::fs::read_to_string(&ignore_path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let newline = if existing.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut pattern = String::from("/");
        for character in relative.chars() {
            if matches!(character, '\\' | '*' | '?' | '[' | ']' | '!' | '#' | ' ') {
                pattern.push('\\');
            }
            pattern.push(character);
        }
        if metadata.file_type().is_dir() {
            pattern.push('/');
        }
        let mut addition = String::new();
        if !existing.is_empty() && !existing.ends_with('\n') {
            addition.push_str(newline);
        }
        addition.push_str(&pattern);
        addition.push_str(newline);
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(ignore_path)?
            .write_all(addition.as_bytes())?;
        Ok(())
    }

    pub(super) fn prepare_editor(&self, path: &Path) -> Result<EditorCommand, GitError> {
        relative_path(path)?;
        let root = self.repo_root()?;
        let file = root.join(path);
        if !file.is_file() {
            return Err(GitError::Git2(
                "selected file does not exist or is a directory".into(),
            ));
        }
        let editor = self.run_git_owned(&["var".into(), "GIT_EDITOR".into()])?;
        EditorCommand::new(&editor, root, file)
    }
}
