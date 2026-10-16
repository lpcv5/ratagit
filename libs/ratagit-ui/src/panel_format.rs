use ratagit_core::{
    BranchEntry, CommitEntry, CommitHashStatus, FileRowKind, FileTreeRow, StashEntry,
};
use ratatui::style::{Modifier, Style};
use unicode_width::UnicodeWidthChar;

use super::panel_types::{PanelSpan, plain_text};
use crate::theme::{
    ICON_BATCH_SELECTED, ICON_BRANCH, ICON_DIRECTORY_CLOSED, ICON_DIRECTORY_OPEN,
    ICON_SEARCH_MATCH, ICON_STASH, MAIN_UI_THEME, RowRole, UiTheme,
};

pub fn format_file_tree_row(row: &FileTreeRow) -> String {
    plain_text(&file_tree_row_spans(row, &MAIN_UI_THEME))
}

pub(crate) fn file_tree_row_spans(row: &FileTreeRow, theme: &UiTheme) -> Vec<PanelSpan> {
    let indent = "  ".repeat(row.depth);
    let batch = if row.selected_for_batch {
        ICON_BATCH_SELECTED
    } else {
        " "
    };
    let matched = if row.matched { ICON_SEARCH_MATCH } else { " " };
    let prefix = format!("{batch}{matched} {indent}");
    let (marker, suffix) = match row.kind {
        FileRowKind::Directory => {
            let marker = if row.expanded {
                ICON_DIRECTORY_OPEN
            } else {
                ICON_DIRECTORY_CLOSED
            };
            (marker.to_string(), format!(" {}/", row.name))
        }
        FileRowKind::File => (file_tree_status_marker(row), format!(" {}", row.name)),
    };
    let mut spans = vec![
        PanelSpan {
            text: prefix,
            style: Style::default(),
        },
        PanelSpan {
            text: marker.trim_end_matches('U').to_string(),
            style: file_tree_marker_style(row, theme),
        },
    ];
    if row.kind == FileRowKind::File && row.conflicted {
        spans.push(PanelSpan {
            text: "U".to_string(),
            style: Style::default().fg(theme.files.conflict),
        });
    }
    spans.push(PanelSpan {
        text: suffix,
        style: file_tree_name_style(row, theme),
    });
    spans
}

pub fn format_commit_entry(entry: &CommitEntry) -> String {
    plain_text(&commit_entry_spans(entry, &MAIN_UI_THEME))
}

pub(crate) fn commit_entry_spans(entry: &CommitEntry, theme: &UiTheme) -> Vec<PanelSpan> {
    let graph = fixed_width(commit_graph(entry), 1);
    let hash = fixed_width(&entry.id, 7);
    let author = fixed_width(&author_initials(&entry.author_name), 2);
    vec![
        PanelSpan {
            text: graph,
            style: Style::default().fg(theme.commit.graph),
        },
        PanelSpan {
            text: "  ".to_string(),
            style: Style::default(),
        },
        PanelSpan {
            text: hash,
            style: commit_hash_style(entry.hash_status, theme),
        },
        PanelSpan {
            text: "  ".to_string(),
            style: Style::default(),
        },
        PanelSpan {
            text: author,
            style: author_style(&entry.author_name, theme),
        },
        PanelSpan {
            text: "  ".to_string(),
            style: Style::default(),
        },
        PanelSpan {
            text: commit_message_summary(entry),
            style: Style::default(),
        },
    ]
}

pub fn format_branch_entry(entry: &BranchEntry) -> String {
    if entry.is_current {
        format!("{ICON_BRANCH} {}", entry.name)
    } else {
        format!("  {}", entry.name)
    }
}

pub fn format_stash_entry(entry: &StashEntry) -> String {
    format!("{ICON_STASH} {} {}", entry.id, entry.summary)
}

pub(crate) fn file_tree_row_role(row: &FileTreeRow) -> RowRole {
    if row.selected_for_batch {
        RowRole::BatchSelected
    } else {
        RowRole::Normal
    }
}

pub(crate) fn branch_entry_role(entry: &BranchEntry) -> RowRole {
    if entry.is_current {
        RowRole::CurrentBranch
    } else {
        RowRole::Normal
    }
}

