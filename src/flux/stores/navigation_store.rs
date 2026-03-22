use crate::app::graph_highlight::compute_highlight_set;
use crate::app::Command;
use crate::app::SidePanel;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct NavigationStore;

impl NavigationStore {
    pub fn new() -> Self {
        Self
    }

    fn recompute_commit_highlight(ctx: &mut ReduceCtx<'_>) {
        if ctx.app.active_panel == SidePanel::Commits && !ctx.app.commits.tree_mode.active {
            if let Some(idx) = ctx.app.commits.panel.list_state.selected() {
                if let Some(commit) = ctx.app.commits.items.get(idx) {
                    let oid = commit.oid.clone();
                    ctx.app.commits.highlighted_oids =
                        compute_highlight_set(&ctx.app.commits.items, &oid);
                    return;
                }
            }
        }
        ctx.app.commits.highlighted_oids.clear();
    }
}

impl Store for NavigationStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::PanelNext => {
                ctx.app.active_panel = match ctx.app.active_panel {
                    SidePanel::Files => SidePanel::LocalBranches,
                    SidePanel::LocalBranches => SidePanel::Commits,
                    SidePanel::Commits => SidePanel::Stash,
                    SidePanel::Stash => SidePanel::Files,
                };
                ctx.app.restore_search_for_active_scope();
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::EnsureCommitsLoadedForActivePanel,
                ));
            }
            DomainAction::PanelPrev => {
                ctx.app.active_panel = match ctx.app.active_panel {
                    SidePanel::Files => SidePanel::Stash,
                    SidePanel::LocalBranches => SidePanel::Files,
                    SidePanel::Commits => SidePanel::LocalBranches,
                    SidePanel::Stash => SidePanel::Commits,
                };
                ctx.app.restore_search_for_active_scope();
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::EnsureCommitsLoadedForActivePanel,
                ));
            }
            DomainAction::PanelGoto(n) => {
                ctx.app.active_panel = match n {
                    1 => SidePanel::Files,
                    2 => SidePanel::LocalBranches,
                    3 => SidePanel::Commits,
                    4 => SidePanel::Stash,
                    _ => ctx.app.active_panel,
                };
                ctx.app.restore_search_for_active_scope();
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::EnsureCommitsLoadedForActivePanel,
                ));
            }
            DomainAction::ListDown => {
                ctx.app.list_down();
                Self::recompute_commit_highlight(ctx);
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
            }
            DomainAction::ListUp => {
                ctx.app.list_up();
                Self::recompute_commit_highlight(ctx);
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
            }
            DomainAction::ToggleDir => {
                ctx.app.toggle_selected_dir();
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
            }
            DomainAction::CollapseAll => {
                ctx.app.collapse_all();
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
            }
            DomainAction::ExpandAll => {
                ctx.app.expand_all();
                ctx.app.schedule_diff_reload();
                ctx.app.dirty.mark_main_content();
            }
            DomainAction::DiffScrollUp => {
                ctx.app.diff_scroll_up();
                ctx.app.dirty.mark_diff();
            }
            DomainAction::DiffScrollDown => {
                ctx.app.diff_scroll_down();
                ctx.app.dirty.mark_diff();
            }
            _ => return ReduceOutput::none(),
        }
        ReduceOutput::none()
    }
}
