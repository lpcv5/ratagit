use ratagit_core::{AppState, BranchEntry, CommitEntry, FileEntry, PanelFocus, StashEntry};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFrame {
    pub lines: Vec<String>,
}

impl RenderedFrame {
    pub fn as_text(&self) -> String {
        self.lines.join("\n")
    }
}

pub fn render(state: &AppState, size: TerminalSize) -> RenderedFrame {
    let width = size.width.max(1);
    let height = size.height.max(1);
    let mut lines = Vec::with_capacity(height);

    let body_height = height.saturating_sub(1);
    if body_height > 0 {
        lines.extend(render_workspace_rows(state, width, body_height));
    }
    lines.push(pad_and_truncate(
        shortcuts_for_focus(state.focus).to_string(),
        width,
    ));

    normalize_lines(lines, TerminalSize { width, height })
}

pub fn render_terminal(frame: &mut Frame<'_>, state: &AppState) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(area);

    render_panel_grid(frame, state, root[0]);
    render_shortcuts(frame, state, root[1]);
}

fn render_panel_grid(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(area);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(24),
            Constraint::Percentage(28),
            Constraint::Percentage(20),
        ])
        .split(columns[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(columns[1]);

    render_block_panel(
        frame,
        state,
        PanelFocus::Files,
        left[0],
        render_files_lines(state, left[0].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Branches,
        left[1],
        render_branches_lines(state, left[1].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Commits,
        left[2],
        render_commits_lines(state, left[2].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Stash,
        left[3],
        render_stash_lines(state, left[3].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Details,
        right[0],
        render_details_lines(state, right[0].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Log,
        right[1],
        render_log_lines(state, right[1].height.saturating_sub(2) as usize),
    );
}

fn render_block_panel(
    frame: &mut Frame<'_>,
    state: &AppState,
    panel: PanelFocus,
    area: Rect,
    lines: Vec<String>,
) {
    let focused = state.focus == panel;
    let border_style = if focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title = if focused {
        format!(" {} * ", panel_title(panel))
    } else {
        format!(" {} ", panel_title(panel))
    };
    let text = lines.into_iter().map(Line::from).collect::<Vec<_>>();
    let widget = Paragraph::new(text).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(widget, area);
}

fn render_shortcuts(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let widget = Paragraph::new(shortcuts_for_focus(state.focus)).block(
        Block::default()
            .title(" Keys ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(widget, area);
}

fn render_workspace_rows(state: &AppState, total_width: usize, body_height: usize) -> Vec<String> {
    let separator = " | ";
    let separator_width = separator.len();
    if total_width <= separator_width {
        return vec![" ".repeat(total_width); body_height];
    }

    let (left_width, right_width) = split_columns(total_width - separator_width);
    let left_heights = split_vertical(body_height, &[28, 24, 28, 20]);
    let right_heights = split_vertical(body_height, &[70, 30]);

    let left_panels = [
        render_panel(
            panel_title(PanelFocus::Files),
            state.focus == PanelFocus::Files,
            left_width,
            left_heights[0],
            render_files_lines(state, left_heights[0].saturating_sub(1)),
        ),
        render_panel(
            panel_title(PanelFocus::Branches),
            state.focus == PanelFocus::Branches,
            left_width,
            left_heights[1],
            render_branches_lines(state, left_heights[1].saturating_sub(1)),
        ),
        render_panel(
            panel_title(PanelFocus::Commits),
            state.focus == PanelFocus::Commits,
            left_width,
            left_heights[2],
            render_commits_lines(state, left_heights[2].saturating_sub(1)),
        ),
        render_panel(
            panel_title(PanelFocus::Stash),
            state.focus == PanelFocus::Stash,
            left_width,
            left_heights[3],
            render_stash_lines(state, left_heights[3].saturating_sub(1)),
        ),
    ]
    .concat();

    let right_panels = [
        render_panel(
            panel_title(PanelFocus::Details),
            state.focus == PanelFocus::Details,
            right_width,
            right_heights[0],
            render_details_lines(state, right_heights[0].saturating_sub(1)),
        ),
        render_panel(
            panel_title(PanelFocus::Log),
            state.focus == PanelFocus::Log,
            right_width,
            right_heights[1],
            render_log_lines(state, right_heights[1].saturating_sub(1)),
        ),
    ]
    .concat();

    (0..body_height)
        .map(|index| {
            format!(
                "{}{}{}",
                left_panels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(left_width)),
                separator,
                right_panels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(right_width)),
            )
        })
        .collect()
}

fn split_columns(workspace_width: usize) -> (usize, usize) {
    let mut left = workspace_width * 34 / 100;
    if left < 20 {
        left = 20.min(workspace_width);
    }
    if workspace_width.saturating_sub(left) < 20 && workspace_width >= 40 {
        left = workspace_width - 20;
    }
    (left, workspace_width.saturating_sub(left))
}

fn split_vertical(total_height: usize, ratios: &[usize]) -> Vec<usize> {
    if ratios.is_empty() {
        return Vec::new();
    }
    let sum: usize = ratios.iter().sum();
    if sum == 0 {
        return vec![0; ratios.len()];
    }

    let mut values: Vec<usize> = ratios
        .iter()
        .map(|ratio| total_height * ratio / sum)
        .collect();
    let used: usize = values.iter().sum();
    let mut remainder = total_height.saturating_sub(used);
    let mut index = 0usize;
    while remainder > 0 {
        values[index] = values[index].saturating_add(1);
        remainder -= 1;
        index = (index + 1) % values.len();
    }
    values
}

fn render_panel(
    title: &str,
    focused: bool,
    width: usize,
    height: usize,
    content_lines: Vec<String>,
) -> Vec<String> {
    if height == 0 {
        return Vec::new();
    }

    let mut lines = Vec::with_capacity(height);
    let header = if focused {
        format!("> [{title}]")
    } else {
        format!("  [{title}]")
    };
    lines.push(pad_and_truncate(header, width));

    for line in content_lines.into_iter().take(height.saturating_sub(1)) {
        lines.push(pad_and_truncate(line, width));
    }
    while lines.len() < height {
        lines.push(" ".repeat(width));
    }
    lines
}

fn panel_title(panel: PanelFocus) -> &'static str {
    match panel {
        PanelFocus::Files => "Files",
        PanelFocus::Branches => "Branches",
        PanelFocus::Commits => "Commits",
        PanelFocus::Stash => "Stash",
        PanelFocus::Details => "Details",
        PanelFocus::Log => "Log",
    }
}

fn render_files_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    render_indexed_entries(
        &state.files.items,
        state.files.selected,
        max_lines,
        format_file_entry,
    )
}

fn render_branches_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    render_indexed_entries(
        &state.branches.items,
        state.branches.selected,
        max_lines,
        format_branch_entry,
    )
}

fn render_commits_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    let mut lines = render_indexed_entries(
        &state.commits.items,
        state.commits.selected,
        max_lines.saturating_sub(1),
        format_commit_entry,
    );
    if max_lines > 0 {
        lines.push(format!("  draft={}", state.commits.draft_message));
    }
    lines
}

fn render_stash_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    render_indexed_entries(
        &state.stash.items,
        state.stash.selected,
        max_lines,
        format_stash_entry,
    )
}

fn render_details_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("  current={:?}", state.last_left_focus));
    match state.last_left_focus {
        PanelFocus::Files => {
            if let Some(entry) = state.files.items.get(state.files.selected) {
                lines.push(format!("  file={}", entry.path));
                lines.push(format!(
                    "  staged={}",
                    if entry.staged { "yes" } else { "no" }
                ));
            } else {
                lines.push("  file=<empty>".to_string());
            }
        }
        PanelFocus::Branches => {
            if let Some(entry) = state.branches.items.get(state.branches.selected) {
                lines.push(format!("  branch={}", entry.name));
                lines.push(format!(
                    "  is_current={}",
                    if entry.is_current { "yes" } else { "no" }
                ));
            } else {
                lines.push("  branch=<empty>".to_string());
            }
        }
        PanelFocus::Commits => {
            if let Some(entry) = state.commits.items.get(state.commits.selected) {
                lines.push(format!("  commit={} {}", entry.id, entry.summary));
            } else {
                lines.push("  commit=<empty>".to_string());
            }
        }
        PanelFocus::Stash => {
            if let Some(entry) = state.stash.items.get(state.stash.selected) {
                lines.push(format!("  stash={} {}", entry.id, entry.summary));
            } else {
                lines.push("  stash=<empty>".to_string());
            }
        }
        PanelFocus::Details | PanelFocus::Log => {}
    }
    lines.push(format!("  summary={}", state.status.summary));
    lines.into_iter().take(max_lines).collect()
}

