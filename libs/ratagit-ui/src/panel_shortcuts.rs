use ratagit_core::{ActiveLeftView, AppContext};

use crate::frame::RenderContext;
use crate::loading_indicator::loading_indicator_for_state;
use ratatui::{style::Style, text::Line};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShortcutSegment {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShortcutLine {
    Segments(Vec<ShortcutSegment>),
    Text(String),
    Search {
        text: String,
        no_matches: bool,
        hints: Vec<ShortcutSegment>,
    },
}

impl ShortcutSegment {
    pub(crate) fn width(&self) -> usize {
        self.key.width() + self.label.width() + 3
    }

    fn essential(&self) -> bool {
        self.key == "?" || self.key.contains("Esc")
    }
}

impl ShortcutLine {
    // Compatibility text projections retain their existing unbounded contents.
    // Only the terminal projection selects whole, badge-sized segments.
    pub(crate) fn for_width(self, state: &AppContext, width: usize) -> Self {
        let line = terminal_shortcut_line(state, self);
        let search = &state.ui.search;
        let editing = matches!(line, Self::Text(_));
        let confirmed = !state.ui.command_palette.active
            && !search.active
            && !search.query.is_empty()
            && state
                .active_search_scope()
                .is_some_and(|scope| search.scope == Some(scope))
            && matches!(&line, Self::Segments(values) if values.iter().any(|s| s.key == "?"));
        if editing || confirmed {
            let count = search.matches.len();
            let status = match count {
                0 if search.query.is_empty() => "type to search".to_string(),
                0 => "no matches".to_string(),
                1 => "1 match".to_string(),
                _ => format!("{count} matches"),
            };
            let mut hints = if editing {
                vec![
                    ShortcutSegment {
                        key: "Enter",
                        label: "confirm",
                    },
                    ShortcutSegment {
                        key: "Esc",
                        label: "cancel",
                    },
                ]
            } else {
                vec![
                    ShortcutSegment {
                        key: "n/N",
                        label: "next/prev",
                    },
                    ShortcutSegment {
                        key: "Esc",
                        label: "clear",
                    },
                    ShortcutSegment {
                        key: "?",
                        label: "commands",
                    },
                ]
            };
            // Reserve a useful query preview and the full result count first.
            let minimum_text =
                "search: ".width() + search.query.width().min(8) + 2 + status.width();
            hints = fit_segments(&hints, width.saturating_sub(minimum_text + 2));
            let hints_width = segments_width(&hints);
            let text_width = width.saturating_sub(hints_width + usize::from(!hints.is_empty()) * 2);
            let query_width = text_width.saturating_sub("search: ".width() + status.width() + 2);
            let query = ellipsize(&search.query, query_width);
            let text = format!("search: {query}  {status}");
            return Self::Search {
                text: ellipsize(&text, text_width),
                no_matches: count == 0 && !search.query.is_empty(),
                hints,
            };
        }
        match line {
            Self::Segments(values) => Self::Segments(fit_segments(&values, width)),
            other => other,
        }
    }

    pub(crate) fn minimum_width(state: &AppContext) -> usize {
        match terminal_shortcut_line(state, shortcut_line_for_state(state)) {
            Self::Segments(values) => segments_width(
                &values
                    .into_iter()
                    .filter(ShortcutSegment::essential)
                    .collect::<Vec<_>>(),
            ),
            _ => 0,
        }
    }
}

fn terminal_shortcut_line(state: &AppContext, mut line: ShortcutLine) -> ShortcutLine {
    if let ShortcutLine::Segments(values) = &mut line
        && state.ui.focus == ratagit_core::PanelFocus::Details
        && state.ui.details.patch.active
        && state.ui.details.patch.discard.is_none()
        && state.ui.details.repository_confirm.is_none()
        && !state.ui.command_palette.active
        && state.ui.workflow.dialog.is_none()
        && state.ui.workflow.remote_request.is_none()
    {
        if !values.iter().any(|s| s.key.contains("Esc")) {
            values.push(ShortcutSegment {
                key: "Esc",
                label: "clear/back",
            });
        }
        if !values.iter().any(|s| s.key == "?") {
            values.push(ShortcutSegment {
                key: "?",
                label: "commands",
            });
        }
    }
    line
}

fn segments_width(segments: &[ShortcutSegment]) -> usize {
    segments.iter().map(ShortcutSegment::width).sum::<usize>()
        + segments.len().saturating_sub(1) * 2
}

fn fit_segments(segments: &[ShortcutSegment], width: usize) -> Vec<ShortcutSegment> {
    let mut included = vec![false; segments.len()];
    let mut used = 0;
    // Escape and commands survive before optional actions. At very small widths
    // the first essential segment wins; no key or label is ever sliced.
    for essential in [true, false] {
        for (index, segment) in segments.iter().enumerate() {
            if segment.essential() != essential {
                continue;
            }
            let next = segment.width() + usize::from(used > 0) * 2;
            if used + next <= width {
                included[index] = true;
                used += next;
            }
        }
    }
    segments
        .iter()
        .zip(included)
        .filter_map(|(s, keep)| keep.then_some(*s))
        .collect()
}

fn ellipsize(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    let line = Line::from(text);
    for grapheme in line.styled_graphemes(Style::default()) {
        let next = grapheme.symbol.width();
        if used + next > width - 1 {
            break;
        }
        result.push_str(grapheme.symbol);
        used += next;
    }
    result.push('…');
    result
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
        ShortcutLine::Text(text) | ShortcutLine::Search { text, .. } => text,
    }
}

