//! Exercise the production keyboard mapper, renderer and backend together.
use super::*;
use ratagit_git::HybridGitBackend;
use ratagit_harness::Runtime;
use ratagit_ui::TerminalSize;
use std::{fs, path::Path, process::Command, time::Duration};

fn git(repo: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", repo.join("empty-config"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn content(changes: &[usize]) -> String {
    (1..=60)
        .map(|line| {
            if changes.contains(&line) {
                format!("CHANGED-{line:02}\n")
            } else {
                format!("line-{line:02}\n")
            }
        })
        .collect()
}

struct Keys {
    runtime: Runtime<HybridGitBackend>,
    sequence: Vec<KeyCode>,
}

impl Keys {
    fn key(&mut self, code: KeyCode) {
        self.sequence.push(code);
        match key_effect_for_key(self.runtime.state(), code, KeyModifiers::NONE, 8, 18, 10) {
            KeyEffect::Dispatch(action) => self.runtime.dispatch_ui(action),
            effect => panic!("unexpected key effect {code:?}: {effect:?}"),
        }
    }

    fn screen(&self) -> String {
        self.runtime.render_terminal_text()
    }

    fn save(&self, repo: &Path, case: &str) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tmp/pr8-details-keyboard")
            .join(case);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("screen.txt"), self.screen()).unwrap();
        fs::write(
            path.join("app_context.txt"),
            format!("{:#?}", self.runtime.state()),
        )
        .unwrap();
        fs::write(
            path.join("input_sequence.txt"),
            format!("{:?}", self.sequence),
        )
        .unwrap();
        let state = [
            git(repo, &["status", "--short"]),
            git(repo, &["diff", "--cached"]),
            git(repo, &["diff"]),
        ]
        .concat();
        fs::write(path.join("git_state.txt"), state).unwrap();
    }
}

fn scenario(selection: &str, operation: &str) {
    let staged_d = operation == "unstage-d";
    let case_name = format!("{selection}-{operation}");
    let operation = if staged_d { "unstage" } else { operation };
    let temp = tempfile::tempdir().unwrap();
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
    fs::write(repo.join("a.txt"), content(&[])).unwrap();
    fs::write(repo.join("b.txt"), "base\n").unwrap();
    git(repo, &["add", "a.txt", "b.txt"]);
    git(repo, &["commit", "-m", "base"]);
    fs::write(repo.join("a.txt"), content(&[5, 25, 45])).unwrap();
    if operation == "unstage" {
        git(repo, &["add", "a.txt"]);
    }
    // Every operation must preserve an unrelated staged file.
    fs::write(repo.join("b.txt"), "staged sentinel\n").unwrap();
    git(repo, &["add", "b.txt"]);
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
        assert_eq!(keys.runtime.state().ui.focus, PanelFocus::Files);
        keys.key(KeyCode::Enter);
        assert_eq!(keys.runtime.state().ui.focus, PanelFocus::Details);
        assert!(keys.screen().contains("Hunk 1/3"));
        assert_eq!(
            keys.runtime.state().ui.details.patch.staged,
            operation == "unstage"
        );
        let chosen = match selection {
            "single" => {
                keys.key(KeyCode::Char('j'));
                vec![25]
            }
            "multiple" => {
                for c in "xjjx".chars() {
                    keys.key(KeyCode::Char(c));
                }
                assert!(keys.screen().contains("2 selected"));
                vec![5, 45]
            }
            "visual" => {
                for c in "vj".chars() {
                    keys.key(KeyCode::Char(c));
                }
                assert!(keys.screen().contains("2 selected | Range"));
                assert_eq!(
                    keys.runtime.state().ui.details.patch.mode,
                    ratagit_core::partial::SelectionMode::Block
                );
                vec![5, 25]
            }
            _ => unreachable!(),
        };
        if operation == "discard" {
            keys.key(KeyCode::Char('d'));
            assert!(keys.screen().contains("Discard selected lines?"));
            assert_eq!(
                fs::read_to_string(repo.join("a.txt")).unwrap(),
                content(&[5, 25, 45])
            );
            keys.key(KeyCode::Enter);
        } else {
            keys.key(KeyCode::Char(if staged_d { 'd' } else { ' ' }));
        }
        let rest: Vec<_> = [5, 25, 45]
            .into_iter()
            .filter(|line| !chosen.contains(line))
            .collect();
        let (index, worktree) = match operation {
            "stage" => (content(&chosen), content(&[5, 25, 45])),
            "unstage" => (content(&rest), content(&[5, 25, 45])),
            "discard" => (content(&[]), content(&rest)),
            _ => unreachable!(),
        };
        assert_eq!(git(repo, &["show", ":a.txt"]), index.as_bytes());
        assert_eq!(fs::read(repo.join("a.txt")).unwrap(), worktree.as_bytes());
        assert_eq!(git(repo, &["show", ":b.txt"]), b"staged sentinel\n");
        assert!(keys.screen().contains("completed"), "{}", keys.screen());
        assert!(keys.runtime.state().ui.details.patch.marked.is_empty());
        assert!(keys.runtime.state().ui.details.patch.anchor.is_none());
        assert_eq!(keys.runtime.state().ui.focus, PanelFocus::Details);
        if operation != "discard" {
            // PR #8 follows the applied cursor to the destination side.
            assert_eq!(
                keys.runtime.state().ui.details.patch.staged,
                operation == "stage"
            );
            let ids = ratagit_core::partial::cursor_ids(keys.runtime.state());
            let patch = keys.runtime.state().repo.details.patch.as_ref().unwrap();
            assert!(ids.iter().any(|id| {
                let line = &patch.files[id.file].hunks[id.hunk].lines[id.line];
                line.new_line
                    .or(line.old_line)
                    .is_some_and(|line| chosen.contains(&(line as usize)))
            }));
        }
    }));
    keys.save(repo, &case_name);
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

macro_rules! case {
    ($name:ident, $selection:literal, $operation:literal) => {
        #[test]
        fn $name() {
            scenario($selection, $operation);
        }
    };
}
case!(test_keyboard_single_hunk_stage, "single", "stage");
case!(test_keyboard_single_hunk_unstage, "single", "unstage");
case!(test_keyboard_single_hunk_discard, "single", "discard");
case!(test_keyboard_multiple_hunks_stage, "multiple", "stage");
case!(test_keyboard_multiple_hunks_unstage, "multiple", "unstage");
case!(test_keyboard_multiple_hunks_discard, "multiple", "discard");
case!(test_keyboard_visual_hunks_stage, "visual", "stage");
case!(test_keyboard_visual_hunks_unstage, "visual", "unstage");
case!(test_keyboard_visual_hunks_discard, "visual", "discard");
case!(test_keyboard_staged_d_unstages, "single", "unstage-d");
