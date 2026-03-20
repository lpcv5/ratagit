use crate::app::{App, SidePanel};
use crate::git::CommitSyncState;
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::ListItem,
    Frame,
};

pub fn render_commits_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();

    let items: Vec<ListItem> = if app.commits.is_empty() {
        vec![ListItem::new("No commits").style(Style::default().fg(theme.text_muted))]
    } else {
        let query = app.search_query_for_scope(SidePanel::Commits, false, false);
        app.commits
            .iter()
            .map(|c| {
                let color = match c.sync_state {
                    CommitSyncState::DefaultBranch => Color::Green,
                    CommitSyncState::RemoteBranch => Color::Yellow,
                    CommitSyncState::LocalOnly => Color::Red,
                };
                let hash_spans = highlighted_spans(&c.short_hash, query, Style::default().fg(color));
                let message_spans =
                    highlighted_spans(&c.message, query, Style::default().fg(Color::White));
                let mut spans = Vec::with_capacity(hash_spans.len() + 1 + message_spans.len());
                spans.extend(hash_spans);
                spans.push(ratatui::text::Span::raw(" "));
                spans.extend(message_spans);
                ListItem::new(Line::from(spans))
            })
            .collect()
    };
    let mut title = if app.commit_tree_mode {
        if let Some(ref oid) = app.commit_tree_commit_oid {
            format!("Commit Files {} [Esc Back]", &oid[..oid.len().min(7)])
        } else {
            "Commit Files [Esc Back]".to_string()
        }
    } else {
        "Commits [default:green remote:yellow local:red]".to_string()
    };
    if let Some(search) =
        app.search_match_summary_for(SidePanel::Commits, app.commit_tree_mode, false)
    {
        title = format!("{} [{}]", title, search);
    }
    render_revision_tree_panel(
        frame,
        area,
        RevisionTreePanelProps {
            title: &title,
            is_active,
            tree_mode: app.commit_tree_mode,
            tree_nodes: &app.commit_tree_nodes,
            tree_search_query: app.search_query_for_scope(SidePanel::Commits, true, false),
            list_items: items,
            list_state: app.commits_panel.list_state,
        },
    );
}
