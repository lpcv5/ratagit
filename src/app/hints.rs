use crate::app::AppState;

impl AppState {
    pub fn shortcut_hints(&self) -> Vec<(String, String)> {
        if self.input.mode == Some(crate::app::InputMode::Keybindings) {
            return vec![
                ("Up/Down".into(), "Move".into()),
                ("Enter".into(), "Execute".into()),
                ("/".into(), "Filter".into()),
                ("Esc".into(), "Back".into()),
            ];
        }
        super::commands::shortcut_hints(self.keymap(), &self.command_env())
    }
}
