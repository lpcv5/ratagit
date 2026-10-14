use std::process::Command;

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
