use ratagit_core::{AppState, PanelFocus};

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

pub(crate) fn shortcuts_for_state(state: &AppState) -> String {
    match shortcut_line_for_state(state) {
        ShortcutLine::Segments(segments) => segments
            .iter()
            .map(|segment| format!("{} {}", segment.key, segment.label))
            .collect::<Vec<_>>()
            .join("  "),
        ShortcutLine::Text(text) => text,
    }
}

pub(crate) fn shortcut_line_for_state(state: &AppState) -> ShortcutLine {
    if let Some(editor) = &state.editor.kind {
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

    if state.branches.create.active {
        return segments(&[
            ("arrows/Home/End", "cursor"),
            ("Enter", "create"),
            ("Esc", "cancel"),
        ]);
    }

    if state.branches.delete_menu.active {
        return segments(&[("j/k", "select"), ("Enter", "delete"), ("Esc", "cancel")]);
    }

    if state.branches.force_delete_confirm.active {
        return segments(&[("Enter", "force delete"), ("Esc", "cancel")]);
    }

    if state.branches.rebase_menu.active {
        return segments(&[("j/k", "select"), ("Enter", "rebase"), ("Esc", "cancel")]);
    }

    if state.branches.auto_stash_confirm.active {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.reset_menu.active {
        return segments(&[("j/k", "select"), ("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.discard_confirm.active {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state
        .active_search_scope()
        .is_some_and(|scope| state.search.is_input_active_for(scope))
    {
        return ShortcutLine::Text(format!("search: {}", state.search.query));
    }

    match state.focus {
        PanelFocus::Files => segments(&[
            ("space", "stage/unstage"),
            ("d", "discard"),
            ("c", "commit"),
            ("s", "stash"),
            ("D", "reset"),
            ("v", "multi"),
            ("enter", "expand"),
        ]),
        PanelFocus::Branches => segments(&[
            ("space", "checkout"),
            ("n", "new"),
            ("d", "delete"),
            ("r", "rebase"),
        ]),
        PanelFocus::Commits => {
            if state.commits.files.active {
                segments(&[("Esc", "back")])
            } else {
                segments(&[
                    ("enter", "files"),
                    ("s", "squash"),
                    ("f", "fixup"),
                    ("r", "reword"),
                    ("d", "delete"),
                    ("space", "detach"),
                    ("v", "multi"),
                ])
            }
        }
        PanelFocus::Stash => segments(&[("p", "stash push"), ("O", "stash pop")]),
        PanelFocus::Details | PanelFocus::Log => ShortcutLine::Text(String::new()),
    }
}

fn segments(values: &[(&'static str, &'static str)]) -> ShortcutLine {
    ShortcutLine::Segments(
        values
            .iter()
            .map(|(key, label)| ShortcutSegment { key, label })
            .collect(),
    )
}
