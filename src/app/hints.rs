use crate::app::AppState;

impl AppState {
    pub fn shortcut_hints(&self) -> Vec<(String, String)> {
        super::commands::shortcut_hints(self.keymap(), &self.command_env())
    }
}
