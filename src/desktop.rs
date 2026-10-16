//! External application and clipboard dependencies, outside reducer-owned state.
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

pub trait Desktop: Send + Sync {
    fn copy_text(&self, text: &str) -> io::Result<()>;
}

#[derive(Default)]
pub struct SystemDesktop {
    // Keep the clipboard owner alive on X11; serialize access on Windows.
    clipboard: Mutex<Option<arboard::Clipboard>>,
}

impl Desktop for SystemDesktop {
    fn copy_text(&self, text: &str) -> io::Result<()> {
        let mut clipboard = self
            .clipboard
            .lock()
            .map_err(|_| io::Error::other("clipboard lock poisoned"))?;
        if clipboard.is_none() {
            *clipboard =
                Some(arboard::Clipboard::new().map_err(|e| io::Error::other(e.to_string()))?);
        }
        clipboard
            .as_mut()
            .unwrap()
            .set_text(text)
            .map_err(|e| io::Error::other(format!("clipboard: {e}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

pub struct CommitMessageFile {
    path: PathBuf,
}

impl CommitMessageFile {
    pub fn new() -> io::Result<Self> {
        use std::io::Write;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ratagit-commit-{}-{nonce}.txt", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        file.write_all(b"\n# Enter the commit message. Empty message cancels.\n")?;
        Ok(Self { path })
    }
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
    pub fn message(&self) -> io::Result<String> {
        Ok(std::fs::read_to_string(&self.path)?
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string())
    }
}
impl Drop for CommitMessageFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl EditorCommand {
    pub fn from_preferences(
        cwd: PathBuf,
        file: PathBuf,
        preferences: &[Option<String>],
    ) -> io::Result<Self> {
        let editor = preferences
            .iter()
            .filter_map(Option::as_deref)
            .find(|value| !value.trim().is_empty())
            .unwrap_or(if cfg!(windows) { "notepad" } else { "vi" });
        Self::new(editor, cwd, file)
    }

    pub fn new(editor: &str, cwd: PathBuf, file: PathBuf) -> io::Result<Self> {
        let mut words = if std::path::Path::new(editor).is_file() {
            vec![editor.to_string()]
        } else {
            shell_words::split(editor)
                .map_err(|e| io::Error::other(format!("invalid editor: {e}")))?
        };
        if words.is_empty() || words[0].is_empty() {
            return Err(io::Error::other("no editor configured"));
        }
        let program = words.remove(0);
        words.push(
            file.into_os_string()
                .into_string()
                .map_err(|_| io::Error::other("editor path is not UTF-8"))?,
        );
        Ok(Self {
            program,
            args: words,
            cwd,
        })
    }

    pub fn run(&self) -> io::Result<()> {
        let status = std::process::Command::new(&self.program)
            .args(&self.args)
            .current_dir(&self.cwd)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("editor exited with {status}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_precedence_selects_before_parsing_invalid_fallbacks() {
        let root = PathBuf::from("/repo");
        let file = root.join("message.txt");
        let command = EditorCommand::from_preferences(
            root.clone(),
            file.clone(),
            &[
                Some("git-editor --wait".into()),
                Some("configured-editor".into()),
                Some("'invalid visual".into()),
            ],
        )
        .unwrap();
        assert_eq!(command.program, "git-editor");
        assert_eq!(
            command.args,
            vec!["--wait".to_string(), file.to_string_lossy().to_string()]
        );
        let command = EditorCommand::from_preferences(
            root,
            file,
            &[
                Some(" ".into()),
                Some("configured-editor".into()),
                Some("'invalid visual".into()),
            ],
        )
        .unwrap();
        assert_eq!(command.program, "configured-editor");
    }

    #[test]
    fn commit_editor_message_strips_comments_keeps_unicode_body_and_cleans_up() {
        let message_file = CommitMessageFile::new().unwrap();
        let path = message_file.path().to_path_buf();
        std::fs::write(&path, "# comment\n标题\n\n正文\n# tail\n").unwrap();
        assert_eq!(message_file.message().unwrap(), "标题\n\n正文");
        drop(message_file);
        assert!(!path.exists());
    }

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