pub(crate) fn shortcut_line_for_state(state: &AppContext) -> ShortcutLine {
    if matches!(
        state.ui.workflow.dialog,
        Some(ratagit_core::workflow::Dialog::Menu { .. })
    ) {
        return segments(&[
            ("j/k ↑/↓", "select"),
            ("Enter", "confirm"),
            ("Esc", "cancel"),
        ]);
    }
    if state.ui.workflow.dialog.is_some() || state.ui.workflow.remote_request.is_some() {
        return segments(&[
            ("arrows", "select/cursor"),
            ("Enter", "confirm"),
            ("Esc", "cancel"),
        ]);
    }
    if state.ui.details.repository_confirm.is_some() {
        return segments(&[("Enter", "drop stash"), ("Esc", "cancel")]);
    }
    if state.ui.custom_patch.preview_return_focus.is_some() && !state.ui.command_palette.active {
        return segments(&[
            ("j/k", "scroll"),
            ("Ctrl+P", "patch options"),
            ("Tab/Esc", "source"),
            ("?", "commands"),
        ]);
    }
    if state.ui.details.patch.active
        && state.ui.focus == ratagit_core::PanelFocus::Details
        && state.ui.last_left_focus == ratagit_core::PanelFocus::Stash
    {
        return segments(&[("Ctrl+U/D", "scroll"), ("Esc", "back")]);
    }
    if state.ui.details.patch.discard.is_some() {
        return segments(&[("Enter", "discard"), ("Esc", "cancel")]);
    }
    if state.ui.details.patch.active
        && state.ui.focus == ratagit_core::PanelFocus::Details
        && !state.ui.command_palette.active
    {
        if ratagit_core::workflow::commit_files::is_patch(state) {
            return local_segments(&[
                ("space", "patch"),
                ("a", "mode"),
                ("d", "remove"),
                ("n/N", "file"),
                ("Ctrl+P", "menu"),
            ]);
        }
        return segments(&[
            ("j/k", "select"),
            ("a", "mode"),
            ("v", "range"),
            ("x", "mark"),
            ("Tab", "side"),
            ("Space", "apply"),
            (
                "d",
                if state.ui.details.patch.staged {
                    "unstage"
                } else {
                    "discard"
                },
            ),
            ("n/N", "file"),
            ("Esc", "clear/back"),
        ]);
    }
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

    if state.ui.branches.delete_menu.menu.active {
        return segments(&[("j/k", "select"), ("Enter", "delete"), ("Esc", "cancel")]);
    }

    if state.ui.branches.delete_confirm.active {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.ui.branches.force_delete_confirm.active {
        return segments(&[("Enter", "force delete"), ("Esc", "cancel")]);
    }

    if state.ui.branches.rebase_menu.menu.active {
        return segments(&[("j/k", "select"), ("Enter", "rebase"), ("Esc", "cancel")]);
    }

    if state.ui.branches.auto_stash_confirm.active {
        return segments(&[("Enter", "confirm"), ("Esc", "cancel")]);
    }

    if state.ui.reset_menu.menu.active {
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

    match state.active_left_view() {
        Some(ActiveLeftView::RepositoryTab(_)) => local_segments(&[
            ("←/→ [/]", "tabs"),
            ("j/k", "select"),
            ("/", "search"),
            ("r", "refresh"),
        ]),
        Some(ActiveLeftView::Files) => local_segments(&[
            ("space", "stage/unstage"),
            ("d", "discard"),
            ("A", "amend"),
            ("c", "commit"),
            ("s", "stash"),
            ("D", "reset"),
            ("enter", "expand"),
        ]),
        Some(ActiveLeftView::BranchesList) => local_segments(&[
            ("enter", "commits"),
            ("space", "checkout"),
            ("n", "new"),
            ("d", "delete"),
            ("r", "rebase"),
        ]),
        Some(ActiveLeftView::BranchCommits) => {
            local_segments(&[("enter", "files"), ("Esc", "back")])
        }
        Some(ActiveLeftView::BranchCommitFiles) => commit_file_segments(),
        Some(ActiveLeftView::Commits) => local_segments(&[
            ("enter", "files"),
            ("A", "amend"),
            ("s", "squash"),
            ("f", "fixup"),
            ("r", "reword"),
            ("d", "delete"),
            ("space", "detach"),
        ]),
        Some(ActiveLeftView::CommitFiles) => commit_file_segments(),
        Some(ActiveLeftView::Stash) => local_segments(&[("O", "stash pop")]),
        None => local_segments(&[]),
    }
}

fn commit_file_segments() -> ShortcutLine {
    local_segments(&[
        ("enter", "hunk"),
        ("space", "patch"),
        ("a", "all"),
        ("Ctrl+P", "menu"),
        ("Tab", "preview"),
    ])
}

fn local_segments(values: &[(&'static str, &'static str)]) -> ShortcutLine {
    let mut combined = values.to_vec();
    combined.push(("?", "commands"));
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

#[cfg(test)]
mod responsive_tests {
    use super::*;

    #[test]
    fn fit_preserves_whole_segments_and_reserves_essential_actions() {
        let values = [
            ShortcutSegment {
                key: "长键",
                label: "选择",
            },
            ShortcutSegment {
                key: "Esc",
                label: "back",
            },
            ShortcutSegment {
                key: "?",
                label: "commands",
            },
        ];
        for width in 0..100 {
            let fitted = fit_segments(&values, width);
            assert!(segments_width(&fitted) <= width);
            if width >= segments_width(&values[1..]) {
                assert!(fitted.contains(&values[1]));
                assert!(fitted.contains(&values[2]));
            }
        }
        assert_eq!(values[0].width(), 11);
        assert_eq!(fit_segments(&values[..1], 10), vec![]);
        assert_eq!(fit_segments(&values[..1], 11), values[..1]);
    }

    #[test]
    fn search_unicode_preview_preserves_count_without_exceeding_width() {
        let mut state = AppContext::default();
        state.ui.search.active = true;
        state.ui.search.scope = state.active_search_scope();
        state.ui.search.query = "非常长的中文查询e\u{301}文字".to_string();
        for width in 25..101 {
            let ShortcutLine::Search {
                text,
                hints,
                no_matches,
            } = shortcut_line_for_state(&state).for_width(&state, width)
            else {
                panic!("search feedback expected");
            };
            assert!(no_matches);
            assert!(text.contains("no matches"), "{width}: {text}");
            assert!(
                text.width() + segments_width(&hints) + usize::from(!hints.is_empty()) * 2 <= width
            );
        }
    }

    #[test]
    fn legacy_shortcuts_retain_unbounded_projection() {
        let state = AppContext::default();
        assert!(shortcuts_for_state(&state).contains("enter expand  ? commands"));
        let ShortcutLine::Segments(values) = shortcut_line_for_state(&state).for_width(&state, 35)
        else {
            panic!("shortcuts expected");
        };
        assert!(values.iter().any(|segment| segment.key == "?"));
        assert!(segments_width(&values) <= 35);
    }
}
