use super::*;
use std::sync::Mutex;

type Calls = Arc<Mutex<Vec<&'static str>>>;

#[derive(Default)]
struct Failures {
    init: bool,
    enable: bool,
    disable: bool,
    restore: bool,
}

struct RecordingTerminal(Calls);
impl Drop for RecordingTerminal {
    fn drop(&mut self) {
        self.0.lock().unwrap().push("drop");
    }
}

struct RecordingOps {
    calls: Calls,
    failures: Failures,
}
impl RecordingOps {
    fn call(&self, name: &'static str, fail: bool) -> io::Result<()> {
        self.calls.lock().unwrap().push(name);
        if fail {
            Err(io::Error::other(format!("{name} error")))
        } else {
            Ok(())
        }
    }
}
impl TerminalOps for RecordingOps {
    type Terminal = RecordingTerminal;
    fn init(&mut self) -> io::Result<RecordingTerminal> {
        self.call("init", self.failures.init)?;
        Ok(RecordingTerminal(self.calls.clone()))
    }
    fn install_panic_hook(&mut self) {
        self.calls.lock().unwrap().push("hook");
    }
    fn enable_mouse(&mut self) -> io::Result<()> {
        self.call("enable", self.failures.enable)
    }
    fn disable_mouse(&mut self) -> io::Result<()> {
        self.call("disable", self.failures.disable)
    }
    fn restore(&mut self) -> io::Result<()> {
        self.call("restore", self.failures.restore)
    }
}

fn ops(failures: Failures) -> (RecordingOps, Calls) {
    let calls = Calls::default();
    (
        RecordingOps {
            calls: calls.clone(),
            failures,
        },
        calls,
    )
}

#[test]
fn test_finish_restores_mouse_then_terminal_then_drops_once() {
    let (ops, calls) = ops(Failures::default());
    let mut session = TerminalSession::with_ops(ops).unwrap();
    session.enable_mouse().unwrap();
    session.finish(Ok(())).unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        ["init", "hook", "enable", "disable", "restore", "drop"]
    );
}

#[test]
fn test_initialization_failure_restores_without_installing_mouse_hook() {
    let (ops, calls) = ops(Failures {
        init: true,
        restore: true,
        ..Default::default()
    });
    let error = match TerminalSession::with_ops(ops) {
        Err(error) => error,
        Ok(_) => panic!("initialization must fail"),
    };
    assert_eq!(error.to_string(), "init error");
    assert_eq!(*calls.lock().unwrap(), ["init", "restore"]);
}

#[test]
fn test_application_error_survives_both_cleanup_failures() {
    let (ops, calls) = ops(Failures {
        disable: true,
        restore: true,
        ..Default::default()
    });
    let session = TerminalSession::with_ops(ops).unwrap();
    let error = session
        .finish(Err(color_eyre::eyre::eyre!("application error")))
        .unwrap_err();
    assert_eq!(error.to_string(), "application error");
    assert_eq!(
        *calls.lock().unwrap(),
        ["init", "hook", "disable", "restore", "drop"]
    );
}

#[test]
fn test_mouse_cleanup_error_has_priority_but_restore_is_attempted() {
    let (ops, calls) = ops(Failures {
        disable: true,
        restore: true,
        ..Default::default()
    });
    let session = TerminalSession::with_ops(ops).unwrap();
    assert_eq!(
        session.finish(Ok(())).unwrap_err().to_string(),
        "disable error"
    );
    assert_eq!(
        *calls.lock().unwrap(),
        ["init", "hook", "disable", "restore", "drop"]
    );
}

#[test]
fn test_restore_failure_is_reported_after_mouse_cleanup_and_drop() {
    let (ops, calls) = ops(Failures {
        restore: true,
        ..Default::default()
    });
    let session = TerminalSession::with_ops(ops).unwrap();
    assert_eq!(
        session.finish(Ok(())).unwrap_err().to_string(),
        "restore error"
    );
    assert_eq!(
        *calls.lock().unwrap(),
        ["init", "hook", "disable", "restore", "drop"]
    );
}

#[test]
fn test_mouse_enable_failure_still_restores_terminal() {
    let (ops, calls) = ops(Failures {
        enable: true,
        ..Default::default()
    });
    let mut session = TerminalSession::with_ops(ops).unwrap();
    let result = session.enable_mouse().map_err(Into::into);
    assert_eq!(
        session.finish(result).unwrap_err().to_string(),
        "enable error"
    );
    assert_eq!(
        *calls.lock().unwrap(),
        ["init", "hook", "enable", "disable", "restore", "drop"]
    );
}

#[test]
fn test_unwinding_restores_and_drops_active_session() {
    let (ops, calls) = ops(Failures::default());
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut session = TerminalSession::with_ops(ops).unwrap();
        session.enable_mouse().unwrap();
        panic!("controlled UI panic");
    }));
    assert!(unwind.is_err());
    assert_eq!(
        *calls.lock().unwrap(),
        ["init", "hook", "enable", "disable", "restore", "drop"]
    );
}

#[cfg(unix)]
#[path = "pty_test_support.rs"]
mod pty;

#[cfg(unix)]
#[test]
fn test_panic_fixture() {
    if std::env::var_os("RATAGIT_TERMINAL_PANIC_TEST").is_none() {
        return;
    }
    color_eyre::install().unwrap();
    let mut session = TerminalSession::new().unwrap();
    session.enable_mouse().unwrap();
    panic!("RATAGIT_CONTROLLED_TERMINAL_PANIC");
}

#[cfg(unix)]
#[test]
fn test_real_panic_restores_terminal_before_diagnostics() {
    let fixture = crate::process_tests::support::ProcessFixture::new();
    let mut terminal = pty::Pty::new();
    let original = terminal.mode();
    let mut child = terminal.spawn(
        fixture
            .command(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "terminal::tests::test_panic_fixture",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("RATAGIT_TERMINAL_PANIC_TEST", "1"),
    );
    let mut output = Vec::new();
    terminal.wait_for(&mut child, &mut output, b"\x1b[?1006h");
    assert!(!terminal.wait(&mut child, &mut output).success());
    assert_eq!(terminal.mode(), original);
    let panic = output
        .windows(b"RATAGIT_CONTROLLED_TERMINAL_PANIC".len())
        .position(|window| window == b"RATAGIT_CONTROLLED_TERMINAL_PANIC")
        .expect("panic diagnostics");
    let mouse = output
        .windows(8)
        .position(|window| window == b"\x1b[?1006l")
        .expect("mouse cleanup");
    let restore = output
        .windows(8)
        .position(|window| window == b"\x1b[?1049l")
        .expect("terminal restoration");
    assert!(
        mouse < restore && restore < panic,
        "terminal cleanup must precede panic diagnostics"
    );
}
