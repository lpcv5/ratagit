use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;

/// A command produced by a store reducer, to be executed by the main loop.
#[derive(Debug)]
pub enum Command {
    /// No operation — matched exhaustively but never constructed by stores.
    /// Stores use `ReduceOutput::none()` (empty commands vec) instead.
    #[allow(dead_code)]
    None,
    /// A synchronous action to dispatch immediately after the current action.
    Sync(DomainAction),
    /// An async effect to run in the effect runtime.
    Effect(EffectRequest),
}
