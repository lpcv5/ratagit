use crate::flux::action::{DomainAction, RuntimeEvent};
use crate::flux::effects::EffectRequest;

/// An owned command published by the dispatcher for the effect loop.
#[derive(Debug)]
pub enum Command {
    /// The UI owns terminal handoff; repository preparation is an effect.
    RunEditor(crate::desktop::EditorCommand),
    /// No operation — matched exhaustively but never constructed by stores.
    /// Reducers omit command emission for no-op updates instead.
    #[allow(dead_code)]
    None,
    /// A follow-up action routed back through the normal action queue.
    Sync(DomainAction),
    /// An async effect to run in the effect runtime.
    Effect(EffectRequest),
}

impl Command {
    /// A state transition queued for the dispatcher, never the I/O executor.
    pub fn runtime(event: RuntimeEvent) -> Self {
        Self::Sync(DomainAction::Runtime(event))
    }
}
