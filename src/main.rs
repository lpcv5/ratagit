mod app;
mod config;
mod git;
mod ui;

use app::{update, App};
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

fn main() -> Result<()> {
    // Comment in English.
    color_eyre::install()?;

    // Comment in English.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Comment in English.
    let mut app = App::new()?;

    // Comment in English.
    let res = run_app(&mut terminal, &mut app);

    // Comment in English.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    const MAX_EVENTS_PER_FRAME: usize = 8;
    const DIFF_RELOAD_DEBOUNCE: Duration = Duration::from_millis(80);
    let mut async_commands: Vec<Receiver<app::Message>> = Vec::new();

    loop {
        // Render only if dirty
        if app.dirty.is_dirty() {
            terminal.draw(|f| {
                app.render(f);
            })?;
            app.dirty.clear();
        }

        // Comment in English.
        if event::poll(std::time::Duration::from_millis(16))? {
            for _ in 0..MAX_EVENTS_PER_FRAME {
                if let Event::Key(key) = event::read()? {
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                        let msg = app.handle_key(key);

                        if let Some(msg) = msg {
                            apply_message(app, msg, &mut async_commands);
                        }
                    }
                }

                if !event::poll(std::time::Duration::from_millis(0))? {
                    break;
                }
            }
        }

        let mut idx = 0usize;
        while idx < async_commands.len() {
            match async_commands[idx].try_recv() {
                Ok(msg) => {
                    apply_message(app, msg, &mut async_commands);
                    async_commands.swap_remove(idx);
                }
                Err(TryRecvError::Empty) => {
                    idx += 1;
                }
                Err(TryRecvError::Disconnected) => {
                    async_commands.swap_remove(idx);
                }
            }
        }

        if let Err(err) = app.flush_pending_refresh() {
            app.push_log(format!("refresh failed: {}", err), false);
        }
        if app.has_pending_diff_reload() && app.diff_reload_debounce_elapsed(DIFF_RELOAD_DEBOUNCE)
        {
            app.flush_pending_diff_reload();
        }

        // Comment in English.
        if !app.running {
            return Ok(());
        }
    }
}

fn apply_message(
    app: &mut App,
    msg: app::Message,
    async_commands: &mut Vec<Receiver<app::Message>>,
) {
    if let Some(command) = update(app, msg) {
        apply_command(app, command, async_commands);
    }
}

fn apply_command(
    app: &mut App,
    command: app::Command,
    async_commands: &mut Vec<Receiver<app::Message>>,
) {
    match command {
        app::Command::None => {}
        app::Command::Sync(msg) => apply_message(app, msg, async_commands),
        app::Command::Async(rx) => async_commands.push(rx),
    }
}
