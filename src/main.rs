use std::error::Error;
use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratagit_core::{AppState, FileInputMode, PanelFocus, UiAction};
use ratagit_git::{CliGitBackend, GitBackend, MockGitBackend, is_git_repo};
use ratagit_harness::Runtime;
use ratagit_observe::{ObserveConfig, init_observability};
use ratagit_testkit::fixture_dirty_repo;
use ratagit_ui::{TerminalSize, render_terminal};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() -> Result<(), Box<dyn Error>> {
    let _ = init_observability(&ObserveConfig::default());
    run_tui()
}

fn run_tui() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;
    let backend = select_backend()?;
    let mut runtime = Runtime::new(
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
        terminal.draw(|frame| {
            render_terminal(frame, runtime.state());
        })?;

        if !event::poll(Duration::from_millis(100))? {
            runtime.tick();
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('q') => break,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
            _ => {
                if let Some(action) = ui_action_for_key(runtime.state(), key.code, key.modifiers) {
                    runtime.dispatch_ui(action);
                }
            }
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}

fn ui_action_for_key(state: &AppState, code: KeyCode, modifiers: KeyModifiers) -> Option<UiAction> {
    if state.editor.is_active() {
        return match code {
            KeyCode::Enter => Some(UiAction::EditorConfirm),
            KeyCode::Esc => Some(UiAction::EditorCancel),
            KeyCode::Backspace => Some(UiAction::EditorBackspace),
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
        KeyCode::Char('b') => Some(UiAction::CreateBranch {
            name: "feature/new".to_string(),
        }),
        KeyCode::Char('o') => Some(UiAction::CheckoutSelectedBranch),
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

    fn map_key(state: &AppState, code: KeyCode) -> Option<UiAction> {
        ui_action_for_key(state, code, KeyModifiers::NONE)
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
            map_key(&state, KeyCode::Char('b')),
            Some(UiAction::CreateBranch {
                name: "feature/new".to_string()
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('o')),
            Some(UiAction::CheckoutSelectedBranch)
        );

        state.focus = PanelFocus::Commits;
        assert_eq!(
            map_key(&state, KeyCode::Char('c')),
            Some(UiAction::CreateCommit {
                message: "mvp commit".to_string()
            })
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
        let mut state = AppState::default();
        state.editor.kind = Some(ratagit_core::EditorKind::Commit {
            message: String::new(),
            body: String::new(),
            active_field: ratagit_core::CommitField::Body,
        });
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
            ui_action_for_key(&state, KeyCode::Char('j'), KeyModifiers::CONTROL),
            Some(UiAction::EditorInsertNewline)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('m')),
            Some(UiAction::EditorInputChar('m'))
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

#[derive(Debug, Clone)]
enum AppBackend {
    Cli(CliGitBackend),
    Mock(MockGitBackend),
}

impl GitBackend for AppBackend {
    fn refresh_snapshot(&mut self) -> Result<ratagit_core::RepoSnapshot, ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.refresh_snapshot(),
            Self::Mock(inner) => inner.refresh_snapshot(),
        }
    }

    fn files_details_diff(&mut self, paths: &[String]) -> Result<String, ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.files_details_diff(paths),
            Self::Mock(inner) => inner.files_details_diff(paths),
        }
    }

    fn stage_file(&mut self, path: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.stage_file(path),
            Self::Mock(inner) => inner.stage_file(path),
        }
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.unstage_file(path),
            Self::Mock(inner) => inner.unstage_file(path),
        }
    }

    fn stage_files(&mut self, paths: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.stage_files(paths),
            Self::Mock(inner) => inner.stage_files(paths),
        }
    }

    fn unstage_files(&mut self, paths: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.unstage_files(paths),
            Self::Mock(inner) => inner.unstage_files(paths),
        }
    }

    fn create_commit(&mut self, message: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.create_commit(message),
            Self::Mock(inner) => inner.create_commit(message),
        }
    }

    fn create_branch(&mut self, name: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.create_branch(name),
            Self::Mock(inner) => inner.create_branch(name),
        }
    }

    fn checkout_branch(&mut self, name: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.checkout_branch(name),
            Self::Mock(inner) => inner.checkout_branch(name),
        }
    }

    fn stash_push(&mut self, message: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.stash_push(message),
            Self::Mock(inner) => inner.stash_push(message),
        }
    }

    fn stash_files(
        &mut self,
        message: &str,
        paths: &[String],
    ) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.stash_files(message, paths),
            Self::Mock(inner) => inner.stash_files(message, paths),
        }
    }

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.stash_pop(stash_id),
            Self::Mock(inner) => inner.stash_pop(stash_id),
        }
    }

    fn discard_files(&mut self, paths: &[String]) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.discard_files(paths),
            Self::Mock(inner) => inner.discard_files(paths),
        }
    }
}

fn select_backend() -> Result<AppBackend, Box<dyn Error>> {
    let cwd = std::env::current_dir()?;
    if is_git_repo(&cwd) {
        Ok(AppBackend::Cli(CliGitBackend::new(cwd)))
    } else {
        Ok(AppBackend::Mock(MockGitBackend::new(fixture_dirty_repo())))
    }
}
