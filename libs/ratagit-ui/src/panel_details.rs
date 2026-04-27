use ratagit_core::{AppState, PanelFocus};
use ratatui::style::{Color, Modifier, Style};

use super::panel_types::{PanelLine, PanelSpan};
use crate::theme::RowRole;

pub(crate) fn render_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    match state.last_left_focus {
        PanelFocus::Files => render_files_details_lines(state, max_lines),
        PanelFocus::Branches => render_branch_details_lines(state, max_lines),
        PanelFocus::Commits => render_commit_details_lines(state, max_lines),
        // TODO(details-stash): replace placeholder with stash entry details projection.
        PanelFocus::Stash => render_placeholder_details_lines(
            "  details(stash): pending details implementation",
            max_lines,
        ),
        PanelFocus::Details | PanelFocus::Log => Vec::new(),
    }
}

pub(crate) fn render_log_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let mut lines = Vec::new();
    if state.work.refresh_pending {
        lines.push(PanelLine::new(
            "  work=refreshing repository",
            RowRole::Notice,
        ));
    }
    if state.status.status_scan_skipped {
        lines.push(PanelLine::new(
            "  status=huge repo metadata-only; file scan skipped",
            RowRole::Notice,
        ));
        lines.push(PanelLine::new(
            "  tip=focus Commits/Branches or narrow Git outside ratagit",
            RowRole::Notice,
        ));
    } else if state.status.large_repo_mode {
        lines.push(PanelLine::new(
            "  status=large repo fast mode; untracked scan skipped",
            RowRole::Notice,
        ));
        lines.push(PanelLine::new(
            "  tip=consider git untrackedCache/fsmonitor/splitIndex",
            RowRole::Notice,
        ));
    }
    if state.status.status_truncated {
        lines.push(PanelLine::new(
            "  status=truncated at 50000 file entries or 64 MiB output",
            RowRole::Notice,
        ));
    }
    if let Some(total) = state.details.files_diff_truncated_from {
        lines.push(PanelLine::new(
            format!("  details=diff limited to first 100 of {total} files"),
            RowRole::Notice,
        ));
    }
    if let Some(operation) = &state.work.operation_pending {
        lines.push(PanelLine::new(
            format!("  work=running {operation}"),
            RowRole::Notice,
        ));
    }
    if let Some(error) = &state.status.last_error {
        lines.push(PanelLine::new(format!("  error={error}"), RowRole::Error));
    }

    let keep = max_lines.saturating_sub(lines.len());
    if keep > 0 {
        let start = state.notices.len().saturating_sub(keep);
        for notice in &state.notices[start..] {
            lines.push(PanelLine::new(
                format!("  notice={notice}"),
                RowRole::Notice,
            ));
        }
    }
    lines.into_iter().take(max_lines).collect()
}

fn render_files_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }

    if let Some(error) = &state.details.files_error {
        return vec![PanelLine::new(format!("  error={error}"), RowRole::Error)];
    }

    if state.work.details_pending && state.details.files_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(files): loading diff",
            RowRole::Muted,
        )];
    }

    if state.details.files_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(files): no diff for current selection",
            RowRole::Muted,
        )];
    }

    // TODO(files-hunks): upgrade details rows into selectable hunk models for partial staging.
    let mut rendered = Vec::new();
    if let Some(total) = state.details.files_diff_truncated_from {
        rendered.push(PanelLine::new(
            format!("  details(files): showing first 100 of {total} files"),
            RowRole::Notice,
        ));
    }
    rendered.extend(render_ansi_details_text(
        &state.details.files_diff,
        state.details.scroll_offset,
        max_lines.saturating_sub(rendered.len()),
    ));
    rendered
}

fn render_branch_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }

    if state.details.branch_log_target.is_none() {
        return vec![PanelLine::new(
            "  details(branches): no branch selected",
            RowRole::Muted,
        )];
    }

    if let Some(error) = &state.details.branch_log_error {
        return vec![PanelLine::new(format!("  error={error}"), RowRole::Error)];
    }

    if state.work.details_pending && state.details.branch_log.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(branches): loading log graph",
            RowRole::Muted,
        )];
    }

    if state.details.branch_log.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(branches): no log graph for current selection",
            RowRole::Muted,
        )];
    }

    render_ansi_details_text(
        &state.details.branch_log,
        state.details.scroll_offset,
        max_lines,
    )
}

