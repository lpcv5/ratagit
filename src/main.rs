mod input;

use std::error::Error;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use input::{KeyEffect, key_effect_for_key};
use ratagit_core::{AppState, UiAction};
use ratagit_git::{GitBackend, HybridGitBackend, SharedMockGitBackend, is_git_repo};
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
    let observe_config = ObserveConfig::from_env();
    let _observe_guard = init_observability(&observe_config).ok();
    run_tui()
}

fn run_tui() -> Result<(), Box<dyn Error>> {
    let backend_factory = select_backend_factory()?;
    let mut terminal = setup_terminal()?;
    let mut runtime = build_initial_runtime(backend_factory);
    runtime.dispatch_ui(UiAction::RefreshAll);

    loop {
        runtime.tick();
        terminal.draw(|frame| {
            render_terminal(frame, runtime.state());
        })?;

        if !event::poll(input_poll_interval())? {
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

type BackendFactory = Box<dyn Fn() -> Box<dyn GitBackend + Send> + Send + Sync>;

fn build_initial_runtime(
    backend_factory: BackendFactory,
) -> AsyncRuntime<Box<dyn GitBackend + Send>> {
    AsyncRuntime::new(
        AppState::default(),
        backend_factory,
        initial_terminal_size(),
    )
    .with_debounce_window(runtime_debounce_window())
}

fn initial_terminal_size() -> TerminalSize {
    TerminalSize {
        width: 100,
        height: 30,
    }
}

fn input_poll_interval() -> Duration {
    Duration::from_millis(16)
}

fn runtime_debounce_window() -> Duration {
    Duration::from_millis(80)
}

fn select_backend_factory() -> Result<BackendFactory, Box<dyn Error>> {
    let cwd = std::env::current_dir()?;
    select_backend_factory_for(cwd)
}

fn select_backend_factory_for(cwd: PathBuf) -> Result<BackendFactory, Box<dyn Error>> {
    if is_git_repo(Path::new(&cwd)) {
        HybridGitBackend::open(Path::new(&cwd))?;
        Ok(Box::new(move || {
            Box::new(
                HybridGitBackend::open(Path::new(&cwd))
                    .expect("validated git repository should remain openable"),
            )
        }))
    } else {
        let shared_backend = SharedMockGitBackend::new(fixture_dirty_repo());
        Ok(Box::new(move || Box::new(shared_backend.clone())))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{create_dir_all, remove_dir_all};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use ratagit_git::execute_command;

    use super::*;

    #[test]
    fn startup_constants_are_intentional_regression_points() {
        assert_eq!(
            initial_terminal_size(),
            TerminalSize {
                width: 100,
                height: 30
            }
        );
        assert_eq!(input_poll_interval(), Duration::from_millis(16));
        assert_eq!(runtime_debounce_window(), Duration::from_millis(80));
    }

    #[test]
    fn non_git_backend_factory_builds_shared_mock_runtime_that_can_render() {
        let root = unique_temp_dir("main-non-git");
        create_dir_all(&root).expect("temp dir should be creatable");

        let factory =
            select_backend_factory_for(root.clone()).expect("non-git backend should be selected");
        let mut runtime = build_initial_runtime(factory);
        runtime.dispatch_ui(UiAction::RefreshAll);
        wait_for_refresh(&mut runtime);

        let screen = runtime.render_terminal_text();
        assert!(screen.contains("README.md"));
        assert!(screen.contains("[1]"));
        let _ = remove_dir_all(root);
    }

    #[test]
    fn git_backend_factory_opens_isolated_repository() {
        if !git_available() {
            eprintln!("git is unavailable, skipping git_backend_factory_opens_isolated_repository");
            return;
        }

        let root = unique_temp_dir("main-git");
        create_dir_all(&root).expect("temp dir should be creatable");
        run_git(&root, &["init"]);
        run_git(&root, &["config", "user.name", "ratagit-tests"]);
        run_git(
            &root,
            &["config", "user.email", "ratagit-tests@example.com"],
        );

        let factory =
            select_backend_factory_for(root.clone()).expect("git backend should be selected");
        let mut backend = factory();
        let result = execute_command(&mut backend, ratagit_core::Command::RefreshAll);
        assert!(matches!(result, ratagit_core::GitResult::Refreshed(_)));
        let _ = remove_dir_all(root);
    }

    fn wait_for_refresh(runtime: &mut AsyncRuntime<Box<dyn GitBackend + Send>>) {
        for _ in 0..100 {
            runtime.tick();
            if runtime.state().status.refresh_count > 0 {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for startup refresh");
    }

    fn unique_temp_dir(case_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ratagit-{case_name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ))
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn run_git(repo_path: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .expect("git command should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
