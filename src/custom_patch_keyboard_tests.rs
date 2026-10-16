use super::patch_keyboard_tests::{Keys, content, git};
use super::*;
use ratagit_git::HybridGitBackend;
use ratagit_harness::Runtime;
use ratagit_ui::TerminalSize;
use std::{fs, path::Path};

#[test]
fn custom_patch_keyboard_accumulates_files_and_hunks_previews_copies_and_applies_atomically() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tmp/custom-patch-keyboard");
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
    for path in ["a.txt", "b.txt"] {
        fs::write(repo.join(path), content(&[])).unwrap();
    }
    fs::write(repo.join("c.txt"), "base sentinel\n").unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", "base"]);
    fs::write(repo.join("a.txt"), content(&[2, 28, 56])).unwrap();
    fs::write(repo.join("b.txt"), content(&[3, 30, 57])).unwrap();
    fs::write(repo.join("c.txt"), "unselected commit change\n").unwrap();
    git(repo, &["commit", "-am", "source"]);
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
        ),
        sequence: Vec::new(),
    };
    keys.runtime.dispatch_ui(UiAction::RefreshAll);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        keys.key(KeyCode::Char('3'));
        keys.key(KeyCode::Enter);
        keys.key(KeyCode::Char(' ')); // a: whole file
        assert!(keys.screen().contains("│M a.txt"));
        keys.key(KeyCode::Enter);
        keys.key(KeyCode::Char(' ')); // remove first a hunk
        keys.key(KeyCode::Char('j'));
        keys.key(KeyCode::Char(' ')); // remove second
        keys.key(KeyCode::Char('n')); // b
        keys.key(KeyCode::Char(' ')); // b first hunk
        keys.key(KeyCode::Char('j'));
        keys.key(KeyCode::Char('j'));
        keys.key(KeyCode::Char(' '));
        keys.key(KeyCode::Esc); // return to file list with accumulated selection
        assert_eq!(keys.runtime.state().ui.custom_patch.included.len(), 2);
        assert!(keys.screen().contains("╎M b.txt"));
        keys.key(KeyCode::Char(' ')); // promote b to whole
        keys.key(KeyCode::Char(' ')); // remove b
        assert_eq!(keys.runtime.state().ui.custom_patch.included.len(), 1);
        keys.key(KeyCode::Enter);
        keys.key(KeyCode::Char(' ')); // include b first hunk only
        keys.key(KeyCode::Esc);
        assert_eq!(git(repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(repo, &["ls-files", "--stage"]), index);
        assert!(git(repo, &["status", "--porcelain=v1"]).is_empty());
        keys.control('p');
        let effect = key_effect_for_key(
            keys.runtime.state(),
            KeyCode::Char('y'),
            KeyModifiers::NONE,
            8,
            18,
            10,
        );
        let KeyEffect::Host {
            edit: false,
            path: text,
        } = effect
        else {
            panic!("copy did not use clipboard host: {effect:?}");
        };
        assert!(text.contains("+CHANGED-56"));
        assert!(text.contains("+CHANGED-03"));
        assert!(!text.contains("+CHANGED-02"));
        assert!(!text.contains("+CHANGED-30"));
        assert!(!text.contains("diff --git a/c.txt"));
        keys.runtime.dispatch_ui(UiAction::HostFinished {
            message: "Copied custom patch".into(),
            refresh: false,
        });
        keys.key(KeyCode::Tab);
        assert!(keys.screen().contains("Custom Patch"));
        assert!(keys.screen().contains("2 files ·"));
        assert!(
            keys.runtime
                .state()
                .ui
                .custom_patch
                .preview_return_focus
                .is_some()
        );
        keys.key(KeyCode::Tab);
        assert_eq!(keys.runtime.state().ui.focus, PanelFocus::Commits);
        keys.control('p');
        keys.key(KeyCode::Char('r'));
        keys.key(KeyCode::Enter);
        assert_eq!(
            fs::read_to_string(repo.join("a.txt")).unwrap(),
            content(&[2, 28])
        );
        assert_eq!(
            fs::read_to_string(repo.join("b.txt")).unwrap(),
            content(&[30, 57])
        );
        assert_eq!(git(repo, &["ls-files", "--stage"]), index);
        assert_eq!(git(repo, &["rev-parse", "HEAD"]), head);
        keys.control('p');
        keys.key(KeyCode::Char('a'));
        keys.key(KeyCode::Enter);
        assert_eq!(
            fs::read_to_string(repo.join("a.txt")).unwrap(),
            content(&[2, 28, 56])
        );
        assert_eq!(
            fs::read_to_string(repo.join("b.txt")).unwrap(),
            content(&[3, 30, 57])
        );
        keys.control('p');
        keys.key(KeyCode::Char('d'));
        keys.key(KeyCode::Enter);
        assert_eq!(
            git(repo, &["show", "HEAD:a.txt"]),
            content(&[2, 28]).as_bytes()
        );
        assert_eq!(
            git(repo, &["show", "HEAD:b.txt"]),
            content(&[30, 57]).as_bytes()
        );
        assert_eq!(
            git(repo, &["show", "HEAD:c.txt"]),
            b"unselected commit change\n"
        );
        assert!(git(repo, &["status", "--porcelain=v1"]).is_empty());
        assert!(keys.runtime.state().ui.custom_patch.included.is_empty());
    }));
    keys.save(repo, "custom-patch-cross-file");
    if let Err(error) = result {
        let kept = temp.keep();
        eprintln!("retained failing custom patch repo: {}", kept.display());
        std::panic::resume_unwind(error);
    }
}
