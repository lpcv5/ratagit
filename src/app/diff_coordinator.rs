//! Application wiring for diff decisions, task scheduling and refreshes.
use super::diff::{
    DiffAction, DiffLoad, DiffRefreshSource, DiffReload, DiffTarget, DiffUpdate, HunkView,
    HunkWrite,
};
use super::{AppState, BackgroundLoad, RefreshKind, SidePanel};
use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;
use crate::flux::task_manager::{TaskKey, TaskPriority, TaskRequestKind};
use std::time::{Duration, Instant};
use tracing::debug;

impl AppState {
    pub fn hunk_view(&self) -> HunkView<'_> {
        self.diff
            .view()
            .hunks(self.ui.active_panel == SidePanel::Files)
    }

    pub(super) fn selected_diff_target(&self) -> DiffTarget {
        self.selection()
            .diff_target()
            .with_side(self.diff.interaction().side)
    }

    pub(super) fn should_schedule_diff_for_refresh(&self, source: DiffRefreshSource) -> bool {
        self.selected_diff_target().affected_by(source)
    }

    pub(crate) fn apply_diff_action(&mut self, action: &DomainAction) {
        let action = match action {
            DomainAction::ToggleDiffFocus => DiffAction::ToggleFocus,
            DomainAction::SwitchDiffSide => DiffAction::SwitchSide,
            DomainAction::HunkNext => DiffAction::NextHunk,
            DomainAction::HunkPrevious => DiffAction::PreviousHunk,
            DomainAction::ToggleHunkSelection => DiffAction::ToggleSelection,
            DomainAction::ClearHunkSelection => DiffAction::ClearSelection,
            DomainAction::ApplySelectedHunks(direction) => DiffAction::ApplySelected(*direction),
            DomainAction::ApplyHunksFinished {
                id,
                direction,
                result,
            } => DiffAction::WriteFinished {
                id: *id,
                direction: *direction,
                result,
            },
            DomainAction::PanelNext
            | DomainAction::PanelPrev
            | DomainAction::PanelGoto(_)
            | DomainAction::ListUp
            | DomainAction::ListDown => DiffAction::SelectionChanged,
            DomainAction::StageHunks { patch, file, hunks } => DiffAction::StageHunks {
                patch,
                file: *file,
                hunks,
            },
            DomainAction::StageHunksFinished { result } => DiffAction::StageHunksFinished(result),
            _ => return,
        };
        let output = self
            .diff
            .update(action, self.ui.active_panel == SidePanel::Files);
        self.apply_diff_update(output);
    }

    fn apply_diff_update(&mut self, output: DiffUpdate) {
        if let Some(write) = output.write {
            self.effects.push(match write {
                HunkWrite::Selected {
                    id,
                    patch,
                    ids,
                    direction,
                } => EffectRequest::ApplyHunks {
                    id,
                    patch,
                    ids,
                    direction,
                },
                HunkWrite::Legacy { patch, file, hunks } => {
                    EffectRequest::StageHunks { patch, file, hunks }
                }
            });
        }
        if let Some((message, success)) = output.log {
            self.push_log(message, success);
        }
        if output.refresh_status {
            self.request_refresh(RefreshKind::StatusOnly);
        }
        match output.reload {
            DiffReload::None => {}
            DiffReload::Deferred => self.schedule_diff_reload(),
            DiffReload::Immediate => self.reload_diff_now(),
        }
        if output.redraw {
            self.ui.dirty.mark_all();
        }
    }

    pub(super) fn diff_task_key() -> TaskKey {
        TaskKey::Diff {
            target: "active".to_string(),
        }
    }

    pub(super) fn cancel_pending_diff_task(&mut self) {
        self.cancel_background_task(&Self::diff_task_key());
        self.diff.cancel_load();
    }

    pub fn schedule_diff_reload(&mut self) {
        self.diff.schedule_reload(Instant::now());
        self.cancel_pending_diff_task();
        debug!(event = "schedule_diff_reload", "scheduled diff reload");
    }

    pub fn has_pending_diff_reload(&self) -> bool {
        self.diff.has_pending_reload()
    }

    pub fn diff_reload_debounce_elapsed(&self, debounce: Duration) -> bool {
        self.diff.debounce_elapsed(debounce, Instant::now())
    }

    pub fn pending_diff_reload_at(&self) -> Option<Instant> {
        self.diff.pending_reload_at()
    }

    pub fn flush_pending_diff_reload(&mut self) {
        self.start_pending_diff_load();
    }

    pub(super) fn start_pending_diff_load(&mut self) {
        if !self.diff.has_pending_reload() {
            return;
        }
        let target = self.selected_diff_target();
        let decision = self
            .diff
            .prepare_reload(target, &self.files.view().tree_nodes);
        if matches!(decision, DiffLoad::Unchanged) {
            return;
        }
        // The feature has already selected its next load identity. Cancel only
        // the old runtime task here, without clearing that new feature identity.
        self.cancel_background_task(&Self::diff_task_key());
        if let DiffLoad::Request { cache_key, target } = decision {
            let request = self.tasks.task_manager.enqueue(
                Self::diff_task_key(),
                TaskPriority::High,
                TaskRequestKind::LoadDiff {
                    target: cache_key.task_target(),
                },
            );
            self.queue_background(request, BackgroundLoad::Diff { cache_key, target });
        }
        self.ui.dirty.mark_diff();
    }

    pub fn reload_diff_now(&mut self) {
        self.schedule_diff_reload();
        self.start_pending_diff_load();
    }

    pub fn diff_scroll_up(&mut self) {
        self.diff.update(DiffAction::ScrollUp, false);
    }

    pub fn diff_scroll_down(&mut self) {
        self.diff.update(DiffAction::ScrollDown, false);
    }
}