fn render_log_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(error) = &state.status.last_error {
        lines.push(format!("  error={error}"));
    } else {
        lines.push("  error=<none>".to_string());
    }

    let keep = max_lines.saturating_sub(lines.len());
    if keep > 0 {
        let start = state.notices.len().saturating_sub(keep);
        for notice in &state.notices[start..] {
            lines.push(format!("  notice={notice}"));
        }
    }
    lines.into_iter().take(max_lines).collect()
}

fn render_indexed_entries<T>(
    items: &[T],
    selected: usize,
    max_lines: usize,
    format_item: impl Fn(&T) -> String,
) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }
    if items.is_empty() {
        return vec!["  <empty>".to_string()];
    }
    items
        .iter()
        .enumerate()
        .take(max_lines)
        .map(|(index, item)| {
            if index == selected {
                format!("> {}", format_item(item))
            } else {
                format!("  {}", format_item(item))
            }
        })
        .collect()
}

fn shortcuts_for_focus(focus: PanelFocus) -> &'static str {
    match focus {
        PanelFocus::Files => "keys(files): s stage | u unstage",
        PanelFocus::Branches => "keys(branches): b create branch | o checkout",
        PanelFocus::Commits => "keys(commits): c commit",
        PanelFocus::Stash => "keys(stash): p stash push | O stash pop",
        PanelFocus::Details | PanelFocus::Log => "",
    }
}

fn pad_and_truncate(text: String, width: usize) -> String {
    let truncated = if text.len() > width {
        text.chars().take(width).collect::<String>()
    } else {
        text
    };
    format!("{truncated:width$}", width = width)
}

fn normalize_lines(mut lines: Vec<String>, size: TerminalSize) -> RenderedFrame {
    for line in &mut lines {
        *line = pad_and_truncate(line.clone(), size.width);
    }

    if lines.len() > size.height {
        lines.truncate(size.height);
    } else {
        while lines.len() < size.height {
            lines.push(" ".repeat(size.width));
        }
    }

    RenderedFrame { lines }
}

pub fn format_file_entry(entry: &FileEntry) -> String {
    format!(
        "{} {}",
        if entry.staged { "[S]" } else { "[ ]" },
        entry.path
    )
}

pub fn format_commit_entry(entry: &CommitEntry) -> String {
    format!("{} {}", entry.id, entry.summary)
}

pub fn format_branch_entry(entry: &BranchEntry) -> String {
    format!(
        "{} {}",
        if entry.is_current { "*" } else { " " },
        entry.name
    )
}

pub fn format_stash_entry(entry: &StashEntry) -> String {
    format!("{} {}", entry.id, entry.summary)
}
