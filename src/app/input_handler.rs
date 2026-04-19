use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::events::AppEvent;
use crate::app::AppState;
use crate::components::component_v2::ComponentV2;

use super::ui_state::Panel;
use super::App;

const MAIN_VIEW_PAGE_SCROLL: i16 = 12;

impl App {
    /// Event-driven input handler using ComponentV2 and process_event
    pub(super) fn handle_input_v2(&mut self, event: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            if self.try_handle_escape(key.code)? {
                return Ok(());
            }

            // Handle global keybindings first
            match key.code {
                KeyCode::Char('q') => {
                    self.state.should_quit = true;
                    return Ok(());
                }
                KeyCode::Char('1') if key.modifiers.is_empty() => {
                    self.process_event(AppEvent::SwitchPanel(Panel::Files));
                    return Ok(());
                }
                KeyCode::Char('2') if key.modifiers.is_empty() => {
                    self.process_event(AppEvent::SwitchPanel(Panel::Branches));
                    return Ok(());
                }
                KeyCode::Char('3') if key.modifiers.is_empty() => {
                    self.process_event(AppEvent::SwitchPanel(Panel::Commits));
                    return Ok(());
                }
                KeyCode::Char('4') if key.modifiers.is_empty() => {
                    self.process_event(AppEvent::SwitchPanel(Panel::Stash));
                    return Ok(());
                }
                KeyCode::Char('l') if key.modifiers.is_empty() => {
                    let next = next_left_panel(self.state.ui_state.active_panel);
                    self.process_event(AppEvent::SwitchPanel(next));
                    return Ok(());
                }
                KeyCode::Char('h') if key.modifiers.is_empty() => {
                    let prev = previous_left_panel(self.state.ui_state.active_panel);
                    self.process_event(AppEvent::SwitchPanel(prev));
                    return Ok(());
                }
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    if self.state.ui_state.active_panel != Panel::Log {
                        self.scroll_main_view_by(-MAIN_VIEW_PAGE_SCROLL);
                        return Ok(());
                    }
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    if self.state.ui_state.active_panel != Panel::Log {
                        self.scroll_main_view_by(MAIN_VIEW_PAGE_SCROLL);
                        return Ok(());
                    }
                }
                KeyCode::Char('?') => {
                    self.process_event(AppEvent::Modal(crate::app::events::ModalEvent::ShowHelp));
                    return Ok(());
                }
                _ => {}
            }

            // Dispatch to active panel using ComponentV2
            // We need to work around borrow checker by extracting the event first
            // then processing it. For now, we'll use a temporary workaround.
            let active_panel = self.state.ui_state.active_panel;

            // Create a reference to state that we can pass to components
            // This is safe because we're only reading from state, not mutating it
            let state_ref = &self.state as *const AppState;
            let app_event = unsafe {
                match active_panel {
                    Panel::Files => self
                        .state
                        .components
                        .file_list_panel
                        .handle_key_event(key, &*state_ref),
                    Panel::Branches => self
                        .state
                        .components
                        .branch_list_panel
                        .handle_key_event(key, &*state_ref),
                    Panel::Commits => self
                        .state
                        .components
                        .commit_panel
                        .handle_key_event(key, &*state_ref),
                    Panel::Stash => self
                        .state
                        .components
                        .stash_list_panel
                        .handle_key_event(key, &*state_ref),
                    Panel::MainView => self
                        .state
                        .components
                        .main_view_panel
                        .handle_key_event(key, &*state_ref),
                    Panel::Log => self
                        .state
                        .components
                        .log_panel
                        .handle_key_event(key, &*state_ref),
                }
            };
            self.process_event(app_event);

            // Refresh main view if needed
            if self.should_refresh_commit_tree_diff(&event) {
                self.update_main_view_for_active_panel()?;
            }
        }

        Ok(())
    }

    fn should_refresh_commit_tree_diff(&self, event: &Event) -> bool {
        matches!(
            event,
            Event::Key(key) if matches!(
                key.code,
            KeyCode::Char('j')
                | KeyCode::Char('k')
                | KeyCode::Down
                | KeyCode::Up
                | KeyCode::Enter
                | KeyCode::Esc
        )
        )
    }

    fn try_handle_escape(&mut self, key_code: KeyCode) -> anyhow::Result<bool> {
        if key_code != KeyCode::Esc {
            return Ok(false);
        }

        let event = self
            .state
            .components
            .handle_escape(self.state.ui_state.active_panel);
        if event == AppEvent::None {
            return Ok(false);
        }

        self.process_event(event);
        self.update_main_view_for_active_panel()?;
        Ok(true)
    }
}

pub(super) fn next_left_panel(current: Panel) -> Panel {
    match current {
        Panel::Files => Panel::Branches,
        Panel::Branches => Panel::Commits,
        Panel::Commits => Panel::Stash,
        Panel::Stash => Panel::Files,
        Panel::MainView | Panel::Log => Panel::Files,
    }
}

pub(super) fn previous_left_panel(current: Panel) -> Panel {
    match current {
        Panel::Files => Panel::Stash,
        Panel::Branches => Panel::Files,
        Panel::Commits => Panel::Branches,
        Panel::Stash => Panel::Commits,
        Panel::MainView | Panel::Log => Panel::Stash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyEvent};
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::TryRecvError;

    fn create_test_app() -> (App, mpsc::Receiver<crate::backend::CommandEnvelope>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(8);
        let (_event_tx, event_rx) = mpsc::channel(8);
        (App::new(cmd_tx, event_rx), cmd_rx)
    }

    #[test]
    fn r_key_no_longer_triggers_global_refresh() {
        let (mut app, mut cmd_rx) = create_test_app();
        app.state.ui_state.active_panel = Panel::Files;

        let input = Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        app.handle_input_v2(input).expect("input handling failed");

        assert!(matches!(cmd_rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn esc_in_branch_commits_subview_uses_global_escape_dispatch() {
        let (mut app, _cmd_rx) = create_test_app();
        app.state.ui_state.active_panel = Panel::Branches;
        app.state.components.show_branch_commits();
        app.state.data_cache.saved_commits = Some(vec![crate::backend::git_ops::CommitEntry {
            short_id: "old1234".to_string(),
            id: "old123".to_string(),
            summary: "Old commit".to_string(),
            body: None,
            author: "Author".to_string(),
            timestamp: 1704067200,
        }]);
        app.state.data_cache.commits = vec![crate::backend::git_ops::CommitEntry {
            short_id: "new1234".to_string(),
            id: "new123".to_string(),
            summary: "New commit".to_string(),
            body: None,
            author: "Author".to_string(),
            timestamp: 1704153600,
        }];

        let input = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.handle_input_v2(input).expect("input handling failed");

        assert!(app.state.data_cache.saved_commits.is_none());
        assert_eq!(app.state.data_cache.commits.len(), 1);
        assert_eq!(app.state.data_cache.commits[0].id, "old123");
    }
}
