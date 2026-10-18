use ratagit_core::{AppContext, PanelFocus};

use crate::panels::{
    PanelLine, panel_title, panel_title_label, render_branches_lines_with_theme,
    render_commits_lines_with_theme, render_details_lines_with_theme,
    render_files_lines_with_theme, render_log_lines, render_stash_lines_with_theme,
};
use crate::theme::{PanelLabel, UiTheme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelProjection {
    pub(crate) panel: PanelFocus,
    pub(crate) focused: bool,
    pub(crate) title: PanelLabel,
    pub(crate) legacy_text_title: &'static str,
    pub(crate) caption: Option<String>,
    pub(crate) tabs: Vec<(&'static str, bool)>,
    pub(crate) lines: Vec<PanelLine>,
}

#[cfg(test)]
pub(crate) fn project_panel(
    state: &AppContext,
    panel: PanelFocus,
    visible_lines: usize,
) -> PanelProjection {
    project_panel_with_theme(state, panel, visible_lines, &crate::theme::MAIN_UI_THEME)
}

pub(crate) fn project_panel_with_theme(
    state: &AppContext,
    panel: PanelFocus,
    visible_lines: usize,
    theme: &UiTheme,
) -> PanelProjection {
    PanelProjection {
        panel,
        focused: state.ui.focus == panel,
        title: panel_title_label(state, panel),
        legacy_text_title: panel_title(state, panel),
        caption: patch_caption(state, panel),
        tabs: crate::panels::panel_left::tab_labels(state, panel),
        lines: panel_lines(state, panel, visible_lines, theme),
    }
}

fn patch_caption(state: &AppContext, panel: PanelFocus) -> Option<String> {
    if state.ui.tabs.active(panel).is_some() {
        return Some("read-only · first 1000 entries".into());
    }
    let patch = &state.ui.custom_patch;
    let files = format!(
        "{} {}",
        patch.included.len(),
        if patch.included.len() == 1 {
            "file"
        } else {
            "files"
        }
    );
    if panel == PanelFocus::Details && patch.preview_return_focus.is_some() {
        let limited = String::from_utf8_lossy(&state.repo.custom_patch.preview)
            .split_inclusive('\n')
            .nth(ratagit_core::custom_patch::PREVIEW_LINES)
            .is_some();
        let source: String = patch
            .source
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(7)
            .collect();
        let limit = if limited { " · first 5000 lines" } else { "" };
        return Some(format!("{files}{limit} · {source} · read-only"));
    }
    if panel == PanelFocus::Details && ratagit_core::workflow::commit_files::is_patch(state) {
        let mode = match state.ui.details.patch.mode {
            ratagit_core::partial::SelectionMode::Hunk => "hunk",
            ratagit_core::partial::SelectionMode::Line => "line",
            ratagit_core::partial::SelectionMode::Block => "block",
        };
        let file = state.repo.details.patch.as_ref()?.files.first()?;
        let path = ratagit_core::custom_patch::file_key(file).ok()?;
        return Some(format!("{mode} · {path}"));
    }
    let source = match panel {
        PanelFocus::Commits if state.ui.commits.files.active => {
            state.ui.commits.files.commit_id.as_deref()
        }
        PanelFocus::Branches
            if state.ui.branches.subview == ratagit_core::BranchesSubview::CommitFiles =>
        {
            state.ui.branches.commit_files.commit_id.as_deref()
        }
        _ => None,
    };
    (source.is_some() && source == patch.source.as_deref() && !patch.included.is_empty())
        .then(|| format!("patch: {files}"))
}

fn panel_lines(
    state: &AppContext,
    panel: PanelFocus,
    visible_lines: usize,
    theme: &UiTheme,
) -> Vec<PanelLine> {
    if let Some(tab) = state.ui.tabs.active(panel) {
        return crate::panels::panel_left::render_tab_lines(state, tab, visible_lines, theme);
    }
    match panel {
        PanelFocus::Files => render_files_lines_with_theme(state, visible_lines, theme),
        PanelFocus::Branches => render_branches_lines_with_theme(state, visible_lines, theme),
        PanelFocus::Commits => render_commits_lines_with_theme(state, visible_lines, theme),
        PanelFocus::Stash => render_stash_lines_with_theme(state, visible_lines, theme),
        PanelFocus::Details => render_details_lines_with_theme(state, visible_lines, theme),
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
