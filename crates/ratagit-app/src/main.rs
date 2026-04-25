use std::error::Error;
use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratagit_core::{AppState, PanelFocus, UiAction};
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
    );
    runtime.dispatch_ui(UiAction::RefreshAll);

    loop {
        terminal.draw(|frame| {
            render_terminal(frame, runtime.state());
        })?;

        if !event::poll(Duration::from_millis(100))? {
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
            KeyCode::Char('r') => runtime.dispatch_ui(UiAction::RefreshAll),
            KeyCode::Tab => runtime.dispatch_ui(UiAction::FocusNext),
            KeyCode::BackTab => runtime.dispatch_ui(UiAction::FocusPrev),
            KeyCode::Down | KeyCode::Char('j') => runtime.dispatch_ui(UiAction::MoveDown),
            KeyCode::Up | KeyCode::Char('k') => runtime.dispatch_ui(UiAction::MoveUp),
            KeyCode::Char('1') => runtime.dispatch_ui(UiAction::FocusPanel {
                panel: PanelFocus::Files,
            }),
            KeyCode::Char('2') => runtime.dispatch_ui(UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            }),
            KeyCode::Char('3') => runtime.dispatch_ui(UiAction::FocusPanel {
                panel: PanelFocus::Commits,
            }),
            KeyCode::Char('4') => runtime.dispatch_ui(UiAction::FocusPanel {
                panel: PanelFocus::Stash,
            }),
            KeyCode::Char('5') => runtime.dispatch_ui(UiAction::FocusPanel {
                panel: PanelFocus::Details,
            }),
            KeyCode::Char('6') => runtime.dispatch_ui(UiAction::FocusPanel {
                panel: PanelFocus::Log,
            }),
            KeyCode::Char('s') => runtime.dispatch_ui(UiAction::StageSelectedFile),
            KeyCode::Char('u') => runtime.dispatch_ui(UiAction::UnstageSelectedFile),
            KeyCode::Char('c') => runtime.dispatch_ui(UiAction::CreateCommit {
                message: "mvp commit".to_string(),
            }),
            KeyCode::Char('b') => runtime.dispatch_ui(UiAction::CreateBranch {
                name: "feature/new".to_string(),
            }),
            KeyCode::Char('o') => runtime.dispatch_ui(UiAction::CheckoutSelectedBranch),
            KeyCode::Char('p') => runtime.dispatch_ui(UiAction::StashPush {
                message: "savepoint".to_string(),
            }),
            KeyCode::Char('O') => runtime.dispatch_ui(UiAction::StashPopSelected),
            _ => {}
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
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

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), ratagit_git::GitError> {
        match self {
            Self::Cli(inner) => inner.stash_pop(stash_id),
            Self::Mock(inner) => inner.stash_pop(stash_id),
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
