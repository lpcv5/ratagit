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
    let lines = state.details.files_diff.lines().collect::<Vec<_>>();
    let start = details_scroll_start(lines.len(), state.details.scroll_offset, max_lines);
    lines
        .into_iter()
        .skip(start)
        .map(|line| PanelLine::new(format!("  {line}"), classify_diff_row_role(line)))
        .take(max_lines)
        .collect()
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

    let lines = state.details.branch_log.lines().collect::<Vec<_>>();
    let start = details_scroll_start(lines.len(), state.details.scroll_offset, max_lines);
    lines
        .into_iter()
        .skip(start)
        .map(|line| ansi_branch_log_line(line, "  "))
        .take(max_lines)
        .collect()
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

    let lines = state.details.commit_diff.lines().collect::<Vec<_>>();
    let start = details_scroll_start(lines.len(), state.details.scroll_offset, max_lines);
    lines
        .into_iter()
        .skip(start)
        .map(|line| PanelLine::new(format!("  {line}"), classify_diff_row_role(line)))
        .take(max_lines)
        .collect()
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

    let lines = state.details.commit_file_diff.lines().collect::<Vec<_>>();
    let start = details_scroll_start(lines.len(), state.details.scroll_offset, max_lines);
    lines
        .into_iter()
        .skip(start)
        .map(|line| PanelLine::new(format!("  {line}"), classify_diff_row_role(line)))
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

fn classify_diff_row_role(line: &str) -> RowRole {
    if line.starts_with("### ") {
        return RowRole::DiffSection;
    }
    if line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        return RowRole::DiffMeta;
    }
    if line.starts_with("@@") {
        return RowRole::DiffHunk;
    }
    if line.starts_with('+') && !line.starts_with("+++") {
        return RowRole::DiffAdd;
    }
    if line.starts_with('-') && !line.starts_with("---") {
        return RowRole::DiffRemove;
    }
    RowRole::Normal
}

fn ansi_branch_log_line(line: &str, prefix: &str) -> PanelLine {
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
