use ratagit_core::{AppContext, BranchesSubview, PanelFocus};

use crate::frame::RenderContext;
use crate::loading_indicator::loading_indicator_for_state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShortcutSegment {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShortcutLine {
    Segments(Vec<ShortcutSegment>),
    Text(String),
}

#[cfg(test)]
pub(crate) fn shortcuts_for_state(state: &AppContext) -> String {
    shortcuts_for_state_with_context(state, RenderContext::default())
}

pub(crate) fn shortcuts_for_state_with_context(
    state: &AppContext,
    context: RenderContext,
) -> String {
    let shortcuts = shortcut_text_for_state(state);
    if let Some(indicator) = loading_indicator_for_state(state, context) {
        if shortcuts.is_empty() {
            indicator.text()
        } else {
            format!("{}  {}", indicator.text(), shortcuts)
        }
    } else {
        shortcuts
    }
}

fn shortcut_text_for_state(state: &AppContext) -> String {
    match shortcut_line_for_state(state) {
        ShortcutLine::Segments(segments) => segments
            .iter()
            .map(|segment| format!("{} {}", segment.key, segment.label))
            .collect::<Vec<_>>()
            .join("  "),
        ShortcutLine::Text(text) => text,
    }
}

pub(crate) fn shortcut_line_for_state(state: &AppContext) -> ShortcutLine {
    if let Some(editor) = &state.ui.editor.kind {
        return match editor {
            ratagit_core::EditorKind::Commit { .. } => segments(&[
                ("Tab", "field"),
                ("arrows/Home/End", "cursor"),
                ("Ctrl+J", "newline"),
                ("Enter", "confirm"),
                ("Esc", "cancel"),
            ]),
            ratagit_core::EditorKind::Stash { .. } => segments(&[
                ("arrows/Home/End", "cursor"),
                ("Enter", "confirm"),
                ("Esc", "cancel"),
            ]),
        };
    }

    if state.ui.branches.create.active {
        return segments(&[
            ("arrows/Home/End", "cursor"),
            ("Enter", "create"),
            ("Esc", "cancel"),
        ]);
    }

    if state.ui.branches.delete_menu.active {
        return segments(&[("j/k", "select"), ("Enter", "delete"), ("Esc", "cancel")]);
    }

    if state.ui.branches.delete_confirm.active {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.ui.branches.force_delete_confirm.active {
        return segments(&[("Enter", "force delete"), ("Esc", "cancel")]);
    }

    if state.ui.branches.rebase_menu.active {
        return segments(&[("j/k", "select"), ("Enter", "rebase"), ("Esc", "cancel")]);
    }

    if state.ui.branches.auto_stash_confirm.active {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.ui.reset_menu.active {
        return segments(&[("j/k", "select"), ("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.ui.reset_menu.danger_confirm.is_some() {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.ui.discard_confirm.active {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.ui.push_force_confirm.active {
        return segments(&[("Enter", "force push"), ("Esc", "cancel")]);
    }

    if state.ui.stage_all_confirm.active {
        return segments(&[("Enter", "stage all"), ("Esc", "cancel")]);
    }

    if state
        .active_search_scope()
        .is_some_and(|scope| state.ui.search.is_input_active_for(scope))
    {
        return ShortcutLine::Text(format!("search: {}", state.ui.search.query));
    }

    match state.ui.focus {
        PanelFocus::Files => sync_segments(&[
            ("space", "stage/unstage"),
            ("d", "discard"),
            ("A", "amend"),
            ("c", "commit"),
            ("s", "stash"),
            ("D", "reset"),
            ("enter", "expand"),
        ]),
        PanelFocus::Branches => match state.ui.branches.subview {
            BranchesSubview::List => sync_segments(&[
                ("enter", "commits"),
                ("space", "checkout"),
                ("n", "new"),
                ("d", "delete"),
                ("r", "rebase"),
            ]),
            BranchesSubview::Commits => sync_segments(&[("enter", "files"), ("Esc", "back")]),
            BranchesSubview::CommitFiles => sync_segments(&[("enter", "expand"), ("Esc", "back")]),
        },
        PanelFocus::Commits => {
            if state.ui.commits.files.active {
                sync_segments(&[("Esc", "back")])
            } else {
                sync_segments(&[
                    ("enter", "files"),
                    ("A", "amend"),
                    ("s", "squash"),
                    ("f", "fixup"),
                    ("r", "reword"),
                    ("d", "delete"),
                    ("space", "detach"),
                ])
            }
        }
        PanelFocus::Stash => sync_segments(&[("O", "stash pop")]),
        PanelFocus::Details | PanelFocus::Log => sync_segments(&[]),
    }
}

fn sync_segments(values: &[(&'static str, &'static str)]) -> ShortcutLine {
    let mut combined = vec![("p", "pull"), ("P", "push")];
    combined.extend_from_slice(values);
    segments(&combined)
}

fn segments(values: &[(&'static str, &'static str)]) -> ShortcutLine {
    ShortcutLine::Segments(
        values
            .iter()
            .map(|(key, label)| ShortcutSegment { key, label })
            .collect(),
    )
}
