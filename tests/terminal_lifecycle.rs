use std::process::Command;

#[cfg(unix)]
#[path = "../src/terminal/pty_test_support.rs"]
mod pty;

#[test]
fn test_startup_error_is_reported_before_terminal_initialization() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ratagit"))
        .current_dir(directory.path())
        .env("NO_COLOR", "1")
        .env("RUST_BACKTRACE", "0")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("could not find repository"), "{stderr}");
    // No alternate-screen or mouse-capture commands should precede this error,
    // including when stdout is redirected rather than attached to a terminal.
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
}

#[cfg(unix)]
#[test]
fn test_real_terminal_quit_restores_raw_mode_and_mouse_capture() {
    use std::io::Write;
    let directory = tempfile::tempdir().unwrap();
    git2::Repository::init(directory.path()).unwrap();
    let mut terminal = pty::Pty::new();
    let original = terminal.mode();
    let mut child = terminal.spawn(
        Command::new(env!("CARGO_BIN_EXE_ratagit"))
            .current_dir(directory.path())
            .env("NO_COLOR", "1"),
    );
    let mut output = Vec::new();
    terminal.wait_for(&mut child, &mut output, b"\x1b[?1006h");
    assert_eq!(
        terminal.mode().flags[3] & (libc::ICANON | libc::ECHO),
        0,
        "child must enter raw mode"
    );
    terminal.master.write_all(b"q").unwrap();
    let status = terminal.wait(&mut child, &mut output);
    assert!(status.success(), "{}", String::from_utf8_lossy(&output));
    assert_eq!(terminal.mode(), original);
    let mouse = output
        .windows(8)
        .position(|window| window == b"\x1b[?1006l")
        .expect("mouse capture must end");
    let restore = output
        .windows(8)
        .position(|window| window == b"\x1b[?1049l")
        .expect("alternate screen must end");
    assert!(
        mouse < restore,
        "mouse cleanup must precede terminal restoration"
    );
}