fn render_commit_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if state.commits.files.active {
        return render_commit_file_details_lines(state, max_lines);
    }
    if max_lines == 0 {
        return Vec::new();
    }

    if state.details.commit_diff_target.is_none() {
        return vec![PanelLine::new(
            "  details(commits): no commit selected",
            RowRole::Muted,
        )];
    }

    if let Some(error) = &state.details.commit_diff_error {
        return vec![PanelLine::new(format!("  error={error}"), RowRole::Error)];
    }

    if state.work.details_pending && state.details.commit_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(commits): loading diff",
            RowRole::Muted,
        )];
    }

    if state.details.commit_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(commits): no diff for current selection",
            RowRole::Muted,
        )];
    }

    render_ansi_details_text(
        &state.details.commit_diff,
        state.details.scroll_offset,
        max_lines,
    )
}

fn render_commit_file_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }

    if state.commits.files.loading {
        return vec![PanelLine::new(
            "  details(commit files): loading files",
            RowRole::Muted,
        )];
    }

    if state.details.commit_file_diff_target.is_none() {
        return vec![PanelLine::new(
            "  details(commit files): no file selected",
            RowRole::Muted,
        )];
    }

    if let Some(error) = &state.details.commit_file_diff_error {
        return vec![PanelLine::new(format!("  error={error}"), RowRole::Error)];
    }

    if state.work.details_pending && state.details.commit_file_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(commit files): loading diff",
            RowRole::Muted,
        )];
    }

    if state.details.commit_file_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(commit files): no diff for current file",
            RowRole::Muted,
        )];
    }

    render_ansi_details_text(
        &state.details.commit_file_diff,
        state.details.scroll_offset,
        max_lines,
    )
}

fn render_ansi_details_text(text: &str, scroll_offset: usize, max_lines: usize) -> Vec<PanelLine> {
    let start = if scroll_offset == 0 {
        0
    } else {
        let line_count = text.lines().count();
        details_scroll_start(line_count, scroll_offset, max_lines)
    };
    text.lines()
        .skip(start)
        .map(|line| ansi_output_line(line, "  "))
        .take(max_lines)
        .collect()
}

fn details_scroll_start(content_len: usize, requested_offset: usize, max_lines: usize) -> usize {
    requested_offset.min(content_len.saturating_sub(max_lines))
}

fn render_placeholder_details_lines(message: &str, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }
    vec![PanelLine::new(message, RowRole::Muted)]
}

fn ansi_output_line(line: &str, prefix: &str) -> PanelLine {
    let mut text = prefix.to_string();
    let mut spans = vec![PanelSpan {
        text: prefix.to_string(),
        style: Style::default(),
    }];
    let mut style = Style::default();
    let mut plain = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            let mut code = String::new();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
                code.push(next);
            }
            if !plain.is_empty() {
                text.push_str(&plain);
                spans.push(PanelSpan {
                    text: std::mem::take(&mut plain),
                    style,
                });
            }
            style = apply_sgr_codes(style, &code);
        } else {
            plain.push(ch);
        }
    }

    if !plain.is_empty() {
        text.push_str(&plain);
        spans.push(PanelSpan { text: plain, style });
    }

    PanelLine::new(text, RowRole::Normal).styled_spans(spans)
}

fn apply_sgr_codes(mut style: Style, code: &str) -> Style {
    let parts = if code.is_empty() {
        vec![0]
    } else {
        code.split(';')
            .filter_map(|part| part.parse::<u8>().ok())
            .collect::<Vec<_>>()
    };
    let mut index = 0;
    while index < parts.len() {
        match parts[index] {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            22 => style = style.remove_modifier(Modifier::BOLD),
            30..=37 => style = style.fg(ansi_color(parts[index] - 30, false)),
            39 => style = style.fg(Color::Reset),
            90..=97 => style = style.fg(ansi_color(parts[index] - 90, true)),
            38 if parts.get(index + 1) == Some(&5) => {
                if let Some(color) = parts.get(index + 2) {
                    style = style.fg(Color::Indexed(*color));
                    index += 2;
                }
            }
            38 if parts.get(index + 1) == Some(&2) => {
                if let (Some(red), Some(green), Some(blue)) = (
                    parts.get(index + 2),
                    parts.get(index + 3),
                    parts.get(index + 4),
                ) {
                    style = style.fg(Color::Rgb(*red, *green, *blue));
                    index += 4;
                }
            }
            _ => {}
        }
        index += 1;
    }
    style
}

fn ansi_color(index: u8, bright: bool) -> Color {
    match (index, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        (7, true) => Color::White,
        _ => Color::Reset,
    }
}

#[cfg(test)]
mod tests {
    use ratagit_core::{CommitFileDiffPath, CommitFileDiffTarget};

    use super::*;

