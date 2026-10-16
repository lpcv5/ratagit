use ratagit_core::{AppContext, PanelFocus};

use crate::panels::{
    PanelLine, panel_title, panel_title_label, render_branches_lines, render_commits_lines,
    render_details_lines, render_files_lines, render_log_lines, render_stash_lines,
};
use crate::theme::PanelLabel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelProjection {
    pub(crate) panel: PanelFocus,
    pub(crate) focused: bool,
    pub(crate) title: PanelLabel,
    pub(crate) legacy_text_title: &'static str,
    pub(crate) lines: Vec<PanelLine>,
}

pub(crate) fn project_panel(
    state: &AppContext,
    panel: PanelFocus,
    visible_lines: usize,
) -> PanelProjection {
    PanelProjection {
        panel,
        focused: state.ui.focus == panel,
        title: panel_title_label(state, panel),
        legacy_text_title: panel_title(state, panel),
        lines: panel_lines(state, panel, visible_lines),
    }
}

fn panel_lines(state: &AppContext, panel: PanelFocus, visible_lines: usize) -> Vec<PanelLine> {
    match panel {
        PanelFocus::Files => render_files_lines(state, visible_lines),
        PanelFocus::Branches => render_branches_lines(state, visible_lines),
        PanelFocus::Commits => render_commits_lines(state, visible_lines),
        PanelFocus::Stash => render_stash_lines(state, visible_lines),
        PanelFocus::Details => render_details_lines(state, visible_lines),
        PanelFocus::Log => render_log_lines(state, visible_lines),
    }
}

#[cfg(test)]
mod tests {
    use ratagit_core::{Action, GitResult, PanelFocus, update};
    use ratagit_testkit::fixture_dirty_repo;

    use super::*;

    #[test]
    fn panel_projection_carries_shared_terminal_metadata() {
        let mut state = AppContext::default();
        update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
        );
        state.ui.focus = PanelFocus::Branches;

        let projection = project_panel(&state, PanelFocus::Branches, 2);

        assert_eq!(projection.panel, PanelFocus::Branches);
        assert!(projection.focused);
        assert_eq!(projection.title.badge, "2");
        assert_eq!(projection.title.body, " Branches");
        assert_eq!(projection.legacy_text_title, "[2]  Branches");
        assert_eq!(projection.lines.len(), 2);
    }
}
