mod input;

use std::error::Error;
use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use input::{KeyEffect, key_effect_for_key};
use ratagit_core::{AppState, UiAction};
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

fn select_backend() -> Result<Box<dyn GitBackend + Send>, Box<dyn Error>> {
    let cwd = std::env::current_dir()?;
    if is_git_repo(&cwd) {
        Ok(Box::new(HybridGitBackend::open(cwd)?))
    } else {
        Ok(Box::new(MockGitBackend::new(fixture_dirty_repo())))
    }
}
