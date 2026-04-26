use std::error::Error;
use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratagit_core::{AppState, BranchDeleteMode, FileInputMode, PanelFocus, UiAction};
use ratagit_git::{GitBackend, HybridGitBackend, MockGitBackend, is_git_repo};
use ratagit_harness::AsyncRuntime;
use ratagit_observe::{ObserveConfig, init_observability};
use ratagit_testkit::fixture_dirty_repo;
use ratagit_ui::{
    TerminalSize, details_content_lines_for_terminal_size, details_scroll_lines_for_terminal_size,
    render_terminal,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() -> Result<(), Box<dyn Error>> {
    let _ = init_observability(&ObserveConfig::default());
    run_tui()
}

fn run_tui() -> Result<(), Box<dyn Error>> {
    let backend = select_backend()?;
    let mut terminal = setup_terminal()?;
    let mut runtime = AsyncRuntime::new(
        AppState::default(),
        backend,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .with_debounce_window(Duration::from_millis(80));
    runtime.dispatch_ui(UiAction::RefreshAll);

    loop {
        runtime.tick();
        terminal.draw(|frame| {
            render_terminal(frame, runtime.state());
        })?;

        if !event::poll(Duration::from_millis(16))? {
            runtime.tick();
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let terminal_size = terminal.size()?;
        let details_scroll_lines = details_scroll_lines_for_terminal_size(TerminalSize {
            width: terminal_size.width as usize,
            height: terminal_size.height as usize,
        });
        let details_visible_lines = details_content_lines_for_terminal_size(TerminalSize {
            width: terminal_size.width as usize,
            height: terminal_size.height as usize,
        });

        match key_effect_for_key(
            runtime.state(),
            key.code,
            key.modifiers,
            details_scroll_lines,
            details_visible_lines,
        ) {
            KeyEffect::Quit => break,
            KeyEffect::Dispatch(action) => {
                runtime.dispatch_ui(action);
                runtime.tick();
            }
            KeyEffect::Ignore => {}
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyEffect {
    Quit,
    Dispatch(UiAction),
    Ignore,
}

fn key_effect_for_key(
    state: &AppState,
    code: KeyCode,
    modifiers: KeyModifiers,
    details_scroll_lines: usize,
    details_visible_lines: usize,
) -> KeyEffect {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return KeyEffect::Quit;
    }

    if let Some(action) = ui_action_for_key(
        state,
        code,
        modifiers,
        details_scroll_lines,
        details_visible_lines,
    ) {
        return KeyEffect::Dispatch(action);
    }

    if code == KeyCode::Char('q') {
        return KeyEffect::Quit;
    }

    KeyEffect::Ignore
}

fn ui_action_for_key(
    state: &AppState,
    code: KeyCode,
    modifiers: KeyModifiers,
    details_scroll_lines: usize,
    details_visible_lines: usize,
) -> Option<UiAction> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('u') => {
                return Some(UiAction::DetailsScrollUp {
                    lines: details_scroll_lines,
                });
            }
            KeyCode::Char('d') => {
                return Some(UiAction::DetailsScrollDown {
                    lines: details_scroll_lines,
                    visible_lines: details_visible_lines,
                });
            }
            _ => {}
        }
    }

    if state.editor.is_active() {
        return match code {
            KeyCode::Enter => Some(UiAction::EditorConfirm),
            KeyCode::Esc => Some(UiAction::EditorCancel),
            KeyCode::Backspace => Some(UiAction::EditorBackspace),
            KeyCode::Left => Some(UiAction::EditorMoveCursorLeft),
            KeyCode::Right => Some(UiAction::EditorMoveCursorRight),
            KeyCode::Home => Some(UiAction::EditorMoveCursorHome),
            KeyCode::End => Some(UiAction::EditorMoveCursorEnd),
            KeyCode::Tab => Some(UiAction::EditorNextField),
            KeyCode::BackTab => Some(UiAction::EditorPrevField),
            KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                Some(UiAction::EditorInsertNewline)
            }
            KeyCode::Char(ch)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                Some(UiAction::EditorInputChar(ch))
            }
            _ => None,
        };
    }

    if state.branches.create.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchCreate),
            KeyCode::Esc => Some(UiAction::CancelBranchCreate),
            KeyCode::Backspace => Some(UiAction::BranchCreateBackspace),
            KeyCode::Left => Some(UiAction::BranchCreateMoveCursorLeft),
            KeyCode::Right => Some(UiAction::BranchCreateMoveCursorRight),
            KeyCode::Home => Some(UiAction::BranchCreateMoveCursorHome),
            KeyCode::End => Some(UiAction::BranchCreateMoveCursorEnd),
            KeyCode::Char(ch)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                Some(UiAction::BranchCreateInputChar(ch))
            }
            _ => None,
        };
    }

    if state.branches.delete_menu.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchDeleteMenu),
            KeyCode::Esc => Some(UiAction::CancelBranchDeleteMenu),
            KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveBranchDeleteMenuUp),
            KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveBranchDeleteMenuDown),
            _ => None,
        };
    }

    if state.branches.force_delete_confirm.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchForceDelete),
            KeyCode::Esc => Some(UiAction::CancelBranchForceDelete),
            _ => None,
        };
    }

    if state.branches.rebase_menu.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchRebaseMenu),
            KeyCode::Esc => Some(UiAction::CancelBranchRebaseMenu),
            KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveBranchRebaseMenuUp),
            KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveBranchRebaseMenuDown),
            _ => None,
        };
    }

    if state.branches.auto_stash_confirm.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmAutoStash),
            KeyCode::Esc => Some(UiAction::CancelAutoStash),
            _ => None,
        };
    }

    if state.reset_menu.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmResetMenu),
            KeyCode::Esc => Some(UiAction::CancelResetMenu),
            KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveResetMenuUp),
            KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveResetMenuDown),
            _ => None,
        };
    }

    if state.discard_confirm.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmDiscard),
            KeyCode::Esc => Some(UiAction::CancelDiscard),
            _ => None,
        };
    }

    if state.focus == PanelFocus::Files && state.files.mode == FileInputMode::SearchInput {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmFileSearch),
            KeyCode::Esc => Some(UiAction::CancelFileSearch),
            KeyCode::Backspace => Some(UiAction::BackspaceFileSearch),
            KeyCode::Char(ch) => Some(UiAction::InputFileSearchChar(ch)),
            _ => None,
        };
    }

    if state.focus == PanelFocus::Files {
        match code {
            // TODO(files-hunks): map Enter to hunk-level editing/partial stage workflow.
            KeyCode::Enter => return Some(UiAction::ToggleSelectedDirectory),
            KeyCode::Char(' ') => return Some(UiAction::ToggleSelectedFileStage),
            KeyCode::Char('v') => return Some(UiAction::ToggleFilesMultiSelect),
            KeyCode::Char('/') => return Some(UiAction::StartFileSearch),
            KeyCode::Char('n') => return Some(UiAction::NextFileSearchMatch),
            KeyCode::Char('N') => return Some(UiAction::PrevFileSearchMatch),
            KeyCode::Esc => return Some(UiAction::CancelFileSearch),
            KeyCode::Char('c') => return Some(UiAction::OpenCommitEditor),
            KeyCode::Char('s') => return Some(UiAction::OpenStashEditor),
            KeyCode::Char('d') => return Some(UiAction::OpenDiscardConfirm),
            KeyCode::Char('D') => return Some(UiAction::OpenResetMenu),
            _ => {}
        }
    }

    if state.focus == PanelFocus::Branches {
        match code {
            KeyCode::Char(' ') => return Some(UiAction::CheckoutSelectedBranch),
            KeyCode::Char('n') => return Some(UiAction::OpenBranchCreateInput),
            KeyCode::Char('d') => return Some(UiAction::OpenBranchDeleteMenu),
            KeyCode::Char('r') => return Some(UiAction::OpenBranchRebaseMenu),
            _ => {}
        }
    }

    if state.focus == PanelFocus::Commits {
        match code {
            KeyCode::Char(' ') => return Some(UiAction::CheckoutSelectedCommitDetached),
            KeyCode::Char('c') => return Some(UiAction::OpenCommitEditor),
            KeyCode::Char('v') => return Some(UiAction::ToggleCommitsMultiSelect),
            KeyCode::Char('s') => return Some(UiAction::SquashSelectedCommits),
            KeyCode::Char('f') => return Some(UiAction::FixupSelectedCommits),
            KeyCode::Char('r') => return Some(UiAction::OpenCommitRewordEditor),
            KeyCode::Char('d') => return Some(UiAction::DeleteSelectedCommits),
            _ => {}
        }
    }

    match code {
        KeyCode::Char('r') => Some(UiAction::RefreshAll),
        KeyCode::Char('l') => Some(UiAction::FocusNext),
        KeyCode::Char('h') => Some(UiAction::FocusPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveDown),
        KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveUp),
        KeyCode::Char('1') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Files,
        }),
        KeyCode::Char('2') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
        KeyCode::Char('3') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
        KeyCode::Char('4') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Stash,
        }),
        KeyCode::Char('5') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Details,
        }),
        KeyCode::Char('6') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Log,
        }),
        KeyCode::Char('c') => Some(UiAction::CreateCommit {
            message: "mvp commit".to_string(),
        }),
        KeyCode::Char('p') => Some(UiAction::StashPush {
            message: "savepoint".to_string(),
        }),
        KeyCode::Char('O') => Some(UiAction::StashPopSelected),
        KeyCode::Tab | KeyCode::BackTab => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DETAILS_SCROLL_LINES: usize = 7;
    const TEST_DETAILS_VISIBLE_LINES: usize = 18;

    fn map_key(state: &AppState, code: KeyCode) -> Option<UiAction> {
        ui_action_for_key(
            state,
            code,
            KeyModifiers::NONE,
            TEST_DETAILS_SCROLL_LINES,
            TEST_DETAILS_VISIBLE_LINES,
        )
    }

    fn active_commit_editor_state() -> AppState {
        let mut state = AppState::default();
        state.editor.kind = Some(ratagit_core::EditorKind::Commit {
            message: String::new(),
            message_cursor: 0,
            body: String::new(),
            body_cursor: 0,
            active_field: ratagit_core::CommitField::Body,
            intent: ratagit_core::CommitEditorIntent::Create,
        });
        state
    }

    fn active_reset_menu_state() -> AppState {
        let mut state = AppState::default();
        state.reset_menu.active = true;
        state
    }

    fn active_discard_confirm_state() -> AppState {
        let mut state = AppState::default();
        state.discard_confirm.active = true;
        state.discard_confirm.paths = vec!["a.txt".to_string()];
        state
    }

    fn active_branch_create_state() -> AppState {
        let mut state = AppState::default();
        state.branches.create.active = true;
        state
    }

    #[test]
    fn panel_navigation_uses_h_and_l_not_tab() {
        let state = AppState::default();
        assert_eq!(
            map_key(&state, KeyCode::Char('l')),
            Some(UiAction::FocusNext)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('h')),
            Some(UiAction::FocusPrev)
        );
        assert_eq!(map_key(&state, KeyCode::Tab), None);
        assert_eq!(map_key(&state, KeyCode::BackTab), None);
    }

    #[test]
    fn files_search_input_maps_text_until_confirm_or_escape() {
        let mut state = AppState::default();
        state.files.mode = FileInputMode::SearchInput;
        assert_eq!(
            map_key(&state, KeyCode::Char('r')),
            Some(UiAction::InputFileSearchChar('r'))
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('D')),
            Some(UiAction::InputFileSearchChar('D'))
        );
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmFileSearch)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelFileSearch)
        );
    }

    #[test]
    fn files_panel_git_keys_map_to_file_actions() {
        let state = AppState::default();
        assert_eq!(
            map_key(&state, KeyCode::Char(' ')),
            Some(UiAction::ToggleSelectedFileStage)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('c')),
            Some(UiAction::OpenCommitEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('s')),
            Some(UiAction::OpenStashEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('d')),
            Some(UiAction::OpenDiscardConfirm)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('D')),
            Some(UiAction::OpenResetMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('/')),
            Some(UiAction::StartFileSearch)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('v')),
            Some(UiAction::ToggleFilesMultiSelect)
        );
    }

    #[test]
    fn focused_panel_git_keys_map_to_panel_actions() {
        let mut state = AppState {
            focus: PanelFocus::Branches,
            last_left_focus: PanelFocus::Branches,
            ..AppState::default()
        };
        assert_eq!(
            map_key(&state, KeyCode::Char('n')),
            Some(UiAction::OpenBranchCreateInput)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char(' ')),
            Some(UiAction::CheckoutSelectedBranch)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('d')),
            Some(UiAction::OpenBranchDeleteMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('r')),
            Some(UiAction::OpenBranchRebaseMenu)
        );

        state.focus = PanelFocus::Commits;
        assert_eq!(
            map_key(&state, KeyCode::Char('s')),
            Some(UiAction::SquashSelectedCommits)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('f')),
            Some(UiAction::FixupSelectedCommits)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('r')),
            Some(UiAction::OpenCommitRewordEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('d')),
            Some(UiAction::DeleteSelectedCommits)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char(' ')),
            Some(UiAction::CheckoutSelectedCommitDetached)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('v')),
            Some(UiAction::ToggleCommitsMultiSelect)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('c')),
            Some(UiAction::OpenCommitEditor)
        );

        state.focus = PanelFocus::Stash;
        assert_eq!(
            map_key(&state, KeyCode::Char('p')),
            Some(UiAction::StashPush {
                message: "savepoint".to_string()
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('O')),
            Some(UiAction::StashPopSelected)
        );
    }

    #[test]
    fn editor_mode_maps_keys_before_any_other_mode() {
        let mut state = active_commit_editor_state();
        state.files.mode = FileInputMode::SearchInput;

        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::EditorConfirm)
        );
        assert_eq!(
            map_key(&state, KeyCode::Tab),
            Some(UiAction::EditorNextField)
        );
        assert_eq!(
            map_key(&state, KeyCode::Left),
            Some(UiAction::EditorMoveCursorLeft)
        );
        assert_eq!(
            map_key(&state, KeyCode::Right),
            Some(UiAction::EditorMoveCursorRight)
        );
        assert_eq!(
            map_key(&state, KeyCode::Home),
            Some(UiAction::EditorMoveCursorHome)
        );
        assert_eq!(
            map_key(&state, KeyCode::End),
            Some(UiAction::EditorMoveCursorEnd)
        );
        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('j'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::EditorInsertNewline)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('m')),
            Some(UiAction::EditorInputChar('m'))
        );
    }

    #[test]
    fn editor_mode_q_dispatches_instead_of_quitting() {
        let state = active_commit_editor_state();

        assert_eq!(
            key_effect_for_key(
                &state,
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            KeyEffect::Dispatch(UiAction::EditorInputChar('q'))
        );
    }

    #[test]
    fn ctrl_u_and_ctrl_d_map_to_global_details_scroll() {
        let state = AppState::default();

        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::DetailsScrollUp {
                lines: TEST_DETAILS_SCROLL_LINES
            })
        );
        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::DetailsScrollDown {
                lines: TEST_DETAILS_SCROLL_LINES,
                visible_lines: TEST_DETAILS_VISIBLE_LINES
            })
        );
    }

    #[test]
    fn global_details_scroll_keys_work_while_editor_is_active() {
        let state = active_commit_editor_state();

        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::DetailsScrollDown {
                lines: TEST_DETAILS_SCROLL_LINES,
                visible_lines: TEST_DETAILS_VISIBLE_LINES
            })
        );
    }

    #[test]
    fn reset_menu_maps_navigation_confirm_and_cancel_before_panels() {
        let state = active_reset_menu_state();

        assert_eq!(
            map_key(&state, KeyCode::Down),
            Some(UiAction::MoveResetMenuDown)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('j')),
            Some(UiAction::MoveResetMenuDown)
        );
        assert_eq!(
            map_key(&state, KeyCode::Up),
            Some(UiAction::MoveResetMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('k')),
            Some(UiAction::MoveResetMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmResetMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelResetMenu)
        );
    }

    #[test]
    fn branch_create_input_maps_text_until_confirm_or_cancel() {
        let state = active_branch_create_state();

        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmBranchCreate)
        );
        assert_eq!(
            map_key(&state, KeyCode::Backspace),
            Some(UiAction::BranchCreateBackspace)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('x')),
            Some(UiAction::BranchCreateInputChar('x'))
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelBranchCreate)
        );
    }

    #[test]
    fn discard_confirm_maps_confirm_and_cancel_before_panels() {
        let state = active_discard_confirm_state();

        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmDiscard)
        );
        assert_eq!(map_key(&state, KeyCode::Esc), Some(UiAction::CancelDiscard));
        assert_eq!(map_key(&state, KeyCode::Char('d')), None);
    }

    #[test]
    fn ctrl_c_quits_even_when_editor_is_active() {
        let state = active_commit_editor_state();

        assert_eq!(
            key_effect_for_key(
                &state,
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            KeyEffect::Quit
        );
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()
}

#[derive(Debug)]
enum AppBackend {
    Hybrid(HybridGitBackend),
    Mock(MockGitBackend),
}

impl GitBackend for AppBackend {
    fn refresh_snapshot(&mut self) -> Result<ratagit_core::RepoSnapshot, ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.refresh_snapshot(),
            Self::Mock(inner) => inner.refresh_snapshot(),
        }
    }

    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ratagit_core::CommitEntry>, ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.load_more_commits(offset, limit),
            Self::Mock(inner) => inner.load_more_commits(offset, limit),
        }
    }

    fn files_details_diff(&mut self, paths: &[String]) -> Result<String, ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.files_details_diff(paths),
            Self::Mock(inner) => inner.files_details_diff(paths),
        }
    }

    fn branch_details_log(
        &mut self,
        branch: &str,
        max_count: usize,
    ) -> Result<String, ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.branch_details_log(branch, max_count),
            Self::Mock(inner) => inner.branch_details_log(branch, max_count),
        }
    }

    fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.commit_details_diff(commit_id),
            Self::Mock(inner) => inner.commit_details_diff(commit_id),
        }
    }

    fn stage_file(&mut self, path: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.stage_file(path),
            Self::Mock(inner) => inner.stage_file(path),
        }
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.unstage_file(path),
            Self::Mock(inner) => inner.unstage_file(path),
        }
    }

    fn stage_files(&mut self, paths: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.stage_files(paths),
            Self::Mock(inner) => inner.stage_files(paths),
        }
    }

    fn unstage_files(&mut self, paths: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.unstage_files(paths),
            Self::Mock(inner) => inner.unstage_files(paths),
        }
    }

    fn create_commit(&mut self, message: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.create_commit(message),
            Self::Mock(inner) => inner.create_commit(message),
        }
    }

    fn create_branch(
        &mut self,
        name: &str,
        start_point: &str,
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.create_branch(name, start_point),
            Self::Mock(inner) => inner.create_branch(name, start_point),
        }
    }

    fn checkout_branch(
        &mut self,
        name: &str,
        auto_stash: bool,
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.checkout_branch(name, auto_stash),
            Self::Mock(inner) => inner.checkout_branch(name, auto_stash),
        }
    }

    fn delete_branch(
        &mut self,
        name: &str,
        mode: BranchDeleteMode,
        force: bool,
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.delete_branch(name, mode, force),
            Self::Mock(inner) => inner.delete_branch(name, mode, force),
        }
    }

    fn rebase_branch(
        &mut self,
        target: &str,
        interactive: bool,
        auto_stash: bool,
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.rebase_branch(target, interactive, auto_stash),
            Self::Mock(inner) => inner.rebase_branch(target, interactive, auto_stash),
        }
    }

    fn squash_commits(&mut self, commit_ids: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.squash_commits(commit_ids),
            Self::Mock(inner) => inner.squash_commits(commit_ids),
        }
    }

    fn fixup_commits(&mut self, commit_ids: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.fixup_commits(commit_ids),
            Self::Mock(inner) => inner.fixup_commits(commit_ids),
        }
    }

    fn reword_commit(
        &mut self,
        commit_id: &str,
        message: &str,
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.reword_commit(commit_id, message),
            Self::Mock(inner) => inner.reword_commit(commit_id, message),
        }
    }

    fn delete_commits(&mut self, commit_ids: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.delete_commits(commit_ids),
            Self::Mock(inner) => inner.delete_commits(commit_ids),
        }
    }

    fn checkout_commit_detached(
        &mut self,
        commit_id: &str,
        auto_stash: bool,
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.checkout_commit_detached(commit_id, auto_stash),
            Self::Mock(inner) => inner.checkout_commit_detached(commit_id, auto_stash),
        }
    }

    fn stash_push(&mut self, message: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.stash_push(message),
            Self::Mock(inner) => inner.stash_push(message),
        }
    }

    fn stash_files(
        &mut self,
        message: &str,
        paths: &[String],
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.stash_files(message, paths),
            Self::Mock(inner) => inner.stash_files(message, paths),
        }
    }

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.stash_pop(stash_id),
            Self::Mock(inner) => inner.stash_pop(stash_id),
        }
    }

    fn reset(&mut self, mode: ratagit_core::ResetMode) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.reset(mode),
            Self::Mock(inner) => inner.reset(mode),
        }
    }

    fn nuke(&mut self) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.nuke(),
            Self::Mock(inner) => inner.nuke(),
        }
    }

    fn discard_files(&mut self, paths: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Hybrid(inner) => inner.discard_files(paths),
            Self::Mock(inner) => inner.discard_files(paths),
        }
    }
}

fn select_backend() -> Result<AppBackend, Box<dyn Error>> {
    let cwd = std::env::current_dir()?;
    if is_git_repo(&cwd) {
        Ok(AppBackend::Hybrid(HybridGitBackend::open(cwd)?))
    } else {
        Ok(AppBackend::Mock(MockGitBackend::new(fixture_dirty_repo())))
    }
}
