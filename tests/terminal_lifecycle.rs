mod support;
use support::ProcessFixture;

#[cfg(unix)]
#[path = "support/pty.rs"]
mod pty;

#[test]
fn test_invalid_git_directory_fails_before_terminal_setup() {
    let fixture = ProcessFixture::new();
    std::fs::write(fixture.work.join(".git"), "invalid gitfile\n").unwrap();
    let output = fixture.run(&mut fixture.command(env!("CARGO_BIN_EXE_ratagit")));
    let stderr = fixture.normalize(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("code ") && stderr.contains("class "),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
    assert!(!stderr.contains('\x1b'));
}

#[test]
fn test_redirected_terminal_failure_has_no_alternate_screen_output() {
    let fixture = ProcessFixture::new();
    let output = fixture.run(&mut fixture.command(env!("CARGO_BIN_EXE_ratagit")));
    assert!(!output.status.success());
    assert!(
        !output
            .stdout
            .windows(8)
            .any(|bytes| bytes == b"\x1b[?1049h")
    );
    assert!(!fixture.normalize(&output.stderr).is_empty());
}

#[cfg(unix)]
#[test]
fn test_quit_restores_terminal_mode() {
    use std::io::Write;
    let fixture = ProcessFixture::new();
    let mut terminal = pty::Pty::new();
    let original = terminal.mode();
    let mut child = terminal.spawn(&mut fixture.command(env!("CARGO_BIN_EXE_ratagit")));
    let mut output = Vec::new();
    terminal.wait_for(&mut child, &mut output, b"Files");
    terminal.master.write_all(b"q").unwrap();
    assert!(terminal.wait(&mut child, &mut output).success());
    assert_eq!(terminal.mode(), original);
    assert!(pty::contains(&output, b"\x1b[?1049l"));
}
#[test]
fn test_child_git_configuration_is_isolated_without_parent_environment_changes() {
    let parent_home = std::env::var_os("HOME");
    let parent_git_config = std::env::var_os("GIT_CONFIG_GLOBAL");
    let fixture = ProcessFixture::new();
    let output = fixture.run(
        fixture
            .command("git")
            .args(["config", "--global", "user.name"]),
    );
    assert!(output.status.success());
    assert_eq!(fixture.normalize(&output.stdout).trim(), "Process Fixture");
    assert_eq!(std::env::var_os("HOME"), parent_home);
    assert_eq!(std::env::var_os("GIT_CONFIG_GLOBAL"), parent_git_config);
}
