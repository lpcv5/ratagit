use ratagit_core::AppContext;

use crate::frame::RenderContext;

const SPINNER_FRAMES: [&str; 4] = ["/", "|", "\\", "-"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadingIndicator {
    pub(crate) spinner: &'static str,
    pub(crate) kind: String,
    pub(crate) spotlight_index: usize,
}

impl LoadingIndicator {
    pub(crate) fn text(&self) -> String {
        format!("{} loading: {}", self.spinner, self.kind)
    }
}

pub(crate) fn loading_indicator_for_state(
    state: &AppContext,
    context: RenderContext,
) -> Option<LoadingIndicator> {
    loading_kind_for_state(state).map(|kind| {
        let spinner_frame = context.spinner_frame;
        let text_width = "loading: ".chars().count() + kind.chars().count();
        LoadingIndicator {
            spinner: spinner_for_frame(spinner_frame),
            kind,
            spotlight_index: spotlight_index_for_frame(spinner_frame, text_width),
        }
    })
}

fn spinner_for_frame(frame: usize) -> &'static str {
    SPINNER_FRAMES[frame % SPINNER_FRAMES.len()]
}

fn spotlight_index_for_frame(frame: usize, text_width: usize) -> usize {
    if text_width == 0 {
        0
    } else {
        frame % text_width
    }
}

fn loading_kind_for_state(state: &AppContext) -> Option<String> {
    if let Some(operation) = &state.work.operation_pending {
        return Some(operation.clone());
    }
    if state.work.refresh_pending {
        return Some("refresh".to_string());
    }
    if state.work.details_pending {
        return Some("details".to_string());
    }
    if state.work.commit_files_loading {
        return Some("commit files".to_string());
    }
    if state.work.commits_loading_more {
        return Some("commits".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use ratagit_core::{AppContext, RefreshTarget};

    use super::*;

    #[test]
    fn loading_indicator_uses_spinner_frame() {
        let mut state = AppContext::default();
        state.work.refresh_pending = true;

        let first = loading_indicator_for_state(&state, RenderContext { spinner_frame: 0 })
            .expect("refresh should show loading");
        let next = loading_indicator_for_state(&state, RenderContext { spinner_frame: 1 })
            .expect("refresh should show loading");

        assert_eq!(first.text(), "/ loading: refresh");
        assert_eq!(next.text(), "| loading: refresh");
        assert_eq!(first.spotlight_index, 0);
        assert_eq!(next.spotlight_index, 1);
    }

    #[test]
    fn loading_kind_prioritizes_mutations_over_reads() {
        let mut state = AppContext::default();
        state.work.operation_pending = Some("push".to_string());
        state.work.refresh_pending = true;
        state.work.pending_refreshes.insert(RefreshTarget::Files);
        state.work.details_pending = true;

        let indicator = loading_indicator_for_state(&state, RenderContext::default())
            .expect("operation should show loading");

        assert_eq!(indicator.text(), "/ loading: push");
    }

    #[test]
    fn loading_indicator_is_hidden_without_pending_work() {
        assert!(
            loading_indicator_for_state(&AppContext::default(), RenderContext::default()).is_none()
        );
    }
}
