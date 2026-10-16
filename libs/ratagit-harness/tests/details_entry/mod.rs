use super::*;
use ratagit_core::Command;
use ratagit_git::execute_command;

pub(super) fn verify() {
    let size = TerminalSize {
        width: 120,
        height: 35,
    };
    let mut before = Runtime::new(
        AppContext::default(),
        MockGitBackend::new(fixture_many_files()),
        size,
    );
    before.dispatch_ui(UiAction::RefreshAll);
    let mut frames = vec![(
        "before",
        before.state().clone(),
        before.render_terminal_text(),
    )];
    // Hold the reviewed-patch command deterministically, without a worker race.
    let mut pending = Runtime::new(before.state().clone(), before.backend().clone(), size)
        .with_debounce_window(Duration::from_secs(60));
    pending.dispatch_ui(UiAction::OpenFilePatch);
    frames.push((
        "pending",
        pending.state().clone(),
        pending.render_terminal_text(),
    ));
    let mut backend = pending.backend().clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let position = |screen: &str, needle: &str| {
            screen
                .lines()
                .enumerate()
                .find_map(|(y, line)| line.find(needle).map(|x| (line[..x].chars().count(), y)))
                .unwrap_or_else(|| panic!("missing {needle}: {screen}"))
        };
        let before_screen = &frames[0].2;
        let pending_screen = &frames[1].2;
        assert_eq!(
            position(before_screen, "Branches"),
            position(pending_screen, "Branches")
        );
        assert_eq!(
            position(before_screen, "-old staged file-00.txt"),
            position(pending_screen, "-old staged file-00.txt")
        );
        assert!(!pending_screen.contains("Loading reviewed patch"));
        assert_eq!(pending.backend().snapshot(), before.backend().snapshot());
        assert_eq!(
            pending.backend().operations(),
            before.backend().operations()
        );
        assert!(pending.state().repo.details.patch.is_none());
        let mut state = pending.state().clone();
        assert!(update(&mut state, Action::Ui(UiAction::ApplyPatchSelection)).is_empty());
        let (request_id, request) = state.work.details.patch_request.clone().unwrap();
        let result = execute_command(
            &mut backend,
            Command::LoadPatch {
                request_id,
                request,
            },
        );
        assert!(update(&mut state, Action::GitResult(result)).is_empty());
        let ready = Runtime::new(state, backend.clone(), size);
        frames.push(("ready", ready.state().clone(), ready.render_terminal_text()));
        assert!(frames[2].2.contains("+new mock content"));
        assert_eq!(
            position(&frames[0].2, "Branches"),
            position(&frames[2].2, "Branches")
        );
        assert_eq!(backend.snapshot(), before.backend().snapshot());
    }));
    if let Err(payload) = result {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/harness-artifacts/details-entry");
        let _ = std::fs::create_dir_all(&root);
        for (name, state, screen) in frames {
            let _ = std::fs::write(root.join(format!("{name}-screen.txt")), screen);
            let _ = std::fs::write(
                root.join(format!("{name}-state.txt")),
                format!("{state:#?}"),
            );
        }
        let _ = std::fs::write(
            root.join("git-state.txt"),
            format!("{:#?}", backend.snapshot()),
        );
        let _ = std::fs::write(root.join("git-ops.txt"), backend.operations().join("\n"));
        let _ = std::fs::write(
            root.join("inputs.txt"),
            "RefreshAll -> OpenFilePatch -> ApplyPatchSelection (blocked) -> matching LoadedPatch",
        );
        eprintln!("Details entry artifacts: {}", root.display());
        std::panic::resume_unwind(payload);
    }
}
