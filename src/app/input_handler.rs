use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

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
                    self.scroll_main_view_by(-MAIN_VIEW_PAGE_SCROLL);
                    return Ok(());
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    self.scroll_main_view_by(MAIN_VIEW_PAGE_SCROLL);
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
