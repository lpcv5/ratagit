use crate::app::{App, Command};

pub(crate) fn handle_quit(app: &mut App) -> Option<Command> {
    app.running = false;
    None
}