    #[test]
    fn ansi_sgr_parser_handles_reset_bright_indexed_and_rgb_colors() {
        let line = ansi_output_line(
            "\u{1b}[1;33m*\u{1b}[0m plain \u{1b}[91mred\u{1b}[38;5;42midx\u{1b}[38;2;1;2;3mrgb",
            "  ",
        );
        let spans = line.spans.expect("ansi line should keep styled spans");

        assert_eq!(spans[1].text, "*");
        assert_eq!(
            spans[1].style,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(spans[2].text, " plain ");
        assert_eq!(spans[2].style, Style::default());
        assert_eq!(spans[3].text, "red");
        assert_eq!(spans[3].style, Style::default().fg(Color::LightRed));
        assert_eq!(spans[4].text, "idx");
        assert_eq!(spans[4].style, Style::default().fg(Color::Indexed(42)));
        assert_eq!(spans[5].text, "rgb");
        assert_eq!(spans[5].style, Style::default().fg(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn details_diff_output_preserves_ansi_spans_without_semantic_roles() {
        let mut state = AppState {
            last_left_focus: PanelFocus::Commits,
            ..AppState::default()
        };
        state.details.commit_diff_target = Some("abc1234".to_string());
        state.details.commit_diff = concat!(
            "\u{1b}[1mdiff --git a/a.txt b/a.txt\u{1b}[m\n",
            "\u{1b}[31m-old\u{1b}[m\n",
            "\u{1b}[32m+new\u{1b}[m"
        )
        .to_string();

        let lines = render_details_lines(&state, 3);

        assert_eq!(lines[0].text, "  diff --git a/a.txt b/a.txt");
        assert_eq!(lines[0].role, RowRole::Normal);
        assert_eq!(lines[1].text, "  -old");
        assert_eq!(lines[1].role, RowRole::Normal);
        assert_eq!(lines[2].text, "  +new");
        assert_eq!(lines[2].role, RowRole::Normal);
        let add_spans = lines[2]
            .spans
            .as_ref()
            .expect("ansi details output should keep styled spans");
        assert_eq!(add_spans[1].text, "+new");
        assert_eq!(add_spans[1].style, Style::default().fg(Color::Green));
    }

    #[test]
    fn details_lines_report_empty_loading_error_and_clamped_scroll_states() {
        let mut state = AppState {
            last_left_focus: PanelFocus::Files,
            ..AppState::default()
        };
        assert_eq!(
            render_details_lines(&state, 3),
            vec![PanelLine::new(
                "  details(files): no diff for current selection",
                RowRole::Muted
            )]
        );

        state.work.details_pending = true;
        assert_eq!(
            render_details_lines(&state, 3),
            vec![PanelLine::new(
                "  details(files): loading diff",
                RowRole::Muted
            )]
        );

        state.work.details_pending = false;
        state.details.files_error = Some("boom".to_string());
        assert_eq!(
            render_details_lines(&state, 3),
            vec![PanelLine::new("  error=boom", RowRole::Error)]
        );

        state.details.files_error = None;
        state.details.files_diff = "line 1\nline 2\nline 3\nline 4".to_string();
        state.details.scroll_offset = 99;
        assert_eq!(
            render_details_lines(&state, 2)
                .into_iter()
                .map(|line| line.text)
                .collect::<Vec<_>>(),
            vec!["  line 3".to_string(), "  line 4".to_string()]
        );
    }

    #[test]
    fn branch_commit_and_commit_file_placeholders_use_current_state() {
        let mut state = AppState {
            last_left_focus: PanelFocus::Branches,
            ..AppState::default()
        };
        assert_eq!(
            render_details_lines(&state, 1),
            vec![PanelLine::new(
                "  details(branches): no branch selected",
                RowRole::Muted
            )]
        );

        state.details.branch_log_target = Some("main".to_string());
        state.work.details_pending = true;
        assert_eq!(
            render_details_lines(&state, 1),
            vec![PanelLine::new(
                "  details(branches): loading log graph",
                RowRole::Muted
            )]
        );

        state.work.details_pending = false;
        state.last_left_focus = PanelFocus::Commits;
        assert_eq!(
            render_details_lines(&state, 1),
            vec![PanelLine::new(
                "  details(commits): no commit selected",
                RowRole::Muted
            )]
        );

        state.commits.files.active = true;
        state.commits.files.loading = true;
        assert_eq!(
            render_details_lines(&state, 1),
            vec![PanelLine::new(
                "  details(commit files): loading files",
                RowRole::Muted
            )]
        );

        state.commits.files.loading = false;
        state.details.commit_file_diff_target = Some(CommitFileDiffTarget {
            commit_id: "abc".to_string(),
            paths: vec![CommitFileDiffPath {
                path: "a.txt".to_string(),
                old_path: None,
            }],
        });
        assert_eq!(
            render_details_lines(&state, 1),
            vec![PanelLine::new(
                "  details(commit files): no diff for current file",
                RowRole::Muted
            )]
        );
    }
}
