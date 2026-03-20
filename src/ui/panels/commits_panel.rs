use crate::app::App;
use crate::git::CommitSyncState;
use crate::ui::panels::revision_tree_panel::render_revision_tree_panel;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::ListItem,
    Frame,
};

pub fn render_commits_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();

    let items: Vec<ListItem> = if app.commits.is_empty() {
        vec![ListItem::new("No commits").style(Style::default().fg(theme.text_muted))]
    } else {
        app.commits
            .iter()
            .map(|c| {
                let graph = if c.parent_count > 1 { "⑂ " } else { "● " };
                let text = format!("{}{} {} {} {}", graph, c.short_hash, c.time, c.message, c.author);
                let color = match c.sync_state {
                    CommitSyncState::Main => Color::Cyan,
                    CommitSyncState::RemoteSynced => Color::Green,
                    CommitSyncState::LocalOnly => Color::Yellow,
                };
                ListItem::new(text).style(Style::default().fg(color))
            })
            .collect()
    };
    let title = if app.commit_tree_mode {
        if let Some(ref oid) = app.commit_tree_commit_oid {
            format!("Commit Files {} [Esc Back]", &oid[..oid.len().min(7)])
        } else {
            "Commit Files [Esc Back]".to_string()
        }
    } else {
        "Commits [main:cyan synced:green local:yellow]".to_string()
    };
    render_revision_tree_panel(
        frame,
        area,
        &title,
        is_active,
        app.commit_tree_mode,
        &app.commit_tree_nodes,
        items,
        app.commits_panel.list_state,
    );
}