fn commit_file_status_marker(status: ratagit_core::CommitFileStatus) -> &'static str {
    match status {
        ratagit_core::CommitFileStatus::Added => "A",
        ratagit_core::CommitFileStatus::Modified => "M",
        ratagit_core::CommitFileStatus::Deleted => "D",
        ratagit_core::CommitFileStatus::Renamed => "R",
        ratagit_core::CommitFileStatus::Copied => "C",
        ratagit_core::CommitFileStatus::TypeChanged => "T",
        ratagit_core::CommitFileStatus::Unknown => "?",
    }
}

fn file_tree_marker_style(row: &FileTreeRow, theme: &UiTheme) -> Style {
    if let Some(status) = row.commit_status {
        commit_file_status_style(status, theme)
    } else if row.untracked {
        Style::default().fg(theme.files.unknown)
    } else if row.staged {
        Style::default().fg(theme.files.staged)
    } else {
        Style::default()
    }
}

fn file_tree_name_style(row: &FileTreeRow, theme: &UiTheme) -> Style {
    if row.kind == FileRowKind::File && row.staged {
        Style::default().fg(theme.files.staged)
    } else {
        Style::default()
    }
}

fn file_tree_status_marker(row: &FileTreeRow) -> String {
    let status = row.commit_status.unwrap_or(if row.untracked {
        ratagit_core::CommitFileStatus::Unknown
    } else {
        ratagit_core::CommitFileStatus::Modified
    });
    let mut marker = commit_file_status_marker(status).to_string();
    if row.conflicted {
        marker.push('U');
    }
    marker
}

fn fixed_width(text: &str, width: usize) -> String {
    let mut output = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > width {
            break;
        }
        output.push(ch);
        used += ch_width;
    }
    format!("{}{}", output, " ".repeat(width.saturating_sub(used)))
}

fn commit_graph(entry: &CommitEntry) -> &str {
    if entry.graph.is_empty() {
        "●"
    } else {
        &entry.graph
    }
}

fn commit_message_summary(entry: &CommitEntry) -> String {
    if entry.summary.is_empty() {
        entry
            .message
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        entry.summary.clone()
    }
}

fn commit_hash_style(status: CommitHashStatus, theme: &UiTheme) -> Style {
    let color = match status {
        CommitHashStatus::MergedToMain => theme.commit.hash_merged,
        CommitHashStatus::Pushed => theme.commit.hash_pushed,
        CommitHashStatus::Unpushed => theme.commit.hash_unpushed,
    };
    Style::default().fg(color)
}

fn author_style(author_name: &str, theme: &UiTheme) -> Style {
    let hash = author_name.bytes().fold(0usize, |accumulator, byte| {
        accumulator.wrapping_mul(31).wrapping_add(byte as usize)
    });
    Style::default()
        .fg(theme.commit.authors[hash % theme.commit.authors.len()])
        .add_modifier(Modifier::BOLD)
}

fn author_initials(author_name: &str) -> String {
    let words = author_name
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let mut chars = if words.len() >= 2 {
        words
            .iter()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<Vec<_>>()
    } else {
        author_name
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .take(2)
            .collect::<Vec<_>>()
    };
    while chars.len() < 2 {
        chars.push('?');
    }
    chars
        .into_iter()
        .flat_map(char::to_uppercase)
        .take(2)
        .collect()
}

fn commit_file_status_style(status: ratagit_core::CommitFileStatus, theme: &UiTheme) -> Style {
    use ratagit_core::CommitFileStatus;
    let color = match status {
        CommitFileStatus::Added => theme.files.added,
        CommitFileStatus::Modified => theme.files.modified,
        CommitFileStatus::Deleted => theme.files.deleted,
        CommitFileStatus::Renamed => theme.files.renamed,
        CommitFileStatus::Copied => theme.files.copied,
        CommitFileStatus::TypeChanged => theme.files.type_changed,
        CommitFileStatus::Unknown => theme.files.unknown,
    };
    let style = Style::default().fg(color);
    if matches!(
        status,
        CommitFileStatus::Modified | CommitFileStatus::TypeChanged
    ) {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}
