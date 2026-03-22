use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;

/// Documentation comment in English.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Command {
    /// Documentation comment in English.
    None,
    /// Documentation comment in English.
    Sync(DomainAction),
    /// Documentation comment in English.
    Effect(EffectRequest),
}
