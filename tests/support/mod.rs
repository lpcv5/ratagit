//! Per-child environment isolation shared by binary and re-executed unit tests.
#![allow(dead_code)] // Shared support exposes APIs used by different test binaries.
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

pub struct ProcessFixture {
    root: tempfile::TempDir,
    pub work: PathBuf,
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl ProcessFixture {
    pub fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("ratagit-process-")
            .tempdir()
            .unwrap();
        for directory in ["work 树", "home", "config", "local", "tmp", "hooks"] {
            std::fs::create_dir(root.path().join(directory)).unwrap();
        }
        let hooks = root
            .path()
            .join("hooks")
            .to_string_lossy()
            .replace('\\', "/")
            .replace('"', "\\\"");
        std::fs::write(root.path().join("home/.gitconfig"), format!(
            "[user]\nname = Process Fixture\nemail = fixture@example.invalid\n[init]\ndefaultBranch = main\n[core]\nautocrlf = false\nsafecrlf = false\nhooksPath = \"{hooks}\"\n[commit]\ngpgSign = false\n"
        )).unwrap();
        std::fs::write(root.path().join("system.gitconfig"), "").unwrap();
        Self {
            work: root.path().join("work 树"),
            root,
        }
    }

    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.path().join(relative)
    }

    pub fn command(&self, program: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(program);
        self.isolate(&mut command);
        command
    }

    pub fn isolate(&self, command: &mut Command) {
        // Keep executable/DLL lookup and Windows system services available. All
        // Git, app, test-harness and user configuration variables start empty.
        command.env_clear();
        for key in [
            "PATH",
            "SystemRoot",
            "WINDIR",
            "COMSPEC",
            "PATHEXT",
            "SystemDrive",
            "LD_LIBRARY_PATH",
            "DYLD_LIBRARY_PATH",
            "DYLD_FALLBACK_LIBRARY_PATH",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
            .current_dir(&self.work)
            .env("HOME", self.path("home"))
            .env("USERPROFILE", self.path("home"))
            .env("APPDATA", self.path("config"))
            .env("LOCALAPPDATA", self.path("local"))
            .env("XDG_CONFIG_HOME", self.path("config"))
            .env("XDG_CACHE_HOME", self.path("local"))
            .env("XDG_DATA_HOME", self.path("local"))
            .env("TMPDIR", self.path("tmp"))
            .env("TEMP", self.path("tmp"))
            .env("TMP", self.path("tmp"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_SYSTEM", self.path("system.gitconfig"))
            .env("GIT_CONFIG_GLOBAL", self.path("home/.gitconfig"))
            .env("GIT_CONFIG_COUNT", "0")
            .env("GIT_ATTR_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", self.root.path())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_DATE", "2000-01-01T00:00:00+0000")
            .env("GIT_COMMITTER_DATE", "2000-01-01T00:00:00+0000")
            .env("RUST_BACKTRACE", "0")
            .env("RUST_LIB_BACKTRACE", "0")
            .env("NO_COLOR", "1")
            .env("TERM", "dumb");
    }

    pub fn run(&self, command: &mut Command) -> Output {
        let mut child = ChildGuard(
            command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn isolated process"),
        );
        let stdout = child.0.stdout.take().unwrap();
        let stderr = child.0.stderr.take().unwrap();
        // Drain both pipes concurrently so a diagnostic burst cannot deadlock.
        let read = |mut pipe: Box<dyn Read + Send>| {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                pipe.read_to_end(&mut bytes).unwrap();
                bytes
            })
        };
        let stdout = read(Box::new(stdout));
        let stderr = read(Box::new(stderr));
        let deadline = Instant::now() + Duration::from_secs(30);
        let (status, timed_out) = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break (status, false);
            }
            if Instant::now() >= deadline {
                let _ = child.0.kill();
                break (child.0.wait().unwrap(), true);
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let output = Output {
            status,
            stdout: stdout.join().unwrap(),
            stderr: stderr.join().unwrap(),
        };
        assert!(
            !timed_out,
            "process deadline exceeded; stdout={} stderr={}",
            self.normalize(&output.stdout),
            self.normalize(&output.stderr)
        );
        output
    }

    pub fn normalize(&self, bytes: &[u8]) -> String {
        let root = self.root.path().to_string_lossy();
        let text = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
        // Preserve terminal controls so lifecycle assertions can detect leaks.
        // Relative filenames, native separators and diagnostic codes survive.
        text.replace(&format!("\\\\?\\{root}"), "<ROOT>")
            .replace(root.as_ref(), "<ROOT>")
            .replace(&root.replace('\\', "/"), "<ROOT>")
    }
}
