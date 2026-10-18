use crate::input::{KeyEffect, key_effect_for_key_in_viewport};
use crossterm::event::{KeyCode, KeyModifiers};
use ratagit_core::{AppContext, UiAction};
use ratagit_git::MockGitBackend;
use ratagit_harness::Runtime;
use ratagit_testkit::fixture_dirty_repo;
use ratagit_ui::{TerminalSize, render_terminal_text};

fn effect(state: &AppContext, code: KeyCode, size: TerminalSize) -> KeyEffect {
    key_effect_for_key_in_viewport(state, code, KeyModifiers::NONE, size, 5, 10, 3)
}

fn runtime() -> Runtime<MockGitBackend> {
    let mut runtime = Runtime::new(
        AppContext::default(),
        MockGitBackend::new(fixture_dirty_repo()),
        TerminalSize {
            width: 80,
            height: 24,
        },
    );
    runtime.dispatch_ui(UiAction::RefreshAll);
    runtime
}

#[test]
fn modal_viewport_hidden_confirmation_cannot_discard_and_escape_cancels() {
    let mut runtime = runtime();
    runtime.dispatch_ui(UiAction::OpenDiscardConfirm);
    assert!(runtime.state().ui.discard_confirm.active);
    let before = runtime.backend().snapshot().clone();
    let operations = runtime.backend().operations().to_vec();

    for size in [
        TerminalSize {
            width: 80,
            height: 8,
        },
        TerminalSize {
            width: 80,
            height: 12,
        },
        TerminalSize {
            width: 35,
            height: 24,
        },
    ] {
        let screen = render_terminal_text(runtime.state(), size);
        assert!(screen.contains("40"), "{screen}");
        assert!(screen.contains("Esc cancel"), "{screen}");
        for key in [KeyCode::Enter, KeyCode::Char('d'), KeyCode::Char('j')] {
            assert_eq!(effect(runtime.state(), key, size), KeyEffect::Ignore);
        }
    }
    assert_eq!(runtime.backend().snapshot(), &before);
    assert_eq!(runtime.backend().operations(), operations);

    let small = TerminalSize {
        width: 80,
        height: 12,
    };
    let KeyEffect::Dispatch(cancel) = effect(runtime.state(), KeyCode::Esc, small) else {
        panic!("Escape must cancel a hidden confirmation");
    };
    runtime.dispatch_ui(cancel);
    assert!(!runtime.state().ui.discard_confirm.active);
    assert_eq!(runtime.backend().snapshot(), &before);
    assert_eq!(runtime.backend().operations(), operations);
}

#[test]
fn modal_viewport_palette_resumes_after_resize_without_hidden_execution() {
    let mut runtime = runtime();
    runtime.dispatch_ui(UiAction::OpenCommandPalette);
    let before = runtime.backend().snapshot().clone();
    let operations = runtime.backend().operations().to_vec();
    let small = TerminalSize {
        width: 80,
        height: 12,
    };
    assert!(render_terminal_text(runtime.state(), small).contains("Window too small"));
    assert_eq!(
        effect(runtime.state(), KeyCode::Enter, small),
        KeyEffect::Ignore
    );
    assert_eq!(
        effect(runtime.state(), KeyCode::Char('j'), small),
        KeyEffect::Ignore
    );
    assert_eq!(runtime.state().ui.command_palette.selected, 0);

    let restored = TerminalSize {
        width: 80,
        height: 24,
    };
    let screen = render_terminal_text(runtime.state(), restored);
    assert!(screen.contains("Commands"), "{screen}");
    assert!(!screen.contains("Window too small"));
    assert!(matches!(
        effect(runtime.state(), KeyCode::Enter, restored),
        KeyEffect::Dispatch(_)
    ));

    runtime.dispatch_ui(UiAction::StartPaletteSearch);
    assert!(runtime.state().ui.command_palette.search_active);
    let KeyEffect::Dispatch(cancel) = effect(runtime.state(), KeyCode::Esc, small) else {
        panic!("Escape must close palette even while filtering");
    };
    runtime.dispatch_ui(cancel);
    assert!(!runtime.state().ui.command_palette.active);
    assert_eq!(runtime.backend().snapshot(), &before);
    assert_eq!(runtime.backend().operations(), operations);
}

#[test]
fn modal_viewport_quit_is_available_and_normal_navigation_is_unrestricted() {
    let mut state = AppContext::default();
    let small = TerminalSize {
        width: 35,
        height: 8,
    };
    assert!(matches!(
        effect(&state, KeyCode::Char('j'), small),
        KeyEffect::Dispatch(_)
    ));
    state.ui.command_palette.active = true;
    assert_eq!(
        key_effect_for_key_in_viewport(
            &state,
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            small,
            5,
            10,
            3,
        ),
        KeyEffect::Quit,
    );
}
