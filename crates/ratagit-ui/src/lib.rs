use ratagit_core::{AppState, BranchEntry, CommitEntry, FileEntry, PanelFocus, StashEntry};

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
    let mut lines = Vec::new();
    lines.push(format!(
        "ratagit MVP | branch={} | focus={:?}",
        state.status.current_branch, state.focus
    ));
    lines.push(format!(
        "summary={}{}",
        state.status.summary,
        state
            .status
            .last_error
            .as_ref()
            .map(|error| format!(" | error={error}"))
            .unwrap_or_default()
    ));
    lines.push(String::new());

    lines.extend(render_status_panel(state));
    lines.extend(render_files_panel(state));
    lines.extend(render_commits_panel(state));
    lines.extend(render_branches_panel(state));
    lines.extend(render_stash_panel(state));

    if let Some(last_notice) = state.notices.last() {
        lines.push(String::new());
        lines.push(format!("notice={last_notice}"));
    }

    normalize_lines(lines, size)
}

fn render_status_panel(state: &AppState) -> Vec<String> {
    vec![
        panel_header(state.focus, PanelFocus::Status, "Status"),
        format!("  detached_head={}", state.status.detached_head),
        format!("  refresh_count={}", state.status.refresh_count),
    ]
}

fn render_files_panel(state: &AppState) -> Vec<String> {
    let mut lines = vec![panel_header(state.focus, PanelFocus::Files, "Files")];
    lines.extend(render_indexed_entries(
        &state.files.items,
        state.files.selected,
        |entry| {
            format!(
                "{} {}",
                if entry.staged { "[S]" } else { "[ ]" },
                entry.path
            )
        },
    ));
    lines
}

fn render_commits_panel(state: &AppState) -> Vec<String> {
    let mut lines = vec![panel_header(state.focus, PanelFocus::Commits, "Commits")];
    lines.extend(render_indexed_entries(
        &state.commits.items,
        state.commits.selected,
        |entry| format!("{} {}", entry.id, entry.summary),
    ));
    lines.push(format!("  draft={}", state.commits.draft_message));
    lines
}

fn render_branches_panel(state: &AppState) -> Vec<String> {
    let mut lines = vec![panel_header(state.focus, PanelFocus::Branches, "Branches")];
    lines.extend(render_indexed_entries(
        &state.branches.items,
        state.branches.selected,
        |entry| {
            format!(
                "{} {}",
                if entry.is_current { "*" } else { " " },
                entry.name
            )
        },
    ));
    lines
}

fn render_stash_panel(state: &AppState) -> Vec<String> {
    let mut lines = vec![panel_header(state.focus, PanelFocus::Stash, "Stash")];
    lines.extend(render_indexed_entries(
        &state.stash.items,
        state.stash.selected,
        |entry| format!("{} {}", entry.id, entry.summary),
    ));
    lines
}

fn render_indexed_entries<T>(
    items: &[T],
    selected: usize,
    format_item: impl Fn(&T) -> String,
) -> Vec<String> {
    if items.is_empty() {
        return vec!["  <empty>".to_string()];
    }
    items
        .iter()
        .enumerate()
        .take(3)
        .map(|(index, item)| {
            if index == selected {
                format!("> {}", format_item(item))
            } else {
                format!("  {}", format_item(item))
            }
        })
        .collect()
}

fn panel_header(focus: PanelFocus, panel: PanelFocus, title: &str) -> String {
    if focus == panel {
        format!("> [{title}]")
    } else {
        format!("  [{title}]")
    }
}

fn normalize_lines(mut lines: Vec<String>, size: TerminalSize) -> RenderedFrame {
    for line in &mut lines {
        let truncated = if line.len() > size.width {
            line.chars().take(size.width).collect::<String>()
        } else {
            line.clone()
        };
        *line = format!("{truncated:width$}", width = size.width);
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
