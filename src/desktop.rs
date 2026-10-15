//! External application and clipboard dependencies, outside reducer-owned state.
use crate::git::GitError;
use std::path::PathBuf;
use std::sync::Mutex;

pub trait Desktop: Send + Sync {
    fn copy_text(&self, text: &str) -> Result<(), GitError>;
}

#[derive(Default)]
pub struct SystemDesktop {
    // Keep the clipboard owner alive on X11; serialize access on Windows.
    clipboard: Mutex<Option<arboard::Clipboard>>,
}

impl Desktop for SystemDesktop {
    fn copy_text(&self, text: &str) -> Result<(), GitError> {
        let mut clipboard = self
            .clipboard
            .lock()
            .map_err(|_| GitError::Worker("clipboard lock poisoned".into()))?;
        if clipboard.is_none() {
            *clipboard =
                Some(arboard::Clipboard::new().map_err(|e| GitError::Git2(e.to_string()))?);
        }
        clipboard
            .as_mut()
            .unwrap()
            .set_text(text)
            .map_err(|e| GitError::Git2(format!("clipboard: {e}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

impl EditorCommand {
    pub fn new(editor: &str, cwd: PathBuf, file: PathBuf) -> Result<Self, GitError> {
        let mut words = if std::path::Path::new(editor).is_file() {
            vec![editor.to_string()]
        } else {
            shell_words::split(editor)
                .map_err(|e| GitError::Git2(format!("invalid editor: {e}")))?
        };
        if words.is_empty() || words[0].is_empty() {
            return Err(GitError::Git2("no editor configured".into()));
        }
        let program = words.remove(0);
        words.push(
            file.into_os_string()
                .into_string()
                .map_err(|_| GitError::Git2("editor path is not UTF-8".into()))?,
        );
        Ok(Self {
            program,
            args: words,
            cwd,
        })
    }

    pub fn run(&self) -> Result<(), GitError> {
        let status = std::process::Command::new(&self.program)
            .args(&self.args)
            .current_dir(&self.cwd)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(GitError::Git2(format!("editor exited with {status}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_arguments_preserve_spaces_unicode_and_shell_metacharacters() {
        let file = PathBuf::from("/repo/目录/a file;$(echo bad).txt");
        let editor = EditorCommand::new(
            "'C:/Program Files/Editor/editor.exe' --wait --reuse-window",
            "/repo".into(),
            file.clone(),
        )
        .unwrap();
        assert_eq!(editor.program, "C:/Program Files/Editor/editor.exe");
        assert_eq!(
            editor.args,
            ["--wait", "--reuse-window", file.to_str().unwrap()]
        );
        assert!(EditorCommand::new("", "/repo".into(), file.clone()).is_err());
        assert!(EditorCommand::new("'unterminated", "/repo".into(), file).is_err());
    }
}
