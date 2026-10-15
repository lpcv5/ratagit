//! Optional per-job timing log shared by both worker lanes.

use std::sync::{Mutex, OnceLock};
use tracing::debug;

/// Set to Some(path) before any Git2Repository is constructed to enable
/// per-job timing logs written to that file.  Controlled by --debug flag.
static GIT_JOB_LOG: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

/// Enable git job timing logs, written to `path`.
/// Call this once at startup before any repository is opened.
pub fn enable_git_job_log(path: &str) {
    use std::fs::OpenOptions;
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
    {
        let _ = GIT_JOB_LOG.set(Mutex::new(file));
    }
}

pub(super) fn write_git_job_log(msg: &str) {
    debug!("{}", msg);
    if let Some(lock) = GIT_JOB_LOG.get() {
        if let Ok(mut file) = lock.lock() {
            use std::io::Write;
            let _ = writeln!(file, "{}", msg);
            let _ = file.flush();
        }
    }
}
