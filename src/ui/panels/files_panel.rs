use crate::app::App;
use crate::ui::widgets::file_tree::{FileTree, FileTreeState};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render_files_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let border_style = if is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title_style = if is_active {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Files")
        .border_style(border_style)
        .title_style(title_style);

    if app.file_tree_nodes.is_empty() {
        let items = vec![ListItem::new("No changes").style(Style::default().fg(Color::DarkGray))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let widget = FileTree::new(app.file_tree_nodes.clone()).block(block);

    let mut state = FileTreeState {
        list_state: app.files_panel.list_state.clone(),
        expanded_dirs: std::collections::HashSet::new(),
    };

    frame.render_stateful_widget(widget, area, &mut state);
}
