use ratagit_core::{Action, AppState, GitResult, update};
use ratagit_testkit::{
    fixture_conflict, fixture_dirty_repo, fixture_empty_repo, fixture_unicode_paths,
};
use ratagit_ui::{TerminalSize, render};

fn render_snapshot(snapshot: ratagit_core::RepoSnapshot, size: TerminalSize) -> String {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(snapshot)),
    );
    assert!(commands.is_empty());
    render(&state, size).as_text()
}

#[test]
fn snapshots_empty_repo_80x24() {
    let text = render_snapshot(
        fixture_empty_repo(),
        TerminalSize {
            width: 80,
            height: 24,
        },
    );
    assert!(text.contains("ratagit MVP"));
    assert!(text.contains("summary=staged: 0, unstaged: 0"));
    assert!(text.contains("[Status]"));
}

#[test]
fn snapshots_dirty_repo_100x30() {
    let text = render_snapshot(
        fixture_dirty_repo(),
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(text.contains("[Files]"));
    assert!(text.contains("[Commits]"));
    assert!(text.contains("[Branches]"));
    assert!(text.contains("[Stash]"));
}

#[test]
fn snapshots_conflict_120x40() {
    let text = render_snapshot(
        fixture_conflict(),
        TerminalSize {
            width: 120,
            height: 40,
        },
    );
    assert!(text.contains("both modified"));
    assert!(text.contains("conflict"));
}

#[test]
fn snapshots_unicode_paths_are_stable() {
    let text = render_snapshot(
        fixture_unicode_paths(),
        TerminalSize {
            width: 80,
            height: 24,
        },
    );
    assert!(text.contains("你好"));
    assert!(text.contains("emoji-"));
}
