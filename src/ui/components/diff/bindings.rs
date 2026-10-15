/// Semantic behavior ports. Keymaps, mouse gestures, toolbars and other owners
/// can share these intents without exposing application actions to the widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffIntent {
    Focus,
    Blur,
    SwitchSide,
    NextLine,
    PreviousLine,
    NextHunk,
    PreviousHunk,
    ToggleSelection,
    ToggleMode,
    ToggleRange,
    ClearSelection,
    ApplySelection,
    StageSelection,
    UnstageSelection,
    DiscardSelection,
    NextFile,
    PreviousFile,
    ScrollUp,
    ScrollDown,
}

/// Bind UI intent to an owner's action. `None` means the behavior is currently
/// unavailable. Resolving a binding does not execute the action or mutate data.
pub trait DiffBehaviorBindings {
    type Action;

    fn resolve_diff_intent(&self, intent: DiffIntent) -> Option<Self::Action>;
}

impl<Action, Handler> DiffBehaviorBindings for Handler
where
    Handler: Fn(DiffIntent) -> Option<Action>,
{
    type Action = Action;

    fn resolve_diff_intent(&self, intent: DiffIntent) -> Option<Self::Action> {
        self(intent)
    }
}
