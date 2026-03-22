use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

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
                ctx.app.dirty.mark_main_content();
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
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
                ctx.app.dirty.mark_all();
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
            }
            DomainAction::SearchClear => {
                ctx.app.clear_search();
                ctx.app.cancel_input();
                ctx.app.push_log("search cleared", true);
                ctx.app.dirty.mark_all();
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
            }
            DomainAction::SearchNext => {
                if ctx.app.search_jump_next() {
                    ctx.app.dirty.mark_main_content();
                    ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                } else {
                    ReduceOutput::none()
                }
            }
            DomainAction::SearchPrev => {
                if ctx.app.search_jump_prev() {
                    ctx.app.dirty.mark_main_content();
                    ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                } else {
                    ReduceOutput::none()
                }
            }
            _ => ReduceOutput::none(),
        }
    }
}
