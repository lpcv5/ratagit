//! Real Git regression coverage through the production key mapper and renderer.
use super::patch_keyboard_tests::{Keys, git};
use super::*;
use ratagit_git::HybridGitBackend;
use ratagit_harness::Runtime;
use ratagit_ui::TerminalSize;
use std::{fs, path::Path, time::Duration};

fn searched_unicode_checkout(branch: bool) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tmp/git-tests/commit-files-keyboard");
    fs::create_dir_all(&root).unwrap();
    let temp = tempfile::tempdir_in(&root).unwrap();
    let repo = temp.path();
    git(repo, &["init", "-b", "main"]);
    for (key, value) in [
        ("user.name", "ratagit-tests"),
        ("user.email", "tests@example.com"),
        ("core.autocrlf", "false"),
        ("commit.gpgsign", "false"),
    ] {
        git(repo, &["config", key, value]);
    }
    let path = "界面.txt";
    for (name, text) in [(path, "base\n"), ("sentinel.txt", "base sentinel\n")] {
        fs::write(repo.join(name), text).unwrap();
    }
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", "base"]);
    fs::write(repo.join(path), "historical Unicode content\n").unwrap();
    git(repo, &["commit", "-am", "source"]);
    fs::write(repo.join(path), "current Unicode content\n").unwrap();
    git(repo, &["commit", "-am", "later"]);
    fs::write(repo.join("sentinel.txt"), "staged sentinel\n").unwrap();
    git(repo, &["add", "sentinel.txt"]);
    let head = git(repo, &["rev-parse", "HEAD"]);
    let index = git(repo, &["ls-files", "--stage"]);
    let mut keys = Keys {
        runtime: Runtime::new(
            AppContext::default(),
            HybridGitBackend::open(repo).unwrap(),
            TerminalSize {
                width: 140,
                height: 40,
            },
        )
        .with_debounce_window(Duration::ZERO),
        sequence: Vec::new(),
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        keys.runtime.dispatch_ui(UiAction::RefreshAll);
        keys.key(KeyCode::Char(if branch { '2' } else { '3' }));
        if branch {
            keys.key(KeyCode::Enter);
        }
        keys.key(KeyCode::Char('j'));
        keys.key(KeyCode::Enter);
        assert!(keys.screen().contains(".txt"), "{}", keys.screen());
        keys.key(KeyCode::Char('/'));
        for ch in path.chars() {
            keys.key(KeyCode::Char(ch));
        }
        assert_eq!(
            input_mode_for_state(keys.runtime.state()),
            InputMode::SearchInput
        );
        assert_eq!(
            key_effect_for_key(
                keys.runtime.state(),
                KeyCode::Char('e'),
                KeyModifiers::NONE,
                8,
                18,
                10
            ),
            KeyEffect::Dispatch(UiAction::InputSearchChar('e'))
        );
        keys.key(KeyCode::Enter);
        assert_eq!(
            input_mode_for_state(keys.runtime.state()),
            InputMode::SearchQuery
        );
        for (code, modifiers, edit) in [
            (KeyCode::Char('e'), KeyModifiers::NONE, true),
            (KeyCode::Char('o'), KeyModifiers::CONTROL, false),
        ] {
            assert_eq!(
                key_effect_for_key(keys.runtime.state(), code, modifiers, 8, 18, 10),
                KeyEffect::Host {
                    edit,
                    path: path.into()
                }
            );
        }
        assert!(keys.screen().contains("historical Unicode content"));
        keys.key(KeyCode::Esc);
        keys.key(KeyCode::Enter);
        assert_eq!(keys.runtime.state().ui.focus, PanelFocus::Details);
        assert_eq!(
            key_effect_for_key(
                keys.runtime.state(),
                KeyCode::Char('n'),
                KeyModifiers::NONE,
                8,
                18,
                3
            ),
            KeyEffect::Dispatch(UiAction::NextPatchFileInViewport {
                up: false,
                visible_lines: 3
            })
        );
        assert_eq!(
            key_effect_for_key(
                keys.runtime.state(),
                KeyCode::Char('N'),
                KeyModifiers::NONE,
                8,
                18,
                3
            ),
            KeyEffect::Dispatch(UiAction::NextPatchFileInViewport {
                up: true,
                visible_lines: 3
            })
        );
        keys.key(KeyCode::Char('r'));
        assert!(!keys.runtime.state().ui.details.patch.active);
        assert_eq!(
            keys.runtime.state().ui.focus,
            if branch {
                PanelFocus::Branches
            } else {
                PanelFocus::Commits
            }
        );
        assert!(keys.screen().contains("界 面 .txt"));
        keys.key(KeyCode::Char('c'));
        assert!(keys.screen().contains("index is preserved"));
        keys.key(KeyCode::Enter);
        assert_eq!(
            fs::read(repo.join(path)).unwrap(),
            b"historical Unicode content\n"
        );
        assert_eq!(git(repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(repo, &["ls-files", "--stage"]), index);
        assert_eq!(
            fs::read(repo.join("sentinel.txt")).unwrap(),
            b"staged sentinel\n"
        );
        assert!(
            keys.runtime
                .state()
                .work
                .mutation
                .operation_pending
                .is_none()
        );
        assert!(keys.runtime.state().ui.workflow.dialog.is_none());
        assert!(keys.screen().contains("completed"), "{}", keys.screen());
    }));
    keys.save(
        repo,
        if branch {
            "branch-searched-unicode-checkout"
        } else {
            "main-searched-unicode-checkout"
        },
    );
    if let Err(error) = result {
        eprintln!("retained failing Git fixture: {}", temp.keep().display());
        std::panic::resume_unwind(error);
    }
}

#[test]
fn commit_files_keyboard_searched_unicode_checkout_preserves_index() {
    searched_unicode_checkout(false);
}

#[test]
fn branch_commit_files_keyboard_searched_unicode_checkout_preserves_index() {
    searched_unicode_checkout(true);
}
