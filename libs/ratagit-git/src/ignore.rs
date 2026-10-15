use crate::{GitError, validate_repo_relative_path};
use std::{fs, io::Write, path::Path};

pub(crate) fn append(root: &Path, path: &str) -> Result<(), GitError> {
    validate_repo_relative_path(path)?;
    if path.contains(['\n', '\r', '\0']) {
        return Err(GitError::new("ignore paths cannot contain newlines"));
    }
    let target = root.join(".gitignore");
    if fs::symlink_metadata(&target).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(GitError::new("refusing to modify a symlinked .gitignore"));
    }
    let mut pattern = String::from("/");
    for ch in path.chars() {
        if matches!(ch, '\\' | '*' | '?' | '[' | ']' | '!' | '#' | ' ') {
            pattern.push('\\');
        }
        pattern.push(ch);
    }
    let old = match fs::read(&target) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    if old
        .split(|byte| *byte == b'\n')
        .any(|line| line.strip_suffix(b"\r").unwrap_or(line) == pattern.as_bytes())
    {
        return Ok(());
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)?;
    if !old.is_empty() && !old.ends_with(b"\n") {
        file.write_all(b"\n")?;
    }
    writeln!(file, "{pattern}")?;
    Ok(())
}
