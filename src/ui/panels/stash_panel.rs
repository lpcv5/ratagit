use crate::app::App;
use crate::ui::theme::UiTheme;
use crate::ui::widgets::file_tree::{FileTree, FileTreeState};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{List, ListItem},
    Frame,
};

pub fn render_stash_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();
    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };

    if app.stash_tree_mode {
        let title = if let Some(index) = app.stash_tree_stash_index {
            format!("Stash Files stash@{{{}}} [Esc Back]", index)
        } else {
            "Stash Files [Esc Back]".to_string()
        };

        if app.stash_tree_nodes.is_empty() {
            let items = vec![ListItem::new("No files").style(Style::default().fg(theme.text_muted))];
            let list = List::new(items)
                .block(theme.panel_block(&title, is_active))
                .highlight_style(highlight);
            let mut state = app.stash_panel.list_state;
            frame.render_stateful_widget(list, area, &mut state);
            return;
        }

        let widget = FileTree::new(app.stash_tree_nodes.clone())
            .block(theme.panel_block(&title, is_active))
            .highlight_style(highlight);
        let mut state = FileTreeState {
            list_state: app.stash_panel.list_state,
            expanded_dirs: std::collections::HashSet::new(),
        };
        frame.render_stateful_widget(widget, area, &mut state);
        return;
    }

    let items: Vec<ListItem> = if app.stashes.is_empty() {
        vec![ListItem::new("No stashes").style(Style::default().fg(theme.text_muted))]
    } else {
        app.stashes
            .iter()
            .map(|s| {
                let text = format!("stash@{{{}}} {}", s.index, s.message);
                ListItem::new(text).style(Style::default().fg(Color::Yellow))
            })
            .collect()
    };

    let list = List::new(items)
        .block(theme.panel_block("Stash", is_active))
        .highlight_style(highlight);

    let mut state = app.stash_panel.list_state;
    frame.render_stateful_widget(list, area, &mut state);
}
