mod support;
use support::ProcessFixture;

#[cfg(unix)]
#[path = "../src/terminal/pty_test_support.rs"]
mod pty;

#[test]
fn test_startup_error_is_reported_before_terminal_initialization() {
    let fixture = ProcessFixture::new();
    let output = fixture.run(&mut fixture.command(env!("CARGO_BIN_EXE_ratagit")));

    assert!(!output.status.success());
    let stderr = fixture.normalize(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("could not find repository"), "{stderr}");
    // No alternate-screen or mouse-capture commands should precede this error,
    // including when stdout is redirected rather than attached to a terminal.
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
}

#[test]
fn test_invalid_git_directory_reports_diagnostic_without_terminal_output() {
    let fixture = ProcessFixture::new();
    std::fs::write(fixture.work.join(".git"), "invalid gitfile\n").unwrap();
    let output = fixture.run(&mut fixture.command(env!("CARGO_BIN_EXE_ratagit")));
    let stderr = fixture.normalize(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Git error:"), "{stderr}");
    assert!(
        stderr.contains("code ") && stderr.contains("class "),
        "{stderr}"
    );
    assert!(!stderr.contains('\x1b'), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn test_unwritable_profile_destination_fails_before_terminal_initialization() {
    let fixture = ProcessFixture::new();
    std::fs::create_dir(fixture.work.join("ratagit-latency.jsonl")).unwrap();
    let output = fixture.run(
        fixture
            .command(env!("CARGO_BIN_EXE_ratagit"))
            .arg("--profile-latency"),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(!output.stderr.is_empty());
    let stderr = fixture.normalize(&output.stderr);
    assert!(!stderr.contains('\x1b'));
    #[cfg(windows)]
    assert!(stderr.contains("os error 5"), "{stderr}");
    #[cfg(unix)]
    assert!(stderr.contains("os error 21"), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(!fixture.path("config/ratagit/keymap.toml").exists());
    assert!(fixture.work.join("ratagit-latency.jsonl").is_dir());
}

#[cfg(unix)]
#[test]
fn test_profile_latency_records_real_terminal_session() {
    use std::io::Write;
    let fixture = ProcessFixture::new();
    git2::Repository::init(&fixture.work).unwrap();
    let mut terminal = pty::Pty::new();
    let original = terminal.mode();
    let mut child = terminal.spawn(
        fixture
            .command(env!("CARGO_BIN_EXE_ratagit"))
            .arg("--profile-latency"),
    );
    let mut output = Vec::new();
    terminal.wait_for(&mut child, &mut output, b"Files");
    terminal.master.write_all(b"q").unwrap();
    assert!(terminal.wait(&mut child, &mut output).success());
    assert_eq!(terminal.mode(), original);
    let content = std::fs::read_to_string(fixture.work.join("ratagit-latency.jsonl")).unwrap();
    let events: Vec<serde_json::Value> = content
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(events
        .iter()
        .any(|event| event["stage"] == "request_started"));
    assert!(events
        .iter()
        .any(|event| event["stage"] == "frame_unobserved"));
    assert!(fixture.work.join("ratagit-debug.log").exists());
    assert!(fixture.path("config/ratagit/keymap.toml").exists());
    assert!(!output
        .windows(b"request_id".len())
        .any(|window| window == b"request_id"));
}

#[cfg(unix)]
#[test]
fn test_real_terminal_quit_restores_raw_mode_and_mouse_capture() {
    use std::io::Write;
    let fixture = ProcessFixture::new();
    git2::Repository::init(&fixture.work).unwrap();
    let mut terminal = pty::Pty::new();
    let original = terminal.mode();
    let mut child = terminal.spawn(&mut fixture.command(env!("CARGO_BIN_EXE_ratagit")));
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
