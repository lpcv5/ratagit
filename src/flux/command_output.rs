use crate::app::Command;
use crate::background::TaskKey;
#[cfg(test)]
use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;

/// Publication order is explicit: store commands precede coordinator requests,
/// preserving the original dispatch contract without a state-owned outbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PublicationOrder {
    Store,
    Coordinator,
}

#[derive(Debug, Default)]
pub struct CommandOutput {
    entries: Vec<(PublicationOrder, Command)>,
}

impl CommandOutput {
    pub(super) fn command(&mut self, command: Command) {
        self.entries.push((PublicationOrder::Store, command));
    }

    pub fn effect(&mut self, request: EffectRequest) {
        self.entries
            .push((PublicationOrder::Coordinator, Command::Effect(request)));
    }

    #[cfg(test)]
    pub fn action(&mut self, action: DomainAction) {
        self.entries
            .push((PublicationOrder::Coordinator, Command::Sync(action)));
    }

    /// Cancel only unpublished coordinator reads, as the former state outbox did.
    pub fn cancel_background(&mut self, key: &TaskKey) {
        self.entries.retain(|(order, command)| {
            !matches!((order, command),
                (PublicationOrder::Coordinator, Command::Effect(EffectRequest::LoadBackground { request, .. }))
                if &request.key == key)
        });
    }

    pub fn drain(&mut self) -> Vec<Command> {
        // Stable ordering also preserves insertion order within each origin.
        self.entries.sort_by_key(|(order, _)| *order);
        std::mem::take(&mut self.entries)
            .into_iter()
            .map(|(_, command)| command)
            .collect()
    }
}
