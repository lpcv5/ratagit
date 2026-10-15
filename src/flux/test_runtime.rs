use crate::app::{App, Command};
use crate::flux::action::Action;
use crate::flux::dispatcher::Dispatcher;
use crate::flux::effects::{self, EffectRequest};
use std::collections::VecDeque;

/// Test scheduling is deterministic, but Git execution and completion events
/// use exactly the same implementation as the production effect runtime.
pub fn run_inline_effect(app: &mut App, request: EffectRequest) -> Option<Vec<Action>> {
    if matches!(
        request,
        EffectRequest::LoadBackground { .. }
            | EffectRequest::StageHunks { .. }
            | EffectRequest::ApplyHunks { .. }
            | EffectRequest::ApplyLines { .. }
            | EffectRequest::ControlRepositoryOperation { .. }
            | EffectRequest::StagePaths(_)
            | EffectRequest::ToggleStageSelection { .. }
            | EffectRequest::PrepareCommitFromVisualSelection { .. }
    ) {
        Some(effects::execute(request, &*app.runtime.repo))
    } else {
        None
    }
}

pub fn drive_actions(app: &mut App, actions: impl IntoIterator<Item = Action>) -> Option<Command> {
    let mut dispatcher = Dispatcher::with_initial_output(std::mem::take(&mut app.initial_output));
    let mut pending: VecDeque<Action> = actions.into_iter().collect();
    let mut external = None;
    loop {
        let commands = if let Some(action) = pending.pop_front() {
            let envelope = dispatcher.next_envelope(action);
            dispatcher.dispatch(app, envelope).commands
        } else {
            let commands = dispatcher.take_initial_commands();
            if commands.is_empty() {
                break;
            }
            commands
        };
        for command in commands {
            match command {
                Command::RunEditor(command) => {
                    if external.is_none() {
                        external = Some(Command::RunEditor(command));
                    }
                }
                Command::None => {}
                Command::Sync(action) => pending.push_back(Action::Domain(action)),
                Command::Effect(request) => match run_inline_effect(app, request.clone()) {
                    Some(actions) => pending.extend(actions),
                    None => {
                        if external.is_none() {
                            external = Some(Command::Effect(request));
                        }
                    }
                },
            }
        }
    }
    external
}

pub fn finish_background(app: &mut App) {
    let _ = drive_actions(app, []);
}
