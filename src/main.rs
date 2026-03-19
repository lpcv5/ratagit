mod app;
mod config;
mod git;
mod ui;

use app::{App, update};
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

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
    loop {
        // Comment in English.
        terminal.draw(|f| {
            app.render(f);
        })?;

        // Comment in English.
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                let msg = app.handle_key(key);

                if let Some(msg) = msg {
                    // Comment in English.
                    let cmd = update(app, msg);

                    // Comment in English.
                    if cmd.is_some() {
                        // TODO: implement this behavior.
                    }
                }
            }
        }

        // Comment in English.
        if !app.running {
            return Ok(());
        }
    }
}
