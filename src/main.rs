mod app;
mod config;
mod git;
mod ui;

use app::{update, App};
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::mpsc::{Receiver, TryRecvError};

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
    let mut async_commands: Vec<Receiver<app::Message>> = Vec::new();

    loop {
        // Comment in English.
        terminal.draw(|f| {
            app.render(f);
        })?;

        // Comment in English.
        if event::poll(std::time::Duration::from_millis(16))? {
            loop {
                if let Event::Key(key) = event::read()? {
                    if key.kind == crossterm::event::KeyEventKind::Press {
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
