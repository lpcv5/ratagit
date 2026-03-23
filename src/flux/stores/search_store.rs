use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct SearchStore;

impl SearchStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for SearchStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::SearchSetQuery(query) => {
                let count = ctx.app.apply_search_query(query.clone());
                if count > 0 {
                    ctx.app.search_select_initial_match();
                }
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                    .with_invalidation(UiInvalidation::all())
            }
            DomainAction::SearchConfirm => {
                let count = ctx.app.apply_search_query(ctx.app.search_query.clone());
                if count == 0 {
                    ctx.app
                        .push_log(format!("search no match: {}", ctx.app.search_query), false);
                } else {
                    ctx.app.push_log(
                        format!("search match {}: {}", count, ctx.app.search_query),
                        true,
                    );
                }
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                    .with_invalidation(UiInvalidation::all())
            }
            DomainAction::SearchClear => {
                ctx.app.clear_search();
                ctx.app.cancel_input();
                ctx.app.push_log("search cleared", true);
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                    .with_invalidation(UiInvalidation::all())
            }
            DomainAction::SearchNext => {
                if ctx.app.search_jump_next() {
                    ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                        .with_invalidation(UiInvalidation::all())
                } else {
                    ReduceOutput::none()
                }
            }
            DomainAction::SearchPrev => {
                if ctx.app.search_jump_prev() {
                    ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                        .with_invalidation(UiInvalidation::all())
                } else {
                    ReduceOutput::none()
                }
            }
            _ => ReduceOutput::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_search_set_query_updates_buffer() {
        let mut store = SearchStore::new();
        let mut app = mock_app();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::SearchSetQuery("foo".to_string())),
        );
        assert_eq!(app.search_query, "foo");
    }

    #[test]
    fn test_search_clear_resets_state() {
        let mut store = SearchStore::new();
        let mut app = mock_app();
        app.search_query = "foo".to_string();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::SearchClear),
        );
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_search_next_returns_output() {
        let mut store = SearchStore::new();
        let mut app = mock_app();
        // SearchNext with no matches is a no-op
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::SearchNext),
        );
        // just verify it compiles and runs without panic
        let _ = output;
    }

    #[test]
    fn test_search_prev_returns_output() {
        let mut store = SearchStore::new();
        let mut app = mock_app();
        // SearchPrev with no matches is a no-op
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::SearchPrev),
        );
        let _ = output;
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = SearchStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
        assert_eq!(output.invalidation, UiInvalidation::none());
    }
}
