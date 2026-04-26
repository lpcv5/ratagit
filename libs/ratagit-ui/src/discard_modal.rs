use ratagit_core::AppState;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::theme::focused_panel_style;

pub(crate) fn render_discard_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    if !state.discard_confirm.active {
        return;
    }

    let modal = centered_rect(area, 72, 11);
    if modal.width < 20 || modal.height < 8 {
        return;
    }

    let block = Block::default()
        .title(" Discard Changes ")
        .borders(Borders::ALL)
        .border_style(focused_panel_style());
    let inner = block.inner(modal);
    let content = inset_rect(inner, 1, 0);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(content);

    frame.render_widget(Clear, modal);
    frame.render_widget(block, modal);
    frame.render_widget(Paragraph::new("Discard selected file changes?"), rows[0]);
    frame.render_widget(
        Paragraph::new(format!(
            "Targets: {}",
            format_target_count(&state.discard_confirm.paths)
        )),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(format_target_paths(&state.discard_confirm.paths))
            .wrap(Wrap { trim: false }),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new("This removes tracked changes and deletes untracked targets.\nEnter discard  |  Esc cancel"),
        rows[3],
    );
}

fn format_target_count(paths: &[String]) -> String {
    match paths {
        [] => "0 files".to_string(),
        [_] => "1 file".to_string(),
        _ => format!("{} files", paths.len()),
    }
}

fn format_target_paths(paths: &[String]) -> String {
    if paths.is_empty() {
        return "No targets selected.".to_string();
    }

    let mut lines = paths
        .iter()
        .take(4)
        .map(|path| format!("- {path}"))
        .collect::<Vec<_>>();
    if paths.len() > lines.len() {
        lines.push(format!("... and {} more", paths.len() - lines.len()));
    }
    lines.join("\n")
}

fn inset_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    let shrink_x = horizontal.saturating_mul(2).min(area.width);
    let shrink_y = vertical.saturating_mul(2).min(area.height);
    Rect::new(
        area.x.saturating_add(horizontal.min(area.width)),
        area.y.saturating_add(vertical.min(area.height)),
        area.width.saturating_sub(shrink_x),
        area.height.saturating_sub(shrink_y),
    )
}

fn centered_rect(area: Rect, target_width: u16, target_height: u16) -> Rect {
    let max_width = area.width.saturating_sub(2).max(1);
    let max_height = area.height.saturating_sub(2).max(1);
    let width = target_width.min(max_width).max(20.min(area.width));
    let height = target_height.min(max_height).max(8.min(area.height));
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}
