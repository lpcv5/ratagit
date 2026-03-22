use super::{dispatch_test_action, App, Command};
use crate::flux::action::DomainAction;
use std::collections::VecDeque;

/// Replays a deterministic interaction trace for behavior-regression tests.
pub fn replay_actions(app: &mut App, trace: &[DomainAction]) {
    let mut queue: VecDeque<DomainAction> = trace.iter().cloned().collect();

    while let Some(action) = queue.pop_front() {
        if let Some(command) = dispatch_test_action(app, action) {
            match command {
                Command::None => {}
                Command::Sync(action) => {
                    queue.push_front(action);
                }
                // Effect requests are executed by runtime loops and are intentionally not run here.
                Command::Effect(_) => {}
            }
        }
    }
}
