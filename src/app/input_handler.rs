use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::events::AppEvent;
use crate::app::AppState;
use crate::components::component_v2::ComponentV2;
use crate::components::panels::CommitModeView;
use crate::components::Intent;

use super::ui_state::Panel;
use super::App;

const MAIN_VIEW_PAGE_SCROLL: i16 = 12;

impl App {
    pub(super) fn handle_input(&mut self, event: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') => {
                    self.state.should_quit = true;
                    return Ok(());
                }
                KeyCode::Char('1') if key.modifiers.is_empty() => {
                    self.execute_intent(Intent::SwitchFocus(Panel::Files))?;
                    return Ok(());
                }
                KeyCode::Char('2') if key.modifiers.is_empty() => {
                    self.execute_intent(Intent::SwitchFocus(Panel::Branches))?;
                    return Ok(());
                }
                KeyCode::Char('3') if key.modifiers.is_empty() => {
                    self.execute_intent(Intent::SwitchFocus(Panel::Commits))?;
                    return Ok(());
                }
                KeyCode::Char('4') if key.modifiers.is_empty() => {
                    self.execute_intent(Intent::SwitchFocus(Panel::Stash))?;
                    return Ok(());
                }
                KeyCode::Char('l') if key.modifiers.is_empty() => {
                    let next = next_left_panel(self.state.ui_state.active_panel);
                    self.execute_intent(Intent::SwitchFocus(next))?;
                    return Ok(());
                }
                KeyCode::Char('h') if key.modifiers.is_empty() => {
                    let prev = previous_left_panel(self.state.ui_state.active_panel);
                    self.execute_intent(Intent::SwitchFocus(prev))?;
                    return Ok(());
                }
                KeyCode::Char('r') if key.modifiers.is_empty() => {
                    self.request_refresh_all();
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
                    self.execute_intent(Intent::ShowHelp)?;
                    return Ok(());
                }
                _ => {}
            }
        }

        let intent = self.state.components.dispatch_event(
            self.state.ui_state.active_panel,
            &event,
            &self.state.data_cache,
        );
        self.execute_intent(intent)?;

        if self.should_refresh_commit_tree_diff(&event) {
            self.update_main_view_for_active_panel()?;
        }

        Ok(())
    }

    fn should_refresh_commit_tree_diff(&self, event: &Event) -> bool {
        if self.state.ui_state.active_panel != Panel::Commits {
            return false;
        }
        if !matches!(
            self.state.components.commit_mode_view(),
            CommitModeView::FilesTree { .. }
        ) {
            return false;
        }
        let Event::Key(key) = event else {
            return false;
        };
        if key.kind != KeyEventKind::Press {
            return false;
        }
        matches!(
            key.code,
            KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Down | KeyCode::Up | KeyCode::Enter
        )
    }

    /// Event-driven input handler using ComponentV2 and process_event
    pub(super) fn handle_input_v2(&mut self, event: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
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
                KeyCode::Char('r') if key.modifiers.is_empty() => {
                    self.request_refresh_all();
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
                    Panel::Files => self.state.components.file_list_panel.handle_key_event(key, &*state_ref),
                    Panel::Branches => self.state.components.branch_list_panel.handle_key_event(key, &*state_ref),
                    Panel::Commits => self.state.components.commit_panel.handle_key_event(key, &*state_ref),
                    Panel::Stash => self.state.components.stash_list_panel.handle_key_event(key, &*state_ref),
                    Panel::MainView => self.state.components.main_view_panel.handle_key_event(key, &*state_ref),
                    Panel::Log => self.state.components.log_panel.handle_key_event(key, &*state_ref),
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
